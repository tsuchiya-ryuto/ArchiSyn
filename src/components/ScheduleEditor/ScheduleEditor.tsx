import { useMemo } from "react";
import { useModelStore } from "../../state/store";
import {
  analyzeScheduling,
  type SchedulingProcess,
} from "../../utils/scheduling";
import { TextField } from "../common/TextField";

const SEVERITY_ICON: Record<string, string> = {
  error: "✖",
  warn: "⚠",
  info: "ℹ",
  hint: "💡",
};

/**
 * スケジュール設計タブ（Phase 5.4）。
 * プロセス配置の編集と、静的解析（利用率・デッドライン・レート整合・
 * チェーンレイテンシ）のリアルタイム表示を行う。
 * 解析モデルは doc/scheduling_design.md §2 参照。
 */
export function ScheduleEditor() {
  const nodes = useModelStore((s) => s.nodes);
  const edges = useModelStore((s) => s.edges);
  const processes = useModelStore((s) => s.schedulingProcesses);
  const setProcesses = useModelStore((s) => s.setSchedulingProcesses);

  const analysis = useMemo(
    () =>
      analyzeScheduling(
        nodes.map((n) => ({
          id: n.id,
          label: n.data.label,
          language: n.data.language,
          periodMs: n.data.periodMs,
          offsetMs: n.data.offsetMs,
          wcetMs: n.data.wcetMs,
        })),
        edges.map((e) => ({ source: e.source, target: e.target })),
        processes,
      ),
    [nodes, edges, processes],
  );

  const updateProcess = (index: number, patch: Partial<SchedulingProcess>) =>
    setProcesses(
      processes.map((p, i) => (i === index ? { ...p, ...patch } : p)),
    );

  const toggleNode = (
    proc: SchedulingProcess,
    index: number,
    nodeId: string,
  ) => {
    const next = proc.nodes.includes(nodeId)
      ? proc.nodes.filter((id) => id !== nodeId)
      : [...proc.nodes, nodeId];
    updateProcess(index, { nodes: next });
  };

  return (
    <div className="schedule-editor">
      <section className="inspector-section">
        <div className="inspector-section-header">
          <h3>プロセス配置</h3>
          <button
            onClick={() =>
              setProcesses([
                ...processes,
                {
                  name: `proc${processes.length + 1}`,
                  executor: "single",
                  nodes: [],
                },
              ])
            }
          >
            + 追加
          </button>
        </div>
        <p className="sidebar-hint">
          同一プロセス（single）のノードは直列実行されます。未所属ノードは単独プロセスです
        </p>
        {processes.map((p, i) => (
          <div key={i} className="type-card">
            <div className="inspector-section-header">
              <TextField
                className="edit-name type-name"
                value={p.name}
                onCommit={(v) => updateProcess(i, { name: v })}
                placeholder="プロセス名"
              />
              <button
                className="remove-button"
                title="プロセスを削除"
                onClick={() =>
                  setProcesses(processes.filter((_, j) => j !== i))
                }
              >
                ×
              </button>
            </div>
            <div className="edit-row">
              <select
                value={p.executor}
                onChange={(e) =>
                  updateProcess(i, {
                    executor: e.currentTarget.value as "single" | "multi",
                  })
                }
              >
                <option value="single">single</option>
                <option value="multi">multi</option>
              </select>
              {p.executor === "multi" && (
                <input
                  type="number"
                  min={2}
                  value={p.threads ?? 2}
                  title="スレッド数"
                  onChange={(e) =>
                    updateProcess(i, { threads: Number(e.currentTarget.value) })
                  }
                />
              )}
              <input
                type="number"
                value={p.priority ?? ""}
                placeholder="RT優先度"
                title="RT 優先度（SCHED_FIFO。launch の chrt に反映予定）"
                onChange={(e) => {
                  const raw = e.currentTarget.value;
                  updateProcess(i, {
                    priority: raw === "" ? undefined : Number(raw),
                  });
                }}
              />
              <TextField
                className="edit-type"
                value={(p.cpu_affinity ?? []).join(",")}
                placeholder="CPU (例 0,1)"
                onCommit={(v) =>
                  updateProcess(i, {
                    cpu_affinity: v
                      .split(",")
                      .map((s) => Number(s.trim()))
                      .filter((n) => Number.isInteger(n) && n >= 0),
                  })
                }
              />
            </div>
            {nodes.map((n) => (
              <label key={n.id} className="launch-node-check">
                <input
                  type="checkbox"
                  checked={p.nodes.includes(n.id)}
                  onChange={() => toggleNode(p, i, n.id)}
                />
                <span>
                  {n.data.label}
                  <span className="sched-node-meta">
                    {" "}
                    {n.data.periodMs}ms / WCET{" "}
                    {n.data.wcetMs !== undefined
                      ? `${n.data.wcetMs}ms`
                      : "未設定"}
                  </span>
                </span>
              </label>
            ))}
          </div>
        ))}
      </section>

      <section className="inspector-section">
        <h3>解析結果</h3>
        {analysis.processes.length > 0 && (
          <table className="sched-table">
            <thead>
              <tr>
                <th>プロセス</th>
                <th>利用率</th>
                <th>判定</th>
              </tr>
            </thead>
            <tbody>
              {analysis.processes.map((p) => (
                <tr key={p.name}>
                  <td>{p.name}</td>
                  <td>
                    {p.utilization === null
                      ? "—"
                      : `${(p.utilization * 100).toFixed(0)}% / ${p.capacity * 100}%`}
                  </td>
                  <td>{p.ok === null ? "—" : p.ok ? "✅" : "✖"}</td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
        <table className="sched-table">
          <thead>
            <tr>
              <th>ノード</th>
              <th>応答上界 / 周期</th>
              <th>判定</th>
            </tr>
          </thead>
          <tbody>
            {analysis.nodes.map((n) => (
              <tr key={n.id}>
                <td title={n.processName}>{n.label}</td>
                <td>
                  {n.responseMs === null ? "—" : `${n.responseMs}`} /{" "}
                  {n.periodMs} ms
                </td>
                <td>{n.ok === null ? "—" : n.ok ? "✅" : "✖"}</td>
              </tr>
            ))}
          </tbody>
        </table>
        {analysis.findings.map((f, i) => (
          <div key={i} className={`sched-finding sched-${f.severity}`}>
            {SEVERITY_ICON[f.severity]} {f.message}
          </div>
        ))}
        {analysis.findings.length === 0 && nodes.length > 0 && (
          <p className="sidebar-hint">指摘はありません</p>
        )}
      </section>
    </div>
  );
}
