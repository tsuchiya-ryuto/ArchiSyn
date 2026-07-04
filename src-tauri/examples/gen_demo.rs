//! Docker での colcon build 検証用にデモワークスペースを生成する。
//! 使い方: cargo run --example gen_demo -- <出力ディレクトリ>

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
                id: "n1".to_string(),
                label: "SensorFusion".to_string(),
                language: Language::Python,
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
                language: Language::Python,
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
        viewport: Viewport::default(),
    }
}

fn main() {
    let out = std::env::args()
        .nth(1)
        .expect("使い方: cargo run --example gen_demo -- <出力ディレクトリ>");
    let ws = generate_workspace(&demo_project()).expect("生成に失敗");
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
