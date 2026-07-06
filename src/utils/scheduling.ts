// スケジューリング静的解析（Phase 5.4b）。
// モデルと仮定は doc/scheduling_design.md §2 を参照。
// - デッドライン = 周期
// - single executor 内のコールバックは非プリエンプティブに直列実行
// - 安全側の上界で判定する

export type SchedulingProcess = {
  name: string;
  executor: "single" | "multi";
  threads?: number;
  priority?: number;
  cpu_affinity?: number[];
  nodes: string[];
};

export type AnalysisNode = {
  id: string;
  label: string;
  language: string;
  periodMs: number;
  offsetMs: number;
  wcetMs?: number;
};

export type AnalysisEdge = { source: string; target: string };

export type Finding = {
  severity: "error" | "warn" | "info" | "hint";
  message: string;
};

export type ProcessMetric = {
  name: string;
  /** 利用率（0〜）。メンバーの WCET が揃っていなければ null */
  utilization: number | null;
  capacity: number;
  ok: boolean | null;
  nodeLabels: string[];
};

export type NodeMetric = {
  id: string;
  label: string;
  periodMs: number;
  /** 応答時間上界 [ms]。WCET が不足していれば null */
  responseMs: number | null;
  /** R ≤ T（デッドライン検証）。判定不能なら null */
  ok: boolean | null;
  processName: string;
};

export type ScheduleAnalysis = {
  findings: Finding[];
  processes: ProcessMetric[];
  nodes: NodeMetric[];
};

const DEFAULT_MULTI_THREADS = 2;

export function analyzeScheduling(
  nodes: AnalysisNode[],
  edges: AnalysisEdge[],
  processes: SchedulingProcess[],
): ScheduleAnalysis {
  const findings: Finding[] = [];
  const byId = new Map(nodes.map((n) => [n.id, n]));

  // --- G: 配置妥当性 ---
  const assigned = new Map<string, string>(); // node id -> process name
  for (const proc of processes) {
    for (const id of proc.nodes) {
      const node = byId.get(id);
      if (!node) {
        findings.push({
          severity: "error",
          message: `プロセス「${proc.name}」のノード ${id} が存在しません`,
        });
        continue;
      }
      const prev = assigned.get(id);
      if (prev) {
        findings.push({
          severity: "error",
          message: `ノード「${node.label}」が複数のプロセス（${prev}, ${proc.name}）に所属しています`,
        });
        continue;
      }
      assigned.set(id, proc.name);
    }
    const langs = new Set(
      proc.nodes
        .map((id) => byId.get(id)?.language)
        .filter((l): l is string => l !== undefined),
    );
    if (langs.size > 1) {
      findings.push({
        severity: "error",
        message: `プロセス「${proc.name}」に異なる言語のノードが混在しています（${[...langs].join(", ")}）。同一プロセスに配置できるのは同一言語のみです`,
      });
    }
  }

  // --- 実効グループ（未所属ノードは単独プロセス）---
  type Group = {
    name: string;
    executor: "single" | "multi";
    members: AnalysisNode[];
  };
  const groups: Group[] = processes.map((p) => ({
    name: p.name,
    executor: p.executor,
    members: p.nodes
      .map((id) => byId.get(id))
      .filter((n): n is AnalysisNode => !!n),
  }));
  for (const node of nodes) {
    if (!assigned.has(node.id)) {
      groups.push({
        name: `(${node.label} 単独)`,
        executor: "single",
        members: [node],
      });
    }
  }
  const groupOf = new Map<string, Group>();
  for (const g of groups) {
    for (const m of g.members) groupOf.set(m.id, g);
  }

  // --- B: 応答時間上界とデッドライン検証 ---
  const responseOf = new Map<string, number>();
  const nodeMetrics: NodeMetric[] = nodes.map((node) => {
    const group = groupOf.get(node.id)!;
    let responseMs: number | null = null;
    if (node.wcetMs !== undefined) {
      if (group.executor === "multi") {
        // マルチスレッドは並列実行を仮定（楽観側。詳細解析は将来課題）
        responseMs = node.wcetMs;
      } else if (group.members.every((m) => m.wcetMs !== undefined)) {
        responseMs = group.members.reduce((sum, m) => sum + (m.wcetMs ?? 0), 0);
      }
    }
    if (responseMs !== null) responseOf.set(node.id, responseMs);
    return {
      id: node.id,
      label: node.label,
      periodMs: node.periodMs,
      responseMs,
      ok: responseMs === null ? null : responseMs <= node.periodMs,
      processName: group.name,
    };
  });
  for (const m of nodeMetrics) {
    if (m.ok === false) {
      findings.push({
        severity: "error",
        message: `「${m.label}」は次の周期までに終わりません（応答時間上界 ${fmt(m.responseMs!)} ms > 周期 ${m.periodMs} ms）`,
      });
    }
  }

  // --- A: プロセス利用率 ---
  const processMetrics: ProcessMetric[] = processes.map((proc) => {
    const members = proc.nodes
      .map((id) => byId.get(id))
      .filter((n): n is AnalysisNode => !!n);
    const capacity =
      proc.executor === "multi" ? (proc.threads ?? DEFAULT_MULTI_THREADS) : 1;
    const missing = members.filter((m) => m.wcetMs === undefined);
    let utilization: number | null = null;
    if (members.length > 0 && missing.length === 0) {
      utilization = members.reduce(
        (sum, m) => sum + (m.wcetMs ?? 0) / m.periodMs,
        0,
      );
    } else if (missing.length > 0) {
      findings.push({
        severity: "warn",
        message: `プロセス「${proc.name}」: WCET 未設定のノードがあり解析できません（${missing.map((m) => m.label).join(", ")}）`,
      });
    }
    const ok = utilization === null ? null : utilization <= capacity;
    if (ok === false) {
      findings.push({
        severity: "error",
        message: `プロセス「${proc.name}」の利用率 ${(utilization! * 100).toFixed(0)}% が容量（${capacity * 100}%）を超えています`,
      });
    }
    return {
      name: proc.name,
      utilization,
      capacity,
      ok,
      nodeLabels: members.map((m) => m.label),
    };
  });

  // --- C: レート整合 ---
  for (const edge of edges) {
    const src = byId.get(edge.source);
    const dst = byId.get(edge.target);
    if (!src || !dst) continue;
    const hi = Math.max(src.periodMs, dst.periodMs);
    const lo = Math.min(src.periodMs, dst.periodMs);
    if (lo > 0 && hi % lo !== 0) {
      findings.push({
        severity: "warn",
        message: `「${src.label}」(${src.periodMs}ms) → 「${dst.label}」(${dst.periodMs}ms) の周期が整数比でないため、位相ドリフトが発生します`,
      });
    }
  }

  // --- E: チェーン最悪レイテンシ（source → sink の最長経路）---
  for (const chain of longestChains(nodes, edges, 3)) {
    const parts: string[] = [];
    let total = 0;
    let complete = true;
    for (let i = 0; i < chain.length; i++) {
      const n = chain[i];
      const r = responseOf.get(n.id);
      if (r === undefined) {
        complete = false;
        break;
      }
      total += i < chain.length - 1 ? n.periodMs + r : r;
      parts.push(n.label);
    }
    if (complete && chain.length >= 2) {
      findings.push({
        severity: "info",
        message: `チェーン最悪レイテンシ: ${parts.join(" → ")} ≈ ${fmt(total)} ms（サンプリング遅れ + 実行遅れの上界）`,
      });
    }
  }

  // --- F: オフセット衝突ヒント ---
  for (const g of groups) {
    if (g.executor !== "single" || g.members.length < 2) continue;
    const seen = new Map<string, AnalysisNode>();
    for (const m of g.members) {
      if (m.wcetMs === undefined) continue;
      const key = `${m.periodMs}/${m.offsetMs}`;
      const other = seen.get(key);
      if (other) {
        findings.push({
          severity: "hint",
          message: `「${other.label}」と「${m.label}」は同一プロセス・同周期・同オフセットです。offset_ms をずらすと実行順序が安定します`,
        });
      } else {
        seen.set(key, m);
      }
    }
  }

  return { findings, processes: processMetrics, nodes: nodeMetrics };
}

function fmt(v: number): string {
  return Number.isInteger(v) ? String(v) : v.toFixed(1);
}

/** source → sink の経路をノード数の多い順に最大 limit 件返す（サイクルは打ち切り） */
function longestChains(
  nodes: AnalysisNode[],
  edges: AnalysisEdge[],
  limit: number,
): AnalysisNode[][] {
  const byId = new Map(nodes.map((n) => [n.id, n]));
  const out = new Map<string, string[]>();
  const hasIncoming = new Set<string>();
  for (const e of edges) {
    out.set(e.source, [...(out.get(e.source) ?? []), e.target]);
    hasIncoming.add(e.target);
  }
  const chains: AnalysisNode[][] = [];
  const walk = (id: string, path: string[]) => {
    if (path.includes(id)) return; // サイクル打ち切り
    const next = [...path, id];
    const targets = out.get(id) ?? [];
    if (targets.length === 0) {
      chains.push(next.map((x) => byId.get(x)!).filter(Boolean));
      return;
    }
    for (const t of targets) walk(t, next);
  };
  for (const n of nodes) {
    if (!hasIncoming.has(n.id) && (out.get(n.id) ?? []).length > 0) {
      walk(n.id, []);
    }
  }
  return chains.sort((a, b) => b.length - a.length).slice(0, limit);
}
