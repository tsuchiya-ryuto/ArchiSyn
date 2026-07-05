import { create } from "zustand";
import {
  addEdge,
  applyEdgeChanges,
  applyNodeChanges,
  type Connection,
  type Edge,
  type EdgeChange,
  type Node,
  type NodeChange,
  type ReactFlowInstance,
} from "@xyflow/react";
import {
  isTypeCompatible,
  type ArchNodeData,
  type CustomType,
  type PortDef,
} from "../types/arcsyn";
import type { ProjectFile } from "../ipc/project";
import { fromProjectFile } from "../ipc/convert";

export type ArchNode = Node<ArchNodeData, "archNode">;

export type PortDirection = "inputs" | "outputs";

type ModelState = {
  nodes: ArchNode[];
  edges: Edge[];
  customTypes: CustomType[];
  nextId: number;
  connectionError: string | null;

  projectName: string;
  middleware: string;
  currentFilePath: string | null;
  fileStatus: string | null;
  rfInstance: ReactFlowInstance<ArchNode, Edge> | null;

  setProjectName: (name: string) => void;
  setMiddleware: (middleware: string) => void;
  setCurrentFilePath: (path: string | null) => void;
  setFileStatus: (status: string | null) => void;
  setRfInstance: (instance: ReactFlowInstance<ArchNode, Edge>) => void;
  applyProjectFile: (file: ProjectFile, path: string | null) => void;

  onNodesChange: (changes: NodeChange<ArchNode>[]) => void;
  onEdgesChange: (changes: EdgeChange[]) => void;
  onConnect: (connection: Connection) => void;
  isValidConnection: (edge: Edge | Connection) => boolean;
  clearConnectionError: () => void;

  addNode: () => void;
  deleteNode: (nodeId: string) => void;
  updateNodeData: (nodeId: string, patch: Partial<ArchNodeData>) => void;

  addPort: (nodeId: string, dir: PortDirection) => void;
  removePort: (nodeId: string, dir: PortDirection, name: string) => void;
  renamePort: (
    nodeId: string,
    dir: PortDirection,
    oldName: string,
    newName: string,
  ) => void;
  setPortType: (
    nodeId: string,
    dir: PortDirection,
    name: string,
    type: string,
  ) => void;

  addCustomType: () => void;
  updateCustomType: (index: number, type: CustomType) => void;
  removeCustomType: (index: number) => void;
};

function findPort(
  nodes: ArchNode[],
  nodeId: string | null,
  dir: PortDirection,
  portName: string | null | undefined,
): PortDef | undefined {
  const node = nodes.find((n) => n.id === nodeId);
  return node?.data[dir].find((p) => p.name === portName);
}

// 接続の妥当性を検査し、問題があればエラーメッセージを返す
function validateConnection(
  nodes: ArchNode[],
  edges: Edge[],
  conn: Edge | Connection,
): string | null {
  const source = findPort(nodes, conn.source, "outputs", conn.sourceHandle);
  const target = findPort(nodes, conn.target, "inputs", conn.targetHandle);
  if (!source || !target) return "ポートが見つかりません";

  const duplicated = edges.some(
    (e) =>
      e.source === conn.source &&
      e.sourceHandle === conn.sourceHandle &&
      e.target === conn.target &&
      e.targetHandle === conn.targetHandle,
  );
  if (duplicated) return "同じ接続が既に存在します";

  if (!isTypeCompatible(source.type, target.type)) {
    return `型が一致しません: ${source.type} → ${target.type}`;
  }
  return null;
}

// ポート定義と矛盾するエッジ（存在しないポート・型不一致）を取り除く
function pruneEdges(nodes: ArchNode[], edges: Edge[]): Edge[] {
  return edges.filter((e) => {
    const source = findPort(nodes, e.source, "outputs", e.sourceHandle);
    const target = findPort(nodes, e.target, "inputs", e.targetHandle);
    return source && target && isTypeCompatible(source.type, target.type);
  });
}

function uniquePortName(ports: PortDef[], base: string): string {
  let i = 1;
  while (ports.some((p) => p.name === `${base}${i}`)) i += 1;
  return `${base}${i}`;
}

function patchNodeData(
  nodes: ArchNode[],
  nodeId: string,
  patch: (data: ArchNodeData) => Partial<ArchNodeData>,
): ArchNode[] {
  return nodes.map((n) =>
    n.id === nodeId ? { ...n, data: { ...n.data, ...patch(n.data) } } : n,
  );
}

// 読込済みノード id（n1, n2, ...）と衝突しない連番の初期値を求める
function nextIdAfter(nodes: ArchNode[]): number {
  let max = 0;
  for (const n of nodes) {
    const m = /^n(\d+)$/.exec(n.id);
    if (m) max = Math.max(max, Number(m[1]));
  }
  return max + 1;
}

export const useModelStore = create<ModelState>()((set, get) => ({
  nodes: [],
  edges: [],
  customTypes: [],
  nextId: 1,
  connectionError: null,

  projectName: "my_project",
  middleware: "ros2_humble",
  currentFilePath: null,
  fileStatus: null,
  rfInstance: null,

  setProjectName: (name) => {
    const trimmed = name.trim();
    if (trimmed !== "") set({ projectName: trimmed });
  },
  setMiddleware: (middleware) => set({ middleware }),
  setCurrentFilePath: (path) => set({ currentFilePath: path }),
  setFileStatus: (status) => set({ fileStatus: status }),
  setRfInstance: (instance) => set({ rfInstance: instance }),

  applyProjectFile: (file, path) => {
    const { nodes, edges, customTypes } = fromProjectFile(file);
    set({
      nodes,
      edges,
      customTypes,
      nextId: nextIdAfter(nodes),
      projectName: file.project.name,
      middleware: file.project.middleware,
      currentFilePath: path,
      connectionError: null,
    });
    // ビューポート（ズーム・パン）も復元して GUI 完全復元とする
    get().rfInstance?.setViewport({
      x: file.viewport.pan.x,
      y: file.viewport.pan.y,
      zoom: file.viewport.zoom,
    });
  },

  onNodesChange: (changes) =>
    set((s) => ({ nodes: applyNodeChanges(changes, s.nodes) })),

  onEdgesChange: (changes) =>
    set((s) => ({ edges: applyEdgeChanges(changes, s.edges) })),

  onConnect: (connection) => {
    const { nodes, edges } = get();
    const error = validateConnection(nodes, edges, connection);
    if (error) {
      set({ connectionError: error });
      return;
    }
    set({ edges: addEdge(connection, edges), connectionError: null });
  },

  isValidConnection: (edge) => {
    const { nodes, edges } = get();
    return validateConnection(nodes, edges, edge) === null;
  },

  clearConnectionError: () => set({ connectionError: null }),

  addNode: () =>
    set((s) => {
      const id = s.nextId;
      const node: ArchNode = {
        id: `n${id}`,
        type: "archNode",
        position: { x: 80 + ((id - 1) % 5) * 70, y: 80 + ((id - 1) % 7) * 50 },
        data: {
          label: `Node${id}`,
          language: "python",
          periodMs: 100,
          inputs: [{ name: "in1", type: "std_msgs/Float64" }],
          outputs: [{ name: "out1", type: "std_msgs/Float64" }],
          params: [],
        },
      };
      return { nodes: [...s.nodes, node], nextId: id + 1 };
    }),

  deleteNode: (nodeId) =>
    set((s) => ({
      nodes: s.nodes.filter((n) => n.id !== nodeId),
      edges: s.edges.filter((e) => e.source !== nodeId && e.target !== nodeId),
    })),

  updateNodeData: (nodeId, patch) =>
    set((s) => ({ nodes: patchNodeData(s.nodes, nodeId, () => patch) })),

  addPort: (nodeId, dir) =>
    set((s) => ({
      nodes: patchNodeData(s.nodes, nodeId, (data) => ({
        [dir]: [
          ...data[dir],
          {
            name: uniquePortName(data[dir], dir === "inputs" ? "in" : "out"),
            type: "std_msgs/Float64",
          },
        ],
      })),
    })),

  removePort: (nodeId, dir, name) =>
    set((s) => {
      const nodes = patchNodeData(s.nodes, nodeId, (data) => ({
        [dir]: data[dir].filter((p) => p.name !== name),
      }));
      return { nodes, edges: pruneEdges(nodes, s.edges) };
    }),

  renamePort: (nodeId, dir, oldName, newName) =>
    set((s) => {
      const node = s.nodes.find((n) => n.id === nodeId);
      const trimmed = newName.trim();
      if (
        !node ||
        trimmed === "" ||
        trimmed === oldName ||
        node.data[dir].some((p) => p.name === trimmed)
      ) {
        return s; // 空文字・重複は無視
      }
      const nodes = patchNodeData(s.nodes, nodeId, (data) => ({
        [dir]: data[dir].map((p) =>
          p.name === oldName ? { ...p, name: trimmed } : p,
        ),
      }));
      // 接続済みエッジのハンドル名も追従させる
      const edges = s.edges.map((e) => {
        if (
          dir === "outputs" &&
          e.source === nodeId &&
          e.sourceHandle === oldName
        ) {
          return { ...e, sourceHandle: trimmed };
        }
        if (
          dir === "inputs" &&
          e.target === nodeId &&
          e.targetHandle === oldName
        ) {
          return { ...e, targetHandle: trimmed };
        }
        return e;
      });
      return { nodes, edges };
    }),

  setPortType: (nodeId, dir, name, type) =>
    set((s) => {
      const nodes = patchNodeData(s.nodes, nodeId, (data) => ({
        [dir]: data[dir].map((p) => (p.name === name ? { ...p, type } : p)),
      }));
      // 型変更で不整合になったエッジは取り除く
      return { nodes, edges: pruneEdges(nodes, s.edges) };
    }),

  addCustomType: () =>
    set((s) => {
      let i = 1;
      while (s.customTypes.some((t) => t.name === `NewType${i}`)) i += 1;
      return {
        customTypes: [
          ...s.customTypes,
          { name: `NewType${i}`, fields: [{ name: "value", type: "float64" }] },
        ],
      };
    }),

  updateCustomType: (index, type) =>
    set((s) => ({
      customTypes: s.customTypes.map((t, i) => (i === index ? type : t)),
    })),

  removeCustomType: (index) =>
    set((s) => ({
      customTypes: s.customTypes.filter((_, i) => i !== index),
    })),
}));
