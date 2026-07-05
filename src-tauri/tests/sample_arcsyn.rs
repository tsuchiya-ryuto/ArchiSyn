//! examples/ 配下のサンプル .arcsyn が常に有効であることを保証する回帰テスト。
//! （読込できること・警告なしでコード生成できること）

use archisyn_lib::codegen::generate_workspace;
use archisyn_lib::commands::load_project;

fn sample_path() -> String {
    format!(
        "{}/../examples/demo_robot.arcsyn",
        env!("CARGO_MANIFEST_DIR")
    )
}

#[test]
fn sample_arcsyn_loads_and_generates() {
    let project = load_project(sample_path()).expect("サンプル .arcsyn の読込に失敗");
    assert_eq!(project.project.name, "demo_robot");
    assert_eq!(project.nodes.len(), 3);
    assert_eq!(project.edges.len(), 2);

    let ws = generate_workspace(&project).expect("サンプルからのコード生成に失敗");
    assert!(
        ws.warnings.is_empty(),
        "サンプルは警告なしで生成できるべき: {:?}",
        ws.warnings
    );
    // 3ノード分の実装部（保護対象）が含まれる
    let protected: Vec<_> = ws
        .files
        .iter()
        .filter(|f| f.protected)
        .map(|f| f.rel_path.to_string_lossy().to_string())
        .collect();
    assert_eq!(protected.len(), 3, "{protected:?}");
}
