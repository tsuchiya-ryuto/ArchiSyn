use serde::{Deserialize, Serialize};

use super::{CustomType, EdgeDef, LaunchSettings, NodeDef};

/// 現在サポートする .arcsyn フォーマットのバージョン
pub const ARCSYN_VERSION: &str = "0.1";

/// .arcsyn ファイル全体（GUI 完全復元に必要な全状態を含む）
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Project {
    pub arcsyn_version: String,
    pub project: ProjectMeta,
    #[serde(default)]
    pub custom_types: Vec<CustomType>,
    #[serde(default)]
    pub nodes: Vec<NodeDef>,
    #[serde(default)]
    pub edges: Vec<EdgeDef>,
    /// launch 設計（引数・起動構成）
    #[serde(default, skip_serializing_if = "LaunchSettings::is_empty")]
    pub launch: LaunchSettings,
    #[serde(default)]
    pub viewport: Viewport,
}

impl Default for Project {
    fn default() -> Self {
        Self {
            arcsyn_version: ARCSYN_VERSION.to_string(),
            project: ProjectMeta::default(),
            custom_types: Vec::new(),
            nodes: Vec::new(),
            edges: Vec::new(),
            launch: LaunchSettings::default(),
            viewport: Viewport::default(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectMeta {
    pub name: String,
    pub middleware: String,
}

impl Default for ProjectMeta {
    fn default() -> Self {
        Self {
            name: "my_project".to_string(),
            middleware: "ros2_humble".to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Viewport {
    pub zoom: f64,
    pub pan: Vec2,
}

impl Default for Viewport {
    fn default() -> Self {
        Self {
            zoom: 1.0,
            pan: Vec2 { x: 0.0, y: 0.0 },
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct Vec2 {
    pub x: f64,
    pub y: f64,
}
