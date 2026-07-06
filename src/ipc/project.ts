import { invoke } from "@tauri-apps/api/core";
import type { CustomType, Language } from "../types/arcsyn";

// Rust 側 model::Project と対応する .arcsyn ファイル表現（キーは snake_case）

export type FilePort = { name: string; type: string };

export type FileParam = { name: string; type: string; default: string };

export type FileNode = {
  id: string;
  label: string;
  language: Language;
  namespace?: string;
  period_ms: number;
  position: { x: number; y: number };
  size?: { w: number; h: number };
  inputs: FilePort[];
  outputs: FilePort[];
  params: FileParam[];
};

export type FileEdge = {
  id: string;
  source: { node: string; port: string };
  target: { node: string; port: string };
};

export type ProjectFile = {
  arcsyn_version: string;
  project: { name: string; middleware: string };
  custom_types: CustomType[];
  nodes: FileNode[];
  edges: FileEdge[];
  viewport: { zoom: number; pan: { x: number; y: number } };
};

export function newProject(): Promise<ProjectFile> {
  return invoke("new_project");
}

export function saveProject(path: string, project: ProjectFile): Promise<void> {
  return invoke("save_project", { path, project });
}

export function loadProject(path: string): Promise<ProjectFile> {
  return invoke("load_project", { path });
}

export type MiddlewareInfo = {
  name: string;
  description: string;
};

export function listMiddlewares(): Promise<MiddlewareInfo[]> {
  return invoke("list_middlewares");
}

export type GenerateReport = {
  written: string[];
  skipped: string[];
  warnings: string[];
};

export function generateCode(
  outDir: string,
  project: ProjectFile,
): Promise<GenerateReport> {
  return invoke("generate_code", { outDir, project });
}
