import { ask, message, open, save } from "@tauri-apps/plugin-dialog";
import { useModelStore } from "../state/store";
import { toProjectFile } from "./convert";
import { loadProject, newProject, saveProject } from "./project";

const FILE_FILTERS = [
  { name: "ArchiSyn プロジェクト", extensions: ["arcsyn"] },
];

function snapshot() {
  const s = useModelStore.getState();
  return toProjectFile({
    nodes: s.nodes,
    edges: s.edges,
    customTypes: s.customTypes,
    projectName: s.projectName,
    middleware: s.middleware,
    viewport: s.rfInstance?.getViewport() ?? { x: 0, y: 0, zoom: 1 },
  });
}

async function confirmDiscard(): Promise<boolean> {
  const s = useModelStore.getState();
  if (s.nodes.length === 0 && s.customTypes.length === 0) return true;
  return ask("現在の内容は破棄されます。続行しますか？", {
    title: "ArchiSyn",
    kind: "warning",
  });
}

async function showError(context: string, error: unknown) {
  await message(`${context}: ${String(error)}`, {
    title: "ArchiSyn",
    kind: "error",
  });
}

export async function newProjectAction() {
  const s = useModelStore.getState();
  if (!(await confirmDiscard())) return;
  try {
    const file = await newProject();
    s.applyProjectFile(file, null);
    s.setFileStatus("新規プロジェクトを作成しました");
  } catch (e) {
    await showError("新規作成に失敗しました", e);
  }
}

export async function openProjectAction() {
  const s = useModelStore.getState();
  if (!(await confirmDiscard())) return;
  const path = await open({ filters: FILE_FILTERS, multiple: false });
  if (typeof path !== "string") return;
  try {
    const file = await loadProject(path);
    s.applyProjectFile(file, path);
    s.setFileStatus(`読み込みました: ${path}`);
  } catch (e) {
    await showError("読み込みに失敗しました", e);
  }
}

export async function saveProjectAction(saveAs = false) {
  const s = useModelStore.getState();
  let path = saveAs ? null : s.currentFilePath;
  if (!path) {
    const selected = await save({
      filters: FILE_FILTERS,
      defaultPath: `${s.projectName}.arcsyn`,
    });
    if (!selected) return;
    path = selected;
  }
  try {
    await saveProject(path, snapshot());
    s.setCurrentFilePath(path);
    s.setFileStatus(`保存しました: ${path}`);
  } catch (e) {
    await showError("保存に失敗しました", e);
  }
}
