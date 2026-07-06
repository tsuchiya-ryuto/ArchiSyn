//! Docker での colcon build 検証用にデモワークスペースを生成する。
//! ImuDriver (Rust) → SensorFusion (Python) → Controller (C++) の3言語混在構成。
//! 使い方: cargo run --example gen_demo -- <出力ディレクトリ> [middleware]
//!   middleware: ros2_humble（既定） | mock_pubsub

use archisyn_lib::codegen::generate_workspace;
use archisyn_lib::fs::safe_write::write_files;
use archisyn_lib::model::*;

fn demo_project() -> Project {
    Project {
        arcsyn_version: ARCSYN_VERSION.to_string(),
        project: ProjectMeta {
            name: "demo_robot".to_string(),
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
        nodes: vec![
            NodeDef {
                id: "n0".to_string(),
                label: "ImuDriver".to_string(),
                language: Language::Rust,
                namespace: None,
                period_ms: 20,
                position: Vec2 { x: -300.0, y: 0.0 },
                size: None,
                inputs: vec![],
                outputs: vec![PortDef {
                    name: "imu".to_string(),
                    ty: "sensor_msgs/Imu".to_string(),
                }],
                params: vec![],
            },
            NodeDef {
                id: "n1".to_string(),
                label: "SensorFusion".to_string(),
                language: Language::Python,
                namespace: None,
                period_ms: 50,
                position: Vec2 { x: 0.0, y: 0.0 },
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
            },
            NodeDef {
                id: "n2".to_string(),
                label: "Controller".to_string(),
                language: Language::Cpp,
                namespace: None,
                period_ms: 100,
                position: Vec2 { x: 300.0, y: 0.0 },
                size: None,
                inputs: vec![PortDef {
                    name: "pose".to_string(),
                    ty: "FusedPose".to_string(),
                }],
                outputs: vec![PortDef {
                    name: "cmd".to_string(),
                    ty: "geometry_msgs/Twist".to_string(),
                }],
                params: vec![],
            },
        ],
        edges: vec![
            EdgeDef {
                id: "e0".to_string(),
                source: Endpoint {
                    node: "n0".to_string(),
                    port: "imu".to_string(),
                },
                target: Endpoint {
                    node: "n1".to_string(),
                    port: "imu".to_string(),
                },
            },
            EdgeDef {
                id: "e1".to_string(),
                source: Endpoint {
                    node: "n1".to_string(),
                    port: "fused".to_string(),
                },
                target: Endpoint {
                    node: "n2".to_string(),
                    port: "pose".to_string(),
                },
            },
        ],
        launch: LaunchSettings {
            args: vec![LaunchArgDef {
                name: "use_sim_time".to_string(),
                default: "false".to_string(),
            }],
            configs: vec![LaunchConfigDef {
                name: "sensors".to_string(),
                nodes: vec!["n0".to_string(), "n1".to_string()],
            }],
        },
        viewport: Viewport::default(),
    }
}

fn main() {
    let out = std::env::args()
        .nth(1)
        .expect("使い方: cargo run --example gen_demo -- <出力ディレクトリ> [middleware]");
    let mut project = demo_project();
    if let Some(mw) = std::env::args().nth(2) {
        project.project.middleware = mw;
    }
    let ws = generate_workspace(&project).expect("生成に失敗");
    let report = write_files(std::path::Path::new(&out), &ws.files).expect("書き込みに失敗");
    println!("written: {} files", report.written.len());
    for p in &report.written {
        println!("  {p}");
    }
    for p in &report.skipped {
        println!("  (protected, skipped) {p}");
    }
    for w in &ws.warnings {
        eprintln!("warning: {w}");
    }
}
