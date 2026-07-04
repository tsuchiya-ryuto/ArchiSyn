type Props = {
  value: string;
  onCommit: (value: string) => void;
  placeholder?: string;
  list?: string;
  className?: string;
};

// blur / Enter で確定するテキスト入力。
// ポート名のリネームなど、入力途中の値でストアを更新したくない場面で使う。
// 非制御 input を value ごとに remount することで外部からの値変更に追従する。
export function TextField({
  value,
  onCommit,
  placeholder,
  list,
  className,
}: Props) {
  return (
    <input
      key={value}
      className={className}
      defaultValue={value}
      placeholder={placeholder}
      list={list}
      onBlur={(e) => {
        const draft = e.currentTarget.value;
        if (draft !== value) {
          onCommit(draft);
          // 確定が拒否された場合（重複名など）に備えて表示を戻す。
          // 受理された場合は key が変わり新しい値で remount される。
          e.currentTarget.value = value;
        }
      }}
      onKeyDown={(e) => {
        if (e.key === "Enter") {
          e.currentTarget.blur();
        } else if (e.key === "Escape") {
          e.currentTarget.value = value;
          e.currentTarget.blur();
        }
      }}
    />
  );
}
