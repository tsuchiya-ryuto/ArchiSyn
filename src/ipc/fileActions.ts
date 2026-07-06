import { ask, message, open, save } from "@tauri-apps/plugin-dialog";
import { useModelStore } from "../state/store";
import { toProjectFile } from "./convert";
import {
  generateCode,
  importGraph,
  loadProject,
  newProject,
  saveProject,
} from "./project";

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
    launchArgs: s.launchArgs,
    launchConfigs: s.launchConfigs,
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

export async function importGraphAction() {
  const s = useModelStore.getState();
  if (!(await confirmDiscard())) return;
  const path = await open({
    filters: [{ name: "ArchiSyn グラフダンプ", extensions: ["json"] }],
    multiple: false,
    title: "tools/introspect.py が出力した JSON を選択",
  });
  if (typeof path !== "string") return;
  try {
    const report = await importGraph(path);
    s.applyProjectFile(report.project, null);
    s.setFileStatus(
      `インポートしました: ${report.project.nodes.length} ノード / ${report.project.edges.length} 接続`,
    );
    if (report.warnings.length > 0) {
      await message(report.warnings.map((w) => `⚠ ${w}`).join("\n"), {
        title: "インポート（確認事項）",
        kind: "warning",
      });
    }
  } catch (e) {
    await showError("インポートに失敗しました", e);
  }
}

export async function generateCodeAction() {
  const s = useModelStore.getState();
  if (s.nodes.length === 0) {
    await message("ノードがありません。生成するものがありません。", {
      title: "ArchiSyn",
      kind: "warning",
    });
    return;
  }
  const outDir = await open({
    directory: true,
    title: "コード生成先のディレクトリを選択",
  });
  if (typeof outDir !== "string") return;
  try {
    const report = await generateCode(outDir, snapshot());
    const lines = [
      `生成完了: ${report.written.length} ファイルを書き込みました`,
      report.skipped.length > 0
        ? `保護によりスキップ: ${report.skipped.length} ファイル（既存の実装部）`
        : null,
      ...report.warnings.map((w) => `⚠ ${w}`),
    ].filter((l): l is string => l !== null);
    await message(lines.join("\n"), { title: "コード生成", kind: "info" });
    s.setFileStatus(`コード生成: ${outDir}`);
  } catch (e) {
    await showError("コード生成に失敗しました", e);
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
