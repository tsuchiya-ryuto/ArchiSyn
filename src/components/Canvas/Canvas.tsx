import {
  Background,
  Controls,
  MiniMap,
  Panel,
  ReactFlow,
  type NodeTypes,
} from "@xyflow/react";
import { useModelStore, type ArchNode } from "../../state/store";
import { ArchNodeView } from "./ArchNode";

const nodeTypes: NodeTypes = { archNode: ArchNodeView };

export function Canvas() {
  const nodes = useModelStore((s) => s.nodes);
  const edges = useModelStore((s) => s.edges);
  const onNodesChange = useModelStore((s) => s.onNodesChange);
  const onEdgesChange = useModelStore((s) => s.onEdgesChange);
  const onConnect = useModelStore((s) => s.onConnect);
  const isValidConnection = useModelStore((s) => s.isValidConnection);
  const addNode = useModelStore((s) => s.addNode);
  const connectionError = useModelStore((s) => s.connectionError);
  const clearConnectionError = useModelStore((s) => s.clearConnectionError);

  return (
    <div className="canvas-container">
      <ReactFlow<ArchNode>
        nodes={nodes}
        edges={edges}
        nodeTypes={nodeTypes}
        onNodesChange={onNodesChange}
        onEdgesChange={onEdgesChange}
        onConnect={onConnect}
        isValidConnection={isValidConnection}
        deleteKeyCode={["Delete", "Backspace"]}
        fitView
      >
        <Background />
        <Controls />
        <MiniMap />
        <Panel position="top-left">
          <button className="toolbar-button" onClick={addNode}>
            + ノード追加
          </button>
        </Panel>
        {connectionError && (
          <Panel position="bottom-center">
            <div
              className="error-toast"
              role="alert"
              onClick={clearConnectionError}
              title="クリックで閉じる"
            >
              {connectionError}
            </div>
          </Panel>
        )}
      </ReactFlow>
    </div>
  );
}
