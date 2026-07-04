import { useState } from "react";
import "@xyflow/react/dist/style.css";
import "./App.css";
import { Canvas } from "./components/Canvas/Canvas";
import { Menu } from "./components/Menu/Menu";
import { NodeInspector } from "./components/NodeInspector/NodeInspector";
import { TypeEditor } from "./components/TypeEditor/TypeEditor";

type SidebarTab = "node" | "types";

function App() {
  const [tab, setTab] = useState<SidebarTab>("node");

  return (
    <div className="app-frame">
      <Menu />
      <div className="app">
        <Canvas />
        <aside className="sidebar">
          <div className="sidebar-tabs">
            <button
              className={tab === "node" ? "active" : ""}
              onClick={() => setTab("node")}
            >
              ノード
            </button>
            <button
              className={tab === "types" ? "active" : ""}
              onClick={() => setTab("types")}
            >
              型
            </button>
          </div>
          <div className="sidebar-body">
            {tab === "node" ? <NodeInspector /> : <TypeEditor />}
          </div>
        </aside>
      </div>
    </div>
  );
}

export default App;
