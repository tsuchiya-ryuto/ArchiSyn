import type { Edge } from "@xyflow/react";
import type { CustomType } from "../types/arcsyn";
import type { ArchNode } from "../state/store";
import type { FileEdge, FileNode, ProjectFile } from "./project";

const ARCSYN_VERSION = "0.1";

type ModelSnapshot = {
  nodes: ArchNode[];
  edges: Edge[];
  customTypes: CustomType[];
  projectName: string;
  middleware: string;
  viewport: { x: number; y: number; zoom: number };
};

// ストアの状態を .arcsyn ファイル表現へ変換する。
// Git 差分を安定させるため座標は丸める（doc/plan.md §6）。
export function toProjectFile(snapshot: ModelSnapshot): ProjectFile {
  return {
    arcsyn_version: ARCSYN_VERSION,
    project: { name: snapshot.projectName, middleware: snapshot.middleware },
    custom_types: snapshot.customTypes,
    nodes: snapshot.nodes.map(toFileNode),
    edges: snapshot.edges.map(toFileEdge),
    viewport: {
      zoom: Math.round(snapshot.viewport.zoom * 100) / 100,
      pan: {
        x: Math.round(snapshot.viewport.x),
        y: Math.round(snapshot.viewport.y),
      },
    },
  };
}

function toFileNode(node: ArchNode): FileNode {
  const size =
    node.measured?.width != null && node.measured?.height != null
      ? {
          w: Math.round(node.measured.width),
          h: Math.round(node.measured.height),
        }
      : undefined;
  return {
    id: node.id,
    label: node.data.label,
    language: node.data.language,
    period_ms: node.data.periodMs,
    position: {
      x: Math.round(node.position.x),
      y: Math.round(node.position.y),
    },
    ...(size ? { size } : {}),
    inputs: node.data.inputs,
    outputs: node.data.outputs,
    params: node.data.params,
  };
}

function toFileEdge(edge: Edge): FileEdge {
  return {
    id: edge.id,
    source: { node: edge.source, port: edge.sourceHandle ?? "" },
    target: { node: edge.target, port: edge.targetHandle ?? "" },
  };
}

// .arcsyn ファイル表現からストアの状態を復元する
export function fromProjectFile(file: ProjectFile): {
  nodes: ArchNode[];
  edges: Edge[];
  customTypes: CustomType[];
} {
  const nodes: ArchNode[] = file.nodes.map((n) => ({
    id: n.id,
    type: "archNode",
    position: { x: n.position.x, y: n.position.y },
    data: {
      label: n.label,
      language: n.language,
      periodMs: n.period_ms,
      inputs: n.inputs,
      outputs: n.outputs,
      params: n.params,
    },
  }));
  const edges: Edge[] = file.edges.map((e) => ({
    id: e.id,
    source: e.source.node,
    sourceHandle: e.source.port,
    target: e.target.node,
    targetHandle: e.target.port,
  }));
  return { nodes, edges, customTypes: file.custom_types };
}
