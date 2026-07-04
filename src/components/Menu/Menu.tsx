import {
  newProjectAction,
  openProjectAction,
  saveProjectAction,
} from "../../ipc/fileActions";
import { useModelStore } from "../../state/store";
import { TextField } from "../common/TextField";

export function Menu() {
  const projectName = useModelStore((s) => s.projectName);
  const setProjectName = useModelStore((s) => s.setProjectName);
  const currentFilePath = useModelStore((s) => s.currentFilePath);
  const fileStatus = useModelStore((s) => s.fileStatus);

  return (
    <header className="menu-bar">
      <span className="app-title">ArchiSyn</span>
      <label className="menu-project-name">
        <span>プロジェクト名</span>
        <TextField value={projectName} onCommit={setProjectName} />
      </label>
      <div className="menu-actions">
        <button onClick={() => void newProjectAction()}>新規</button>
        <button onClick={() => void openProjectAction()}>開く</button>
        <button onClick={() => void saveProjectAction()}>保存</button>
        <button onClick={() => void saveProjectAction(true)}>
          名前を付けて保存
        </button>
      </div>
      <span className="menu-status" title={currentFilePath ?? undefined}>
        {fileStatus ?? currentFilePath ?? "未保存"}
      </span>
    </header>
  );
}
