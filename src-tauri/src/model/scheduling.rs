use serde::{Deserialize, Serialize};

/// スケジューリング設計（Phase 5.4）。doc/scheduling_design.md 参照。
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct SchedulingSettings {
    /// プロセス配置。未所属ノードは各自単独プロセスで実行される
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub processes: Vec<ProcessDef>,
}

impl SchedulingSettings {
    pub fn is_empty(&self) -> bool {
        self.processes.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProcessDef {
    pub name: String,
    /// executor 種別
    #[serde(default)]
    pub executor: ExecutorKind,
    /// multi の場合のスレッド数
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub threads: Option<u32>,
    /// RT 優先度（SCHED_FIFO。launch の prefix chrt に反映）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub priority: Option<i32>,
    /// CPU 割当（launch の prefix taskset に反映）
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub cpu_affinity: Vec<u32>,
    /// 含めるノード id
    #[serde(default)]
    pub nodes: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ExecutorKind {
    #[default]
    Single,
    Multi,
}
