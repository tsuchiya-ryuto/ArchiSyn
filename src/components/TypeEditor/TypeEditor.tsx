import { useModelStore } from "../../state/store";
import {
  PRIMITIVE_TYPES,
  ROS_MSG_TYPES,
  type CustomType,
} from "../../types/arcsyn";
import { TextField } from "../common/TextField";

const FIELD_TYPES_DATALIST = "field-type-options";

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
          <TextField
            className="edit-type"
            value={f.type}
            onCommit={(v) =>
              setFields(
                type.fields.map((x, j) => (j === i ? { ...x, type: v } : x)),
              )
            }
            placeholder="型"
            list={FIELD_TYPES_DATALIST}
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

  const fieldTypeOptions = [
    ...PRIMITIVE_TYPES,
    ...ROS_MSG_TYPES,
    ...customTypes.map((t) => t.name),
  ];

  return (
    <div className="type-editor">
      <datalist id={FIELD_TYPES_DATALIST}>
        {fieldTypeOptions.map((t) => (
          <option key={t} value={t} />
        ))}
      </datalist>

      <div className="inspector-section-header">
        <h3>カスタム型</h3>
        <button onClick={addCustomType}>+ 型を追加</button>
      </div>
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
