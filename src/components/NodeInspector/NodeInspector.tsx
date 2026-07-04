import { useModelStore, type PortDirection } from "../../state/store";
import {
  LANGUAGES,
  PARAM_TYPES,
  ROS_MSG_TYPES,
  type Language,
  type ParamDef,
} from "../../types/arcsyn";
import { TextField } from "../common/TextField";

const PORT_TYPES_DATALIST = "port-type-options";
const PARAM_TYPES_DATALIST = "param-type-options";

function PortListEditor({
  nodeId,
  dir,
  title,
}: {
  nodeId: string;
  dir: PortDirection;
  title: string;
}) {
  const ports = useModelStore(
    (s) => s.nodes.find((n) => n.id === nodeId)?.data[dir] ?? [],
  );
  const addPort = useModelStore((s) => s.addPort);
  const removePort = useModelStore((s) => s.removePort);
  const renamePort = useModelStore((s) => s.renamePort);
  const setPortType = useModelStore((s) => s.setPortType);

  return (
    <section className="inspector-section">
      <div className="inspector-section-header">
        <h3>{title}</h3>
        <button onClick={() => addPort(nodeId, dir)}>+ 追加</button>
      </div>
      {ports.map((p) => (
        <div key={p.name} className="edit-row">
          <TextField
            className="edit-name"
            value={p.name}
            onCommit={(v) => renamePort(nodeId, dir, p.name, v)}
            placeholder="ポート名"
          />
          <TextField
            className="edit-type"
            value={p.type}
            onCommit={(v) => setPortType(nodeId, dir, p.name, v)}
            placeholder="型"
            list={PORT_TYPES_DATALIST}
          />
          <button
            className="remove-button"
            title="削除"
            onClick={() => removePort(nodeId, dir, p.name)}
          >
            ×
          </button>
        </div>
      ))}
    </section>
  );
}

function ParamListEditor({ nodeId }: { nodeId: string }) {
  const params = useModelStore(
    (s) => s.nodes.find((n) => n.id === nodeId)?.data.params ?? [],
  );
  const updateNodeData = useModelStore((s) => s.updateNodeData);

  const setParams = (next: ParamDef[]) =>
    updateNodeData(nodeId, { params: next });

  const updateAt = (index: number, patch: Partial<ParamDef>) =>
    setParams(params.map((p, i) => (i === index ? { ...p, ...patch } : p)));

  return (
    <section className="inspector-section">
      <div className="inspector-section-header">
        <h3>パラメータ</h3>
        <button
          onClick={() =>
            setParams([
              ...params,
              {
                name: `param${params.length + 1}`,
                type: "float64",
                default: "0.0",
              },
            ])
          }
        >
          + 追加
        </button>
      </div>
      {params.map((p, i) => (
        <div key={i} className="edit-row">
          <TextField
            className="edit-name"
            value={p.name}
            onCommit={(v) => updateAt(i, { name: v })}
            placeholder="名前"
          />
          <TextField
            className="edit-type"
            value={p.type}
            onCommit={(v) => updateAt(i, { type: v })}
            placeholder="型"
            list={PARAM_TYPES_DATALIST}
          />
          <TextField
            className="edit-default"
            value={p.default}
            onCommit={(v) => updateAt(i, { default: v })}
            placeholder="既定値"
          />
          <button
            className="remove-button"
            title="削除"
            onClick={() => setParams(params.filter((_, j) => j !== i))}
          >
            ×
          </button>
        </div>
      ))}
    </section>
  );
}

export function NodeInspector() {
  const selected = useModelStore((s) => s.nodes.find((n) => n.selected));
  const customTypes = useModelStore((s) => s.customTypes);
  const updateNodeData = useModelStore((s) => s.updateNodeData);
  const deleteNode = useModelStore((s) => s.deleteNode);

  if (!selected) {
    return (
      <p className="sidebar-empty">
        ノードを選択するとプロパティを編集できます
      </p>
    );
  }

  const portTypeOptions = [...ROS_MSG_TYPES, ...customTypes.map((t) => t.name)];

  return (
    <div className="inspector">
      <datalist id={PORT_TYPES_DATALIST}>
        {portTypeOptions.map((t) => (
          <option key={t} value={t} />
        ))}
      </datalist>
      <datalist id={PARAM_TYPES_DATALIST}>
        {PARAM_TYPES.map((t) => (
          <option key={t} value={t} />
        ))}
      </datalist>

      <section className="inspector-section">
        <label className="field">
          <span>ラベル</span>
          <TextField
            value={selected.data.label}
            onCommit={(v) => updateNodeData(selected.id, { label: v })}
          />
        </label>
        <label className="field">
          <span>言語</span>
          <select
            value={selected.data.language}
            onChange={(e) =>
              updateNodeData(selected.id, {
                language: e.currentTarget.value as Language,
              })
            }
          >
            {LANGUAGES.map((lang) => (
              <option key={lang} value={lang}>
                {lang}
              </option>
            ))}
          </select>
        </label>
        <label className="field">
          <span>実行周期 [ms]</span>
          <input
            type="number"
            min={1}
            value={selected.data.periodMs}
            onChange={(e) => {
              const v = Number(e.currentTarget.value);
              if (Number.isFinite(v) && v > 0) {
                updateNodeData(selected.id, { periodMs: v });
              }
            }}
          />
        </label>
      </section>

      <PortListEditor nodeId={selected.id} dir="inputs" title="入力ポート" />
      <PortListEditor nodeId={selected.id} dir="outputs" title="出力ポート" />
      <ParamListEditor nodeId={selected.id} />

      <button className="danger-button" onClick={() => deleteNode(selected.id)}>
        ノードを削除
      </button>
    </div>
  );
}
