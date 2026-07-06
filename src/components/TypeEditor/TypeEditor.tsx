import { useState } from "react";
import { useModelStore } from "../../state/store";
import { PRIMITIVE_TYPES, type CustomType } from "../../types/arcsyn";
import { typesToTsv } from "../../utils/tableParse";
import { TextField } from "../common/TextField";
import { TypeSearchField } from "../common/TypeSearchField";
import { PasteTableDialog } from "./PasteTableDialog";

function CustomTypeCard({ index, type }: { index: number; type: CustomType }) {
  const customTypes = useModelStore((s) => s.customTypes);
  const updateCustomType = useModelStore((s) => s.updateCustomType);
  const removeCustomType = useModelStore((s) => s.removeCustomType);

  const rename = (name: string) => {
    const trimmed = name.trim();
    if (
      trimmed === "" ||
      customTypes.some((t, i) => i !== index && t.name === trimmed)
    ) {
      return; // 空文字・重複は無視
    }
    updateCustomType(index, { ...type, name: trimmed });
  };

  const setFields = (fields: CustomType["fields"]) =>
    updateCustomType(index, { ...type, fields });

  return (
    <div className="type-card">
      <div className="inspector-section-header">
        <TextField
          className="edit-name type-name"
          value={type.name}
          onCommit={rename}
          placeholder="型名"
        />
        <button
          className="remove-button"
          title="型を削除"
          onClick={() => removeCustomType(index)}
        >
          ×
        </button>
      </div>
      {type.fields.map((f, i) => (
        <div key={i} className="edit-row">
          <TextField
            className="edit-name"
            value={f.name}
            onCommit={(v) =>
              setFields(
                type.fields.map((x, j) => (j === i ? { ...x, name: v } : x)),
              )
            }
            placeholder="フィールド名"
          />
          <TypeSearchField
            className="edit-type"
            value={f.type}
            onCommit={(v) =>
              setFields(
                type.fields.map((x, j) => (j === i ? { ...x, type: v } : x)),
              )
            }
            placeholder="型を検索..."
            extra={PRIMITIVE_TYPES}
          />
          <button
            className="remove-button"
            title="削除"
            onClick={() => setFields(type.fields.filter((_, j) => j !== i))}
          >
            ×
          </button>
        </div>
      ))}
      <button
        onClick={() =>
          setFields([
            ...type.fields,
            { name: `field${type.fields.length + 1}`, type: "float64" },
          ])
        }
      >
        + フィールド追加
      </button>
    </div>
  );
}

export function TypeEditor() {
  const customTypes = useModelStore((s) => s.customTypes);
  const addCustomType = useModelStore((s) => s.addCustomType);
  const setFileStatus = useModelStore((s) => s.setFileStatus);
  const [pasteOpen, setPasteOpen] = useState(false);

  const copyAsTsv = async () => {
    try {
      await navigator.clipboard.writeText(typesToTsv(customTypes));
      setFileStatus(`${customTypes.length} 型を TSV でコピーしました`);
    } catch (e) {
      console.error(e);
    }
  };

  return (
    <div className="type-editor">
      <div className="inspector-section-header">
        <h3>カスタム型</h3>
        <div className="type-editor-actions">
          <button
            onClick={() => setPasteOpen(true)}
            title="Excel / スプレッドシート / CSV からコピーした表を貼り付けて一括登録"
          >
            表から貼り付け
          </button>
          {customTypes.length > 0 && (
            <button
              onClick={() => void copyAsTsv()}
              title="全型定義を TSV としてクリップボードへコピー（表計算ソフトに貼り付け可能）"
            >
              TSV コピー
            </button>
          )}
          <button onClick={addCustomType}>+ 型を追加</button>
        </div>
      </div>
      {pasteOpen && <PasteTableDialog onClose={() => setPasteOpen(false)} />}
      {customTypes.length === 0 && (
        <p className="sidebar-empty">
          カスタム型を追加すると、ポートの型として使えます
        </p>
      )}
      {customTypes.map((t, i) => (
        <CustomTypeCard key={i} index={i} type={t} />
      ))}
    </div>
  );
}
