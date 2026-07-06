import { useMemo, useRef, useState } from "react";
import { useModelStore } from "../../state/store";
import { ROS_TYPE_CATALOG } from "../../types/rosTypes";
import { searchTypes, type TypeCandidate } from "../../utils/typeSearch";

type Props = {
  value: string;
  onCommit: (value: string) => void;
  placeholder?: string;
  className?: string;
  /** カタログに追加する候補（例: カスタム型フィールド用のプリミティブ型） */
  extra?: readonly string[];
};

/**
 * 型の検索付き入力フィールド。
 * ROS 型カタログ（std_msgs / sensor_msgs / geometry_msgs 等）とカスタム型を
 * 部分一致検索でき、自由入力もそのまま確定できる。
 */
export function TypeSearchField({
  value,
  onCommit,
  placeholder,
  className,
  extra,
}: Props) {
  const customTypes = useModelStore((s) => s.customTypes);
  const [draft, setDraft] = useState<string | null>(null); // null = 非編集
  const [highlight, setHighlight] = useState(0);
  const listRef = useRef<HTMLUListElement>(null);

  const candidates: TypeCandidate[] = useMemo(
    () => [
      ...customTypes.map((t) => ({ value: t.name, custom: true })),
      ...(extra ?? []).map((t) => ({ value: t, custom: false })),
      ...ROS_TYPE_CATALOG.map((t) => ({ value: t, custom: false })),
    ],
    [customTypes, extra],
  );

  const open = draft !== null;
  const results = useMemo(
    () => (open ? searchTypes(draft ?? "", candidates, 30) : []),
    [open, draft, candidates],
  );

  const commit = (v: string) => {
    const trimmed = v.trim();
    if (trimmed !== "" && trimmed !== value) onCommit(trimmed);
    setDraft(null);
  };

  const scrollTo = (index: number) => {
    listRef.current?.children[index]?.scrollIntoView({ block: "nearest" });
  };

  return (
    <div className={`type-search ${className ?? ""}`}>
      <input
        value={draft ?? value}
        placeholder={placeholder}
        onFocus={() => {
          setDraft(value);
          setHighlight(0);
        }}
        onChange={(e) => {
          setDraft(e.currentTarget.value);
          setHighlight(0);
        }}
        onBlur={() => {
          // 候補クリック（mousedown で確定）を先に処理させる
          if (draft !== null) commit(draft);
        }}
        onKeyDown={(e) => {
          if (!open) return;
          if (e.key === "ArrowDown") {
            e.preventDefault();
            const next = Math.min(highlight + 1, results.length - 1);
            setHighlight(next);
            scrollTo(next);
          } else if (e.key === "ArrowUp") {
            e.preventDefault();
            const next = Math.max(highlight - 1, 0);
            setHighlight(next);
            scrollTo(next);
          } else if (e.key === "Enter") {
            e.preventDefault();
            commit(results[highlight]?.value ?? draft ?? value);
            e.currentTarget.blur();
          } else if (e.key === "Escape") {
            setDraft(null);
            e.currentTarget.blur();
          }
        }}
      />
      {open && results.length > 0 && (
        <ul className="type-search-list" ref={listRef}>
          {results.map((c, i) => (
            <li
              key={c.value}
              className={i === highlight ? "active" : ""}
              // blur より先に発火させるため mousedown で確定する
              onMouseDown={(e) => {
                e.preventDefault();
                commit(c.value);
              }}
              onMouseEnter={() => setHighlight(i)}
            >
              <span>{c.value}</span>
              {c.custom && <span className="type-badge">カスタム</span>}
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}
