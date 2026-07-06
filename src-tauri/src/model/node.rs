use serde::{Deserialize, Serialize};

use super::Vec2;

fn is_zero(v: &u32) -> bool {
    *v == 0
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NodeDef {
    pub id: String,
    pub label: String,
    pub language: Language,
    pub period_ms: u32,
    /// 位相オフセット [ms]（周期起点からのずれ。省略時 0）
    #[serde(default, skip_serializing_if = "is_zero")]
    pub offset_ms: u32,
    /// 最悪実行時間の見積り [ms]（スケジューリング解析に使用）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wcet_ms: Option<f64>,
    /// ROS 名前空間（例: "front"）。未指定はルート
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub namespace: Option<String>,
    pub position: Vec2,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size: Option<Size>,
    #[serde(default)]
    pub inputs: Vec<PortDef>,
    #[serde(default)]
    pub outputs: Vec<PortDef>,
    #[serde(default)]
    pub params: Vec<ParamDef>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    Python,
    Cpp,
    Rust,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PortDef {
    pub name: String,
    #[serde(rename = "type")]
    pub ty: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParamDef {
    pub name: String,
    #[serde(rename = "type")]
    pub ty: String,
    /// 既定値は文字列として保持し、コード生成時に型に応じて解釈する
    pub default: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Size {
    pub w: f64,
    pub h: f64,
}
