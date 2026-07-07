//! ソース静的解析インポートの動作確認。
//! 使い方: cargo run --example scan_demo -- <Python ワークスペースのディレクトリ>

use archisyn_lib::import_scan::import_python_dir;

fn main() {
    let dir = std::env::args()
        .nth(1)
        .expect("使い方: cargo run --example scan_demo -- <dir>");
    let result = import_python_dir(std::path::Path::new(&dir), "scanned").expect("解析失敗");
    println!("{}", serde_yaml::to_string(&result.project).unwrap());
    for w in &result.warnings {
        println!("warning: {w}");
    }
}
