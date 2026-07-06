use serde::Serialize;

use crate::model::{Project, ARCSYN_VERSION};

/// コード生成の結果レポート（フロントで表示する）
#[derive(Debug, Serialize)]
pub struct GenerateReport {
    pub written: Vec<String>,
    pub skipped: Vec<String>,
    pub warnings: Vec<String>,
}

#[tauri::command]
pub fn new_project() -> Project {
    Project::default()
}

#[tauri::command]
pub fn save_project(path: String, project: Project) -> Result<(), String> {
    let yaml =
        serde_yaml::to_string(&project).map_err(|e| format!("シリアライズに失敗しました: {e}"))?;
    std::fs::write(&path, yaml).map_err(|e| format!("ファイルの書き込みに失敗しました: {e}"))
}

#[tauri::command]
pub fn load_project(path: String) -> Result<Project, String> {
    let text = std::fs::read_to_string(&path)
        .map_err(|e| format!("ファイルの読み込みに失敗しました: {e}"))?;
    let project: Project =
        serde_yaml::from_str(&text).map_err(|e| format!(".arcsyn の解析に失敗しました: {e}"))?;
    if project.arcsyn_version != ARCSYN_VERSION {
        return Err(format!(
            "未対応の arcsyn_version です: {}（対応: {}）",
            project.arcsyn_version, ARCSYN_VERSION
        ));
    }
    Ok(project)
}

/// GUI のミドルウェアセレクタに表示する情報
#[derive(Debug, Serialize)]
pub struct MiddlewareInfo {
    pub name: String,
    pub description: String,
}

#[tauri::command]
pub fn list_middlewares() -> Vec<MiddlewareInfo> {
    crate::codegen::middleware::adapters()
        .iter()
        .map(|a| MiddlewareInfo {
            name: a.name().to_string(),
            description: a.description().to_string(),
        })
        .collect()
}

#[tauri::command]
pub fn generate_code(out_dir: String, project: Project) -> Result<GenerateReport, String> {
    let workspace = crate::codegen::generate_workspace(&project)?;
    let report =
        crate::fs::safe_write::write_files(std::path::Path::new(&out_dir), &workspace.files)?;
    Ok(GenerateReport {
        written: report.written,
        skipped: report.skipped,
        warnings: workspace.warnings,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::*;

    fn sample_project() -> Project {
        Project {
            arcsyn_version: ARCSYN_VERSION.to_string(),
            project: ProjectMeta {
                name: "my_robot".to_string(),
                middleware: "ros2_humble".to_string(),
            },
            custom_types: vec![CustomType {
                name: "FusedPose".to_string(),
                fields: vec![
                    TypeField {
                        name: "position".to_string(),
                        ty: "geometry_msgs/Vector3".to_string(),
                    },
                    TypeField {
                        name: "confidence".to_string(),
                        ty: "float64".to_string(),
                    },
                ],
            }],
            nodes: vec![NodeDef {
                id: "n1".to_string(),
                label: "SensorFusion".to_string(),
                language: Language::Python,
                namespace: None,
                period_ms: 50,
                position: Vec2 { x: 120.0, y: 200.0 },
                size: None,
                inputs: vec![PortDef {
                    name: "imu".to_string(),
                    ty: "sensor_msgs/Imu".to_string(),
                }],
                outputs: vec![PortDef {
                    name: "fused".to_string(),
                    ty: "FusedPose".to_string(),
                }],
                params: vec![ParamDef {
                    name: "alpha".to_string(),
                    ty: "float64".to_string(),
                    default: "0.7".to_string(),
                }],
            }],
            edges: vec![EdgeDef {
                id: "e1".to_string(),
                source: Endpoint {
                    node: "n1".to_string(),
                    port: "fused".to_string(),
                },
                target: Endpoint {
                    node: "n2".to_string(),
                    port: "pose".to_string(),
                },
            }],
            viewport: Viewport {
                zoom: 1.5,
                pan: Vec2 { x: -30.0, y: 12.0 },
            },
        }
    }

    #[test]
    fn yaml_roundtrip_preserves_project() {
        let original = sample_project();
        let yaml = serde_yaml::to_string(&original).unwrap();
        let restored: Project = serde_yaml::from_str(&yaml).unwrap();
        assert_eq!(original, restored);
    }

    #[test]
    fn save_then_load_roundtrip() {
        let dir = std::env::temp_dir().join("arcsyn_test");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("roundtrip.arcsyn");
        let path_str = path.to_string_lossy().to_string();

        let original = sample_project();
        save_project(path_str.clone(), original.clone()).unwrap();
        let loaded = load_project(path_str).unwrap();
        assert_eq!(original, loaded);

        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn load_rejects_unsupported_version() {
        let dir = std::env::temp_dir().join("arcsyn_test");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("bad_version.arcsyn");
        let path_str = path.to_string_lossy().to_string();

        let mut project = sample_project();
        project.arcsyn_version = "99.9".to_string();
        let yaml = serde_yaml::to_string(&project).unwrap();
        std::fs::write(&path, yaml).unwrap();

        let result = load_project(path_str);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("未対応の arcsyn_version"));

        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn yaml_uses_spec_field_names() {
        let yaml = serde_yaml::to_string(&sample_project()).unwrap();
        // plan.md §4 のスキーマとキー名が一致していること
        assert!(yaml.contains("arcsyn_version:"));
        assert!(yaml.contains("period_ms: 50"));
        assert!(yaml.contains("type: sensor_msgs/Imu"));
        assert!(yaml.contains("language: python"));
    }
}
