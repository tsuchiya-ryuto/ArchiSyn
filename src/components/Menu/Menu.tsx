import { useEffect, useState } from "react";
import {
  generateCodeAction,
  importGraphAction,
  importSourceAction,
  newProjectAction,
  openProjectAction,
  saveProjectAction,
} from "../../ipc/fileActions";
import { listMiddlewares, type MiddlewareInfo } from "../../ipc/project";
import { useModelStore } from "../../state/store";
import { TextField } from "../common/TextField";

export function Menu() {
  const projectName = useModelStore((s) => s.projectName);
  const setProjectName = useModelStore((s) => s.setProjectName);
  const middleware = useModelStore((s) => s.middleware);
  const setMiddleware = useModelStore((s) => s.setMiddleware);
  const currentFilePath = useModelStore((s) => s.currentFilePath);
  const fileStatus = useModelStore((s) => s.fileStatus);
  const [middlewares, setMiddlewares] = useState<MiddlewareInfo[]>([]);

  useEffect(() => {
    listMiddlewares().then(setMiddlewares).catch(console.error);
  }, []);

  const selectedInfo = middlewares.find((m) => m.name === middleware);

  return (
    <header className="menu-bar">
      <span className="app-title">ArchiSyn</span>
      <label className="menu-project-name">
        <span>プロジェクト名</span>
        <TextField value={projectName} onCommit={setProjectName} />
      </label>
      <label className="menu-middleware" title={selectedInfo?.description}>
        <span>ミドルウェア</span>
        <select
          value={middleware}
          onChange={(e) => setMiddleware(e.currentTarget.value)}
        >
          {middlewares.map((m) => (
            <option key={m.name} value={m.name} title={m.description}>
              {m.name}
            </option>
          ))}
          {!middlewares.some((m) => m.name === middleware) && (
            <option value={middleware}>{middleware}</option>
          )}
        </select>
      </label>
      <div className="menu-actions">
        <button onClick={() => void newProjectAction()}>新規</button>
        <button onClick={() => void openProjectAction()}>開く</button>
        <button onClick={() => void saveProjectAction()}>保存</button>
        <button onClick={() => void saveProjectAction(true)}>
          名前を付けて保存
        </button>
        <button
          onClick={() => void importGraphAction()}
          title="実行中の ROS 2 システムの解析結果（tools/introspect.py の JSON）からプロジェクトを復元"
        >
          グラフ取込
        </button>
        <button
          onClick={() => void importSourceAction()}
          title="ROS 2 パッケージの Python ソースを静的解析してプロジェクトを復元"
        >
          ソース取込
        </button>
        <button
          className="generate-button"
          onClick={() => void generateCodeAction()}
        >
          コード生成
        </button>
      </div>
      <span className="menu-status" title={currentFilePath ?? undefined}>
        {fileStatus ?? currentFilePath ?? "未保存"}
      </span>
    </header>
  );
}
