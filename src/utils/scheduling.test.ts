import { describe, expect, it } from "vitest";
import {
  analyzeScheduling,
  type AnalysisNode,
  type SchedulingProcess,
} from "./scheduling";

const node = (
  id: string,
  periodMs: number,
  wcetMs?: number,
  overrides: Partial<AnalysisNode> = {},
): AnalysisNode => ({
  id,
  label: id.toUpperCase(),
  language: "python",
  periodMs,
  offsetMs: 0,
  wcetMs,
  ...overrides,
});

const proc = (
  name: string,
  nodes: string[],
  overrides: Partial<SchedulingProcess> = {},
): SchedulingProcess => ({
  name,
  executor: "single",
  nodes,
  ...overrides,
});

describe("analyzeScheduling", () => {
  it("デッドライン検証: 直列実行で周期内に収まるか", () => {
    // 同一 single プロセス: R_a = 5 + 30 = 35 ≤ 50 OK / R_b = 35 ≤ 100 OK
    const r = analyzeScheduling(
      [node("a", 50, 5), node("b", 100, 30)],
      [],
      [proc("p", ["a", "b"])],
    );
    expect(r.nodes.find((n) => n.id === "a")).toMatchObject({
      responseMs: 35,
      ok: true,
    });
    expect(r.findings.filter((f) => f.severity === "error")).toEqual([]);
  });

  it("デッドライン割れをエラーで検出する", () => {
    // R_a = 5 + 60 = 65 > 50 → NG
    const r = analyzeScheduling(
      [node("a", 50, 5), node("b", 100, 60)],
      [],
      [proc("p", ["a", "b"])],
    );
    expect(r.nodes.find((n) => n.id === "a")?.ok).toBe(false);
    expect(
      r.findings.some(
        (f) =>
          f.severity === "error" &&
          f.message.includes("次の周期までに終わりません"),
      ),
    ).toBe(true);
  });

  it("利用率超過をエラーで検出する", () => {
    // U = 40/50 + 50/100 = 1.3 > 1
    const r = analyzeScheduling(
      [node("a", 50, 40), node("b", 100, 50)],
      [],
      [proc("p", ["a", "b"])],
    );
    expect(r.processes[0].utilization).toBeCloseTo(1.3);
    expect(r.processes[0].ok).toBe(false);
  });

  it("multi executor はスレッド数まで許容する", () => {
    const r = analyzeScheduling(
      [node("a", 50, 40), node("b", 100, 50)],
      [],
      [proc("p", ["a", "b"], { executor: "multi", threads: 2 })],
    );
    expect(r.processes[0].capacity).toBe(2);
    expect(r.processes[0].ok).toBe(true);
    // multi は並列仮定なので R = 自身の WCET
    expect(r.nodes.find((n) => n.id === "a")?.responseMs).toBe(40);
  });

  it("WCET 未設定は警告し判定しない", () => {
    const r = analyzeScheduling(
      [node("a", 50, 5), node("b", 100)],
      [],
      [proc("p", ["a", "b"])],
    );
    expect(r.processes[0].utilization).toBeNull();
    expect(r.nodes.find((n) => n.id === "a")?.ok).toBeNull();
    expect(r.findings.some((f) => f.message.includes("WCET 未設定"))).toBe(
      true,
    );
  });

  it("周期の非整数比を警告する", () => {
    const r = analyzeScheduling(
      [node("a", 30, 1), node("b", 50, 1)],
      [{ source: "a", target: "b" }],
      [],
    );
    expect(r.findings.some((f) => f.message.includes("整数比でない"))).toBe(
      true,
    );
  });

  it("チェーン最悪レイテンシを算出する", () => {
    // a(50,R=5) → b(100,R=10): (50+5) + 10 = 65
    const r = analyzeScheduling(
      [node("a", 50, 5), node("b", 100, 10)],
      [{ source: "a", target: "b" }],
      [],
    );
    const chain = r.findings.find((f) =>
      f.message.includes("チェーン最悪レイテンシ"),
    );
    expect(chain?.message).toContain("65 ms");
  });

  it("配置エラー: 多重所属・不明 id・異言語混在", () => {
    const r = analyzeScheduling(
      [node("a", 50, 1), node("b", 50, 1, { language: "cpp" })],
      [],
      [proc("p1", ["a", "b", "ghost"]), proc("p2", ["a"])],
    );
    const errors = r.findings.filter((f) => f.severity === "error");
    expect(errors.some((f) => f.message.includes("ghost"))).toBe(true);
    expect(errors.some((f) => f.message.includes("複数のプロセス"))).toBe(true);
    expect(errors.some((f) => f.message.includes("異なる言語"))).toBe(true);
  });

  it("同周期・同オフセットの衝突をヒントで提案する", () => {
    const r = analyzeScheduling(
      [node("a", 50, 5), node("b", 50, 5)],
      [],
      [proc("p", ["a", "b"])],
    );
    expect(r.findings.some((f) => f.severity === "hint")).toBe(true);
  });
});
