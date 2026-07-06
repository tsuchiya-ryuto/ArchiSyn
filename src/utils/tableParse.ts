// 表計算ソフト（Excel / Google スプレッドシート / CSV）からコピーした
// クリップボードテキストをカスタム型定義へ変換する（Phase 5.1）。
// Excel 等のセル範囲コピーはタブ区切り（TSV）でクリップボードに入る。

import type { CustomType } from "../types/arcsyn";

export type ParseResult = {
  /** 解釈された型（3列モード: 複数型 / 2列モード: フィールドのみの単一グループ） */
  types: CustomType[];
  /** 2列モードのとき true（対象の型名をユーザーが指定する必要がある） */
  fieldsOnly: boolean;
  /** 行単位の警告（スキップした行など） */
  warnings: string[];
};

/** 型名・フィールド名として妥当か（生成コードの識別子になるため） */
const NAME_RE = /^[A-Za-z_][A-Za-z0-9_]*$/;
/** 型は プリミティブ / CustomType 名 / pkg/Type 形式を許容 */
const TYPE_RE = /^[A-Za-z_][A-Za-z0-9_]*(\/[A-Za-z_][A-Za-z0-9_]*)?$/;

const HEADER_WORDS = [
  "name",
  "field",
  "type",
  "typename",
  "型",
  "型名",
  "フィールド",
  "フィールド名",
  "名前",
];

function splitRow(line: string, delimiter: string): string[] {
  return line
    .split(delimiter)
    .map((cell) => cell.trim().replace(/^"(.*)"$/, "$1"));
}

function isHeaderRow(cells: string[]): boolean {
  const normalized = cells.map((c) => c.toLowerCase().replace(/[\s_]/g, ""));
  return normalized.some((c) => HEADER_WORDS.includes(c));
}

/**
 * 表テキストをパースする。
 * - 3列 [型名, フィールド名, 型]: 複数の型を一括定義（型名は連続行で省略可）
 * - 2列 [フィールド名, 型]: 対象の型へのフィールド一括追加（fieldsOnly=true）
 */
export function parseTypeTable(text: string): ParseResult {
  const warnings: string[] = [];
  const lines = text
    .split(/\r\n|\r|\n/)
    .map((l) => l.replace(/\u3000/g, " ")) // 全角スペースを正規化
    .filter((l) => l.trim() !== "");

  if (lines.length === 0) {
    return { types: [], fieldsOnly: false, warnings: ["入力が空です"] };
  }

  // 区切り文字: タブがあれば TSV、なければ CSV
  const delimiter = text.includes("\t") ? "\t" : ",";

  let rows = lines.map((l) => splitRow(l, delimiter));
  if (isHeaderRow(rows[0])) {
    rows = rows.slice(1);
  }

  // 列数は「値が入っているセル数」の最頻値で判定（末尾の空セルは無視）
  const width = (cells: string[]) => {
    let w = cells.length;
    while (w > 0 && cells[w - 1] === "") w -= 1;
    return w;
  };
  const maxWidth = Math.max(...rows.map(width), 0);
  const fieldsOnly = maxWidth <= 2;

  const types: CustomType[] = [];
  let current: CustomType | null = null;

  const findOrCreate = (name: string): CustomType => {
    const existing = types.find((t) => t.name === name);
    if (existing) return existing;
    const created: CustomType = { name, fields: [] };
    types.push(created);
    return created;
  };

  rows.forEach((cells, index) => {
    const lineNo = index + 1;
    const w = width(cells);
    if (w === 0) return;

    let typeName: string | null = null;
    let fieldName: string;
    let fieldType: string;

    if (fieldsOnly) {
      if (w < 2) {
        warnings.push(`${lineNo}行目: 列が不足しています（フィールド名, 型）`);
        return;
      }
      [fieldName, fieldType] = [cells[0], cells[1]];
    } else {
      if (w < 3) {
        // 型名セルが空（直前の型の続き）を許容
        if (w === 2 && current) {
          [fieldName, fieldType] = [cells[0], cells[1]];
          typeName = current.name;
        } else {
          warnings.push(
            `${lineNo}行目: 列が不足しています（型名, フィールド名, 型）`,
          );
          return;
        }
      } else {
        typeName = cells[0] !== "" ? cells[0] : (current?.name ?? null);
        [fieldName, fieldType] = [cells[1], cells[2]];
      }
      if (typeName === null) {
        warnings.push(`${lineNo}行目: 型名がありません`);
        return;
      }
      if (!NAME_RE.test(typeName)) {
        warnings.push(`${lineNo}行目: 型名「${typeName}」が不正です`);
        return;
      }
    }

    if (!NAME_RE.test(fieldName)) {
      warnings.push(`${lineNo}行目: フィールド名「${fieldName}」が不正です`);
      return;
    }
    if (!TYPE_RE.test(fieldType)) {
      warnings.push(`${lineNo}行目: 型「${fieldType}」が不正です`);
      return;
    }

    const target = fieldsOnly
      ? (current ??= findOrCreate(""))
      : findOrCreate(typeName!);
    if (!fieldsOnly) current = target;

    if (target.fields.some((f) => f.name === fieldName)) {
      warnings.push(
        `${lineNo}行目: フィールド「${fieldName}」が重複しているためスキップしました`,
      );
      return;
    }
    target.fields.push({ name: fieldName, type: fieldType });
  });

  const nonEmpty = types.filter((t) => t.fields.length > 0);
  if (nonEmpty.length === 0) {
    warnings.push("有効な行がありませんでした");
  }
  return { types: nonEmpty, fieldsOnly, warnings };
}

/** 型定義を TSV（型名, フィールド名, 型）へ変換する（表計算ソフトへの逆方向）。
 * 型名は全行に明記する（空欄にすると表計算側でソート・フィルタが壊れ、
 * 「同じ型の連続行がスキップされている」ように見えるため）。 */
export function typesToTsv(types: CustomType[]): string {
  const rows: string[] = ["型名\tフィールド名\t型"];
  for (const t of types) {
    for (const f of t.fields) {
      rows.push(`${t.name}\t${f.name}\t${f.type}`);
    }
  }
  return rows.join("\n");
}
