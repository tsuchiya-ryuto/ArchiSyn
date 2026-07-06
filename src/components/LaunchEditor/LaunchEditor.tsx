import { useModelStore } from "../../state/store";
import type { LaunchArg, LaunchConfig } from "../../types/arcsyn";
import { TextField } from "../common/TextField";

/**
 * launch 設計タブ（Phase 5.2）。
 * - 引数: 宣言され、全ノードに同名パラメータとして渡される（例: use_sim_time）
 * - 起動構成: サブシステムごとの launch/<名前>.launch.py を生成
 */
export function LaunchEditor() {
  const nodes = useModelStore((s) => s.nodes);
  const launchArgs = useModelStore((s) => s.launchArgs);
  const launchConfigs = useModelStore((s) => s.launchConfigs);
  const setLaunchArgs = useModelStore((s) => s.setLaunchArgs);
  const setLaunchConfigs = useModelStore((s) => s.setLaunchConfigs);

  const updateArg = (index: number, patch: Partial<LaunchArg>) =>
    setLaunchArgs(
      launchArgs.map((a, i) => (i === index ? { ...a, ...patch } : a)),
    );

  const updateConfig = (index: number, patch: Partial<LaunchConfig>) =>
    setLaunchConfigs(
      launchConfigs.map((c, i) => (i === index ? { ...c, ...patch } : c)),
    );

  const toggleNode = (config: LaunchConfig, index: number, nodeId: string) => {
    const next = config.nodes.includes(nodeId)
      ? config.nodes.filter((id) => id !== nodeId)
      : [...config.nodes, nodeId];
    updateConfig(index, { nodes: next });
  };

  return (
    <div className="launch-editor">
      <section className="inspector-section">
        <div className="inspector-section-header">
          <h3>launch 引数</h3>
          <button
            onClick={() =>
              setLaunchArgs([
                ...launchArgs,
                { name: `arg${launchArgs.length + 1}`, default: "" },
              ])
            }
          >
            + 追加
          </button>
        </div>
        <p className="sidebar-hint">
          宣言した引数は全ノードに同名パラメータとして渡されます（例:
          use_sim_time）
        </p>
        {launchArgs.map((a, i) => (
          <div key={i} className="edit-row">
            <TextField
              className="edit-name"
              value={a.name}
              onCommit={(v) => updateArg(i, { name: v })}
              placeholder="名前"
            />
            <TextField
              className="edit-type"
              value={a.default}
              onCommit={(v) => updateArg(i, { default: v })}
              placeholder="既定値"
            />
            <button
              className="remove-button"
              title="削除"
              onClick={() =>
                setLaunchArgs(launchArgs.filter((_, j) => j !== i))
              }
            >
              ×
            </button>
          </div>
        ))}
      </section>

      <section className="inspector-section">
        <div className="inspector-section-header">
          <h3>起動構成</h3>
          <button
            onClick={() =>
              setLaunchConfigs([
                ...launchConfigs,
                { name: `subsystem${launchConfigs.length + 1}`, nodes: [] },
              ])
            }
          >
            + 追加
          </button>
        </div>
        <p className="sidebar-hint">
          構成ごとに launch/&lt;名前&gt;.launch.py が生成されます（system
          は常に全ノードで生成）
        </p>
        {launchConfigs.map((c, i) => (
          <div key={i} className="type-card">
            <div className="inspector-section-header">
              <TextField
                className="edit-name type-name"
                value={c.name}
                onCommit={(v) => updateConfig(i, { name: v })}
                placeholder="構成名"
              />
              <button
                className="remove-button"
                title="構成を削除"
                onClick={() =>
                  setLaunchConfigs(launchConfigs.filter((_, j) => j !== i))
                }
              >
                ×
              </button>
            </div>
            {nodes.length === 0 && (
              <p className="sidebar-empty">ノードがありません</p>
            )}
            {nodes.map((n) => (
              <label key={n.id} className="launch-node-check">
                <input
                  type="checkbox"
                  checked={c.nodes.includes(n.id)}
                  onChange={() => toggleNode(c, i, n.id)}
                />
                <span>{n.data.label}</span>
              </label>
            ))}
          </div>
        ))}
      </section>
    </div>
  );
}
