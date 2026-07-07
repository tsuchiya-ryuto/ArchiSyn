//! ROS 2 パッケージのソースコード静的解析によるインポート（Phase 5.3 段階2/3）。
//! Python（rclpy）ソースを走査し、import_graph の中間表現（DumpNode）を組み立てる。
//! 段階2（ArchiSyn 生成物のラウンドトリップ）は本解析の特殊ケースとしてカバーされる。

use std::path::Path;

use crate::import_graph::{build_project, DumpNode, DumpParam, DumpPort, ImportResult};
use crate::model::Language;

/// ディレクトリ配下の Python ソースを走査してプロジェクトを復元する
pub fn import_python_dir(root: &Path, project_name: &str) -> Result<ImportResult, String> {
    let mut files = Vec::new();
    collect_py_files(root, &mut files)?;
    if files.is_empty() {
        return Err("Python ソース（.py）が見つかりませんでした".to_string());
    }

    let mut dump_nodes = Vec::new();
    for path in &files {
        let text = std::fs::read_to_string(path)
            .map_err(|e| format!("{} の読み込みに失敗: {e}", path.display()))?;
        if let Some(node) = scan_python_source(&text, path) {
            dump_nodes.push(node);
        }
    }
    if dump_nodes.is_empty() {
        return Err(
            "rclpy ノード（create_publisher / create_subscription 等）が見つかりませんでした"
                .to_string(),
        );
    }
    build_project(dump_nodes, project_name)
}

fn collect_py_files(dir: &Path, out: &mut Vec<std::path::PathBuf>) -> Result<(), String> {
    let entries =
        std::fs::read_dir(dir).map_err(|e| format!("{} の走査に失敗: {e}", dir.display()))?;
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if path.is_dir() {
            // ビルド生成物・隠しディレクトリはスキップ
            if !matches!(name.as_ref(), "build" | "install" | "log" | "__pycache__")
                && !name.starts_with('.')
            {
                collect_py_files(&path, out)?;
            }
        } else if path.extension().is_some_and(|e| e == "py") {
            // setup.py 等のパッケージメタは除外
            if !matches!(name.as_ref(), "setup.py" | "conf.py" | "__init__.py") {
                out.push(path);
            }
        }
    }
    Ok(())
}

/// 1つの Python ソースを rclpy ノードとして解析する。ノードでなければ None。
fn scan_python_source(text: &str, path: &Path) -> Option<DumpNode> {
    let type_alias = build_type_aliases(text);

    let publishers = scan_ports(text, "create_publisher", &type_alias);
    let subscriptions = scan_ports(text, "create_subscription", &type_alias);
    if publishers.is_empty() && subscriptions.is_empty() {
        return None; // Pub/Sub を持たないファイルはノードとみなさない
    }

    let name = scan_node_name(text).unwrap_or_else(|| {
        path.file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "node".to_string())
    });

    Some(DumpNode {
        name,
        namespace: String::new(),
        publishers,
        subscriptions,
        parameters: scan_parameters(text),
        period_ms: scan_period_ms(text),
        language: Some(Language::Python),
    })
}

/// `from <pkg>.msg import <Type>[, <Type2>]` から Type → "pkg/Type" を作る
fn build_type_aliases(text: &str) -> std::collections::HashMap<String, String> {
    let mut map = std::collections::HashMap::new();
    for line in text.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("from ") {
            if let Some((module, types)) = rest.split_once(" import ") {
                let module = module.trim();
                if let Some(pkg) = module.strip_suffix(".msg") {
                    for ty in types.split(',') {
                        // "Imu as ImuMsg" にも対応（別名側をキーにする）
                        let ty = ty.trim();
                        let (orig, alias) = match ty.split_once(" as ") {
                            Some((o, a)) => (o.trim(), a.trim()),
                            None => (ty, ty),
                        };
                        if !orig.is_empty() {
                            map.insert(alias.to_string(), format!("{pkg}/{orig}"));
                        }
                    }
                }
            }
        }
    }
    map
}

/// create_publisher / create_subscription を走査してポートを抽出する。
/// 例: create_publisher(Imu, "imu", 10) / create_subscription(FusedPose, "fused", cb, 10)
fn scan_ports(
    text: &str,
    call: &str,
    aliases: &std::collections::HashMap<String, String>,
) -> Vec<DumpPort> {
    let mut ports = Vec::new();
    let joined = strip_comments(text);
    let bytes = joined.as_bytes();
    let mut search_from = 0;
    while let Some(pos) = joined[search_from..].find(call) {
        let start = search_from + pos + call.len();
        search_from = start;
        // 直後が '(' でなければ別トークン
        let after = joined[start..].trim_start();
        if !after.starts_with('(') {
            continue;
        }
        let args = match extract_args(&joined, start, bytes) {
            Some(a) => a,
            None => continue,
        };
        let (Some(type_arg), Some(topic)) = (nth_arg(&args, 0), string_arg(&args, 1)) else {
            continue;
        };
        let ty = aliases
            .get(type_arg.trim())
            .cloned()
            .unwrap_or_else(|| type_arg.trim().to_string());
        ports.push(DumpPort { topic, ty });
    }
    ports
}

/// create_timer(period_s, ...) の秒 → ms
fn scan_period_ms(text: &str) -> Option<u32> {
    let joined = strip_comments(text);
    let pos = joined.find("create_timer")?;
    let start = pos + "create_timer".len();
    let args = extract_args(&joined, start, joined.as_bytes())?;
    let first = nth_arg(&args, 0)?;
    let secs: f64 = first.trim().parse().ok()?;
    Some((secs * 1000.0).round() as u32)
}

/// super().__init__("name") / create_node(..., "name") からノード名
fn scan_node_name(text: &str) -> Option<String> {
    let joined = strip_comments(text);
    for marker in ["super().__init__(", "create_node("] {
        if let Some(pos) = joined.find(marker) {
            let start = pos + marker.len() - 1; // '(' を含める
            if let Some(args) = extract_args(&joined, start, joined.as_bytes()) {
                // create_node は第1引数が context の場合があるので最初の文字列を採る
                for i in 0..arg_count(&args) {
                    if let Some(s) = string_arg(&args, i) {
                        return Some(s);
                    }
                }
            }
        }
    }
    None
}

/// declare_parameter("name", default) を走査
fn scan_parameters(text: &str) -> Vec<DumpParam> {
    let joined = strip_comments(text);
    let mut params = Vec::new();
    let mut from = 0;
    while let Some(pos) = joined[from..].find("declare_parameter") {
        let start = from + pos + "declare_parameter".len();
        from = start;
        let Some(args) = extract_args(&joined, start, joined.as_bytes()) else {
            continue;
        };
        let Some(name) = string_arg(&args, 0) else {
            continue;
        };
        let (ty, value) = match nth_arg(&args, 1) {
            Some(raw) => param_type_and_value(raw.trim()),
            None => (
                "string".to_string(),
                serde_json::Value::String(String::new()),
            ),
        };
        params.push(DumpParam { name, ty, value });
    }
    params
}

fn param_type_and_value(raw: &str) -> (String, serde_json::Value) {
    if raw == "True" || raw == "False" {
        return ("bool".to_string(), serde_json::Value::Bool(raw == "True"));
    }
    if let Ok(i) = raw.parse::<i64>() {
        return ("int64".to_string(), serde_json::Value::from(i));
    }
    if let Ok(f) = raw.parse::<f64>() {
        return ("float64".to_string(), serde_json::Value::from(f));
    }
    let s = raw.trim_matches(|c| c == '"' || c == '\'');
    (
        "string".to_string(),
        serde_json::Value::String(s.to_string()),
    )
}

// --- 低レベルヘルパ ---

/// 行コメント（# 以降）を除去する（文字列内 # は雑だが解析用途では許容）
fn strip_comments(text: &str) -> String {
    text.lines()
        .map(|line| match line.find('#') {
            Some(i) if !in_string_before(line, i) => &line[..i],
            _ => line,
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn in_string_before(line: &str, idx: usize) -> bool {
    let mut qd = false;
    let mut qs = false;
    for (i, c) in line.char_indices() {
        if i >= idx {
            break;
        }
        match c {
            '"' if !qs => qd = !qd,
            '\'' if !qd => qs = !qs,
            _ => {}
        }
    }
    qd || qs
}

/// `start` 直後の '(' から対応する ')' までの中身を返す（ネスト対応）
fn extract_args(text: &str, start: usize, _bytes: &[u8]) -> Option<String> {
    let open = text[start..].find('(')? + start;
    let mut depth = 0;
    let mut end = None;
    for (i, c) in text[open..].char_indices() {
        match c {
            '(' | '[' | '{' => depth += 1,
            ')' | ']' | '}' => {
                depth -= 1;
                if depth == 0 {
                    end = Some(open + i);
                    break;
                }
            }
            _ => {}
        }
    }
    Some(text[open + 1..end?].to_string())
}

/// トップレベルのカンマで引数分割（ネスト内のカンマは無視）
fn split_args(args: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut depth = 0;
    let mut cur = String::new();
    let mut qd = false;
    let mut qs = false;
    for c in args.chars() {
        match c {
            '"' if !qs => {
                qd = !qd;
                cur.push(c);
            }
            '\'' if !qd => {
                qs = !qs;
                cur.push(c);
            }
            '(' | '[' | '{' if !qd && !qs => {
                depth += 1;
                cur.push(c);
            }
            ')' | ']' | '}' if !qd && !qs => {
                depth -= 1;
                cur.push(c);
            }
            ',' if depth == 0 && !qd && !qs => {
                out.push(cur.trim().to_string());
                cur.clear();
            }
            _ => cur.push(c),
        }
    }
    if !cur.trim().is_empty() {
        out.push(cur.trim().to_string());
    }
    out
}

fn arg_count(args: &str) -> usize {
    split_args(args).len()
}

fn nth_arg(args: &str, n: usize) -> Option<String> {
    split_args(args).into_iter().nth(n)
}

/// n 番目の引数が文字列リテラルなら中身を返す
fn string_arg(args: &str, n: usize) -> Option<String> {
    let a = nth_arg(args, n)?;
    let a = a.trim();
    if (a.starts_with('"') && a.ends_with('"') && a.len() >= 2)
        || (a.starts_with('\'') && a.ends_with('\'') && a.len() >= 2)
    {
        Some(a[1..a.len() - 1].to_string())
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SRC: &str = r#"
import rclpy
from rclpy.node import Node
from sensor_msgs.msg import Imu
from geometry_msgs.msg import Twist as TwistMsg


class Controller(Node):
    def __init__(self):
        super().__init__("controller")  # ノード名
        self.declare_parameter("gain", 0.5)
        self.declare_parameter("enabled", True)
        self._sub = self.create_subscription(Imu, "imu", self._cb, 10)
        self._pub = self.create_publisher(TwistMsg, "cmd_vel", 10)
        self.create_timer(0.05, self.on_update)
"#;

    #[test]
    fn scans_node_from_source() {
        let node = scan_python_source(SRC, Path::new("controller.py")).unwrap();
        assert_eq!(node.name, "controller");
        assert_eq!(node.period_ms, Some(50));
        assert_eq!(node.subscriptions.len(), 1);
        assert_eq!(node.subscriptions[0].topic, "imu");
        assert_eq!(node.subscriptions[0].ty, "sensor_msgs/Imu");
        // 別名 import の解決
        assert_eq!(node.publishers[0].ty, "geometry_msgs/Twist");
        assert_eq!(node.publishers[0].topic, "cmd_vel");
        // パラメータ型の推定
        let gain = node.parameters.iter().find(|p| p.name == "gain").unwrap();
        assert_eq!(gain.ty, "float64");
        let enabled = node
            .parameters
            .iter()
            .find(|p| p.name == "enabled")
            .unwrap();
        assert_eq!(enabled.ty, "bool");
    }

    #[test]
    fn non_node_file_is_ignored() {
        assert!(scan_python_source("x = 1\n", Path::new("util.py")).is_none());
    }

    #[test]
    fn builds_project_with_edges_from_sources() {
        let a = scan_python_source(
            "from std_msgs.msg import Float64\nclass A:\n def f(self):\n  super().__init__(\"a\")\n  self.create_publisher(Float64, \"topic\", 10)\n",
            Path::new("a.py"),
        )
        .unwrap();
        let b = scan_python_source(
            "from std_msgs.msg import Float64\nclass B:\n def f(self):\n  super().__init__(\"b\")\n  self.create_subscription(Float64, \"topic\", cb, 10)\n",
            Path::new("b.py"),
        )
        .unwrap();
        let result = build_project(vec![a, b], "demo").unwrap();
        assert_eq!(result.project.nodes.len(), 2);
        assert_eq!(result.project.edges.len(), 1);
    }
}
