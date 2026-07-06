import { useMemo, useState } from "react";
import { useModelStore } from "../../state/store";
import type { CustomType } from "../../types/arcsyn";
import { parseTypeTable } from "../../utils/tableParse";

const NAME_RE = /^[A-Za-z_][A-Za-z0-9_]*$/;

/**
 * 表計算ソフトからコピーした表を貼り付けてカスタム型を一括登録するダイアログ。
 * - 3列 [型名, フィールド名, 型]: 複数型を一括定義
 * - 2列 [フィールド名, 型]: 指定した1つの型として登録
 */
export function PasteTableDialog({ onClose }: { onClose: () => void }) {
  const customTypes = useModelStore((s) => s.customTypes);
  const upsertCustomTypes = useModelStore((s) => s.upsertCustomTypes);
  const [text, setText] = useState("");
  const [targetName, setTargetName] = useState("NewType");

  const result = useMemo(
    () => (text.trim() === "" ? null : parseTypeTable(text)),
    [text],
  );

  const resolvedTypes: CustomType[] = useMemo(() => {
    if (!result) return [];
    if (!result.fieldsOnly) return result.types;
    return result.types.map((t) => ({ ...t, name: targetName.trim() }));
  }, [result, targetName]);

  const nameInvalid =
    result?.fieldsOnly === true && !NAME_RE.test(targetName.trim());
  const canApply = resolvedTypes.length > 0 && !nameInvalid;
  const overwritten = resolvedTypes.filter((t) =>
    customTypes.some((x) => x.name === t.name),
  );

  const apply = () => {
    upsertCustomTypes(resolvedTypes);
    onClose();
  };

  return (
    <div className="dialog-overlay" onClick={onClose}>
      <div className="dialog" onClick={(e) => e.stopPropagation()}>
        <h3>表から型を一括登録</h3>
        <p className="dialog-hint">
          Excel / スプレッドシート / CSV からコピーした表を貼り付けてください。
          <br />
          3列 = [型名, フィールド名, 型]（複数型）、2列 = [フィールド名,
          型]（単一型）
        </p>
        <textarea
          className="paste-area"
          autoFocus
          placeholder={
            "FusedPose\tposition\tgeometry_msgs/Vector3\n\tconfidence\tfloat64"
          }
          value={text}
          onChange={(e) => setText(e.currentTarget.value)}
        />

        {result?.fieldsOnly && (
          <label className="field">
            <span>登録先の型名</span>
            <input
              value={targetName}
              className={nameInvalid ? "input-invalid" : ""}
              onChange={(e) => setTargetName(e.currentTarget.value)}
            />
          </label>
        )}

        {result && (
          <div className="paste-preview">
            {resolvedTypes.map((t) => (
              <div key={t.name} className="paste-preview-type">
                <strong>{t.name || "（型名未指定）"}</strong>
                <span className="paste-preview-count">
                  {t.fields.length} フィールド
                  {customTypes.some((x) => x.name === t.name) &&
                    "（既存を上書き）"}
                </span>
                <div className="paste-preview-fields">
                  {t.fields.map((f) => `${f.name}: ${f.type}`).join(", ")}
                </div>
              </div>
            ))}
            {result.warnings.map((w, i) => (
              <div key={i} className="paste-warning">
                ⚠ {w}
              </div>
            ))}
          </div>
        )}

        <div className="dialog-actions">
          <button onClick={onClose}>キャンセル</button>
          <button
            className="generate-button"
            disabled={!canApply}
            onClick={apply}
            title={
              overwritten.length > 0
                ? `${overwritten.map((t) => t.name).join(", ")} を上書きします`
                : undefined
            }
          >
            {resolvedTypes.length > 0
              ? `${resolvedTypes.length} 型を登録`
              : "登録"}
          </button>
        </div>
      </div>
    </div>
  );
}
