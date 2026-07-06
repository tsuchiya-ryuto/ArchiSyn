//! スケジューリング設計（プロセス統合 + オフセット）の Docker 検証用デモ。
//! Ticker → Worker（ともに Python）を1プロセス（single executor）に統合する。
//! 使い方: cargo run --example gen_sched_demo -- <出力ディレクトリ>

use archisyn_lib::codegen::generate_workspace;
use archisyn_lib::fs::safe_write::write_files;
use archisyn_lib::model::*;

fn project() -> Project {
    let node = |id: &str, label: &str, period: u32, offset: u32| NodeDef {
        id: id.to_string(),
        label: label.to_string(),
        language: Language::Python,
        namespace: None,
        period_ms: period,
        offset_ms: offset,
        wcet_ms: Some(1.0),
        position: Vec2 { x: 0.0, y: 0.0 },
        size: None,
        inputs: vec![],
        outputs: vec![],
        params: vec![],
    };
    let mut ticker = node("n1", "Ticker", 100, 0);
    ticker.outputs = vec![PortDef {
        name: "count".to_string(),
        ty: "std_msgs/Float64".to_string(),
    }];
    let mut worker = node("n2", "Worker", 100, 30);
    worker.inputs = vec![PortDef {
        name: "count".to_string(),
        ty: "std_msgs/Float64".to_string(),
    }];

    Project {
        arcsyn_version: ARCSYN_VERSION.to_string(),
        project: ProjectMeta {
            name: "sched_demo".to_string(),
            middleware: "ros2_humble".to_string(),
        },
        custom_types: vec![],
        nodes: vec![ticker, worker],
        edges: vec![EdgeDef {
            id: "e1".to_string(),
            source: Endpoint {
                node: "n1".to_string(),
                port: "count".to_string(),
            },
            target: Endpoint {
                node: "n2".to_string(),
                port: "count".to_string(),
            },
        }],
        scheduling: SchedulingSettings {
            processes: vec![ProcessDef {
                name: "control".to_string(),
                executor: ExecutorKind::Single,
                threads: None,
                priority: None, // Docker では CAP_SYS_NICE が無いため付けない
                cpu_affinity: vec![],
                nodes: vec!["n1".to_string(), "n2".to_string()],
            }],
        },
        launch: LaunchSettings::default(),
        viewport: Viewport::default(),
    }
}

fn main() {
    let out = std::env::args()
        .nth(1)
        .expect("使い方: cargo run --example gen_sched_demo -- <出力ディレクトリ>");
    let ws = generate_workspace(&project()).expect("生成に失敗");
    let report = write_files(std::path::Path::new(&out), &ws.files).expect("書き込みに失敗");
    println!("written: {} files", report.written.len());
    for w in &ws.warnings {
        eprintln!("warning: {w}");
    }
}
