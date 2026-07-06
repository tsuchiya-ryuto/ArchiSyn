use serde::{Deserialize, Serialize};

/// launch 設計（Phase 5.2）。未指定なら全ノードの system.launch.py のみ生成される。
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct LaunchSettings {
    /// launch 引数。宣言され、全ノードに同名パラメータとして渡される
    /// （例: use_sim_time）
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub args: Vec<LaunchArgDef>,
    /// 起動構成（サブシステム）。構成ごとに launch/<name>.launch.py が生成される
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub configs: Vec<LaunchConfigDef>,
}

impl LaunchSettings {
    pub fn is_empty(&self) -> bool {
        self.args.is_empty() && self.configs.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LaunchArgDef {
    pub name: String,
    pub default: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LaunchConfigDef {
    pub name: String,
    /// 含めるノードの id
    pub nodes: Vec<String>,
}
