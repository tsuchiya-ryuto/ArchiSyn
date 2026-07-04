import { Handle, Position, type NodeProps } from "@xyflow/react";
import type { ArchNode } from "../../state/store";

export function ArchNodeView({ data, selected }: NodeProps<ArchNode>) {
  return (
    <div className={`arch-node${selected ? " selected" : ""}`}>
      <div className="arch-node-header">
        <span className="arch-node-label">{data.label}</span>
        <span className={`lang-badge lang-${data.language}`}>
          {data.language}
        </span>
      </div>
      <div className="arch-node-period">{data.periodMs} ms</div>
      <div className="arch-node-ports">
        <div className="ports-col">
          {data.inputs.map((p) => (
            <div key={p.name} className="port-row port-row-in" title={p.type}>
              <Handle
                type="target"
                position={Position.Left}
                id={p.name}
                className="port-handle"
              />
              <span className="port-name">{p.name}</span>
            </div>
          ))}
        </div>
        <div className="ports-col ports-col-out">
          {data.outputs.map((p) => (
            <div key={p.name} className="port-row port-row-out" title={p.type}>
              <span className="port-name">{p.name}</span>
              <Handle
                type="source"
                position={Position.Right}
                id={p.name}
                className="port-handle"
              />
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}
