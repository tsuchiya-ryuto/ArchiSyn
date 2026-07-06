//! グラフダンプ JSON のインポート動作確認用。
//! 使い方: cargo run --example import_demo -- <graph.json>

use archisyn_lib::codegen::generate_workspace;
use archisyn_lib::import_graph::import_graph_json;

fn main() {
    let path = std::env::args()
        .nth(1)
        .expect("使い方: cargo run --example import_demo -- <graph.json>");
    let text = std::fs::read_to_string(&path).expect("読み込み失敗");
    let result = import_graph_json(&text, "imported_demo").expect("インポート失敗");

    println!("--- 復元されたプロジェクト ---");
    println!("{}", serde_yaml::to_string(&result.project).unwrap());
    for w in &result.warnings {
        println!("warning: {w}");
    }

    // 復元プロジェクトからコード生成できることも確認
    let ws = generate_workspace(&result.project).expect("生成失敗");
    println!("--- 再生成: {} ファイル ---", ws.files.len());
}
