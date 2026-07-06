// ポート/フィールドの型検索（Phase 5.1 追加要望）。
// カタログの ROS 型とカスタム型を対象に、部分一致でランク付けする。

export type TypeCandidate = {
  value: string;
  /** カスタム型なら true（一覧でバッジ表示・優先表示） */
  custom: boolean;
};

/**
 * 検索クエリで候補を絞り込む。
 * ランク: 型名の前方一致 > 型名の部分一致 > 全体（pkg/Type）の部分一致。
 * 同ランクではカスタム型を先に、その後アルファベット順。
 */
export function searchTypes(
  query: string,
  candidates: TypeCandidate[],
  limit = 50,
): TypeCandidate[] {
  const q = query.trim().toLowerCase();

  const rank = (c: TypeCandidate): number => {
    if (q === "") return 2;
    const full = c.value.toLowerCase();
    const typeName = (c.value.split("/").pop() ?? c.value).toLowerCase();
    if (typeName.startsWith(q)) return 0;
    if (typeName.includes(q)) return 1;
    if (full.includes(q)) return 2;
    return -1;
  };

  return candidates
    .map((c) => ({ c, r: rank(c) }))
    .filter((x) => x.r >= 0)
    .sort((a, b) => {
      if (a.r !== b.r) return a.r - b.r;
      if (a.c.custom !== b.c.custom) return a.c.custom ? -1 : 1;
      return a.c.value.localeCompare(b.c.value);
    })
    .slice(0, limit)
    .map((x) => x.c);
}
