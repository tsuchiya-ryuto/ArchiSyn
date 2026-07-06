//! コード生成エンジン。
//! MiddlewareAdapter（ミドルウェア抽象）と LanguageGenerator（言語別生成）の
//! 組み合わせで、.arcsyn プロジェクトから ROS 2 ワークスペースを生成する。

pub mod language;
pub mod middleware;

use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;
use std::sync::OnceLock;

use tera::Tera;

use crate::model::Project;

/// 生成される1ファイル。protected は実装部（既存なら上書きしない）
#[derive(Debug, Clone)]
pub struct GeneratedFile {
    pub rel_path: PathBuf,
    pub content: String,
    pub protected: bool,
}

#[derive(Debug, Default)]
pub struct GeneratedWorkspace {
    pub files: Vec<GeneratedFile>,
    pub warnings: Vec<String>,
}

/// ラベルを snake_case へ（例: "SensorFusion" → "sensor_fusion"）
pub fn snake_case(input: &str) -> String {
    let mut out = String::new();
    let mut prev_lower = false;
    for c in input.chars() {
        if c.is_alphanumeric() {
            if c.is_uppercase() {
                if prev_lower {
                    out.push('_');
                }
                out.extend(c.to_lowercase());
                prev_lower = false;
            } else {
                out.push(c);
                prev_lower = c.is_lowercase() || c.is_numeric();
            }
        } else if !out.ends_with('_') && !out.is_empty() {
            out.push('_');
            prev_lower = false;
        }
    }
    let out = out.trim_matches('_').to_string();
    if out.is_empty() {
        "node".to_string()
    } else if out.chars().next().unwrap().is_numeric() {
        format!("n{out}")
    } else {
        out
    }
}

/// ラベルを PascalCase へ（例: "sensor_fusion" → "SensorFusion"）
pub fn pascal_case(input: &str) -> String {
    snake_case(input)
        .split('_')
        .map(|w| {
            let mut cs = w.chars();
            match cs.next() {
                Some(c) => c.to_uppercase().collect::<String>() + cs.as_str(),
                None => String::new(),
            }
        })
        .collect()
}

/// ノード id → 一意な snake_case 名（ノード名・ファイル名・実行ファイル名に使う）
pub fn build_node_names(project: &Project) -> HashMap<String, String> {
    let mut used: BTreeMap<String, u32> = BTreeMap::new();
    let mut names = HashMap::new();
    for node in &project.nodes {
        let base = snake_case(&node.label);
        let count = used.entry(base.clone()).or_insert(0);
        *count += 1;
        let name = if *count == 1 {
            base
        } else {
            format!("{base}_{count}")
        };
        names.insert(node.id.clone(), name);
    }
    names
}

/// エッジからトピック名を解決する。
/// トピックは絶対パス `/<namespace>/<node_name>/<port>` に統一する
/// （namespace が異なるノード間のエッジでも配線が壊れないようにするため）。
/// 出力ポートは自ノードのトピックへ publish し、
/// 接続された入力ポートは接続元のトピックを subscribe する。
pub struct TopicMap {
    /// node_id → "/ns/node_name"（ns なしは "/node_name"）
    prefixes: HashMap<String, String>,
    inputs: HashMap<(String, String), String>,
}

impl TopicMap {
    pub fn build(project: &Project, node_names: &HashMap<String, String>) -> Self {
        let mut prefixes = HashMap::new();
        for node in &project.nodes {
            let name = &node_names[&node.id];
            let prefix = match node.namespace.as_deref().map(|ns| ns.trim_matches('/')) {
                Some(ns) if !ns.is_empty() => format!("/{ns}/{name}"),
                _ => format!("/{name}"),
            };
            prefixes.insert(node.id.clone(), prefix);
        }

        let mut inputs = HashMap::new();
        for edge in &project.edges {
            if let Some(source_prefix) = prefixes.get(&edge.source.node) {
                inputs
                    .entry((edge.target.node.clone(), edge.target.port.clone()))
                    .or_insert_with(|| format!("{source_prefix}/{}", edge.source.port));
            }
        }
        Self { prefixes, inputs }
    }

    pub fn output_topic(&self, node_id: &str, port: &str) -> String {
        format!("{}/{port}", self.prefixes[node_id])
    }

    /// 未接続の入力は自ノードのトピックにフォールバックする
    pub fn input_topic(&self, node_id: &str, port: &str) -> String {
        self.inputs
            .get(&(node_id.to_string(), port.to_string()))
            .cloned()
            .unwrap_or_else(|| format!("{}/{port}", self.prefixes[node_id]))
    }
}

/// 埋め込み Tera テンプレート（バイナリに同梱）
pub fn templates() -> &'static Tera {
    static TERA: OnceLock<Tera> = OnceLock::new();
    TERA.get_or_init(|| {
        let mut tera = Tera::default();
        tera.add_raw_templates([
            (
                "python/package_xml.tera",
                include_str!("templates/python/package_xml.tera"),
            ),
            (
                "python/setup_py.tera",
                include_str!("templates/python/setup_py.tera"),
            ),
            (
                "python/setup_cfg.tera",
                include_str!("templates/python/setup_cfg.tera"),
            ),
            (
                "python/interfaces.tera",
                include_str!("templates/python/interfaces.tera"),
            ),
            (
                "python/node_impl.tera",
                include_str!("templates/python/node_impl.tera"),
            ),
            (
                "cpp/package_xml.tera",
                include_str!("templates/cpp/package_xml.tera"),
            ),
            (
                "cpp/cmakelists.tera",
                include_str!("templates/cpp/cmakelists.tera"),
            ),
            (
                "cpp/interfaces_hpp.tera",
                include_str!("templates/cpp/interfaces_hpp.tera"),
            ),
            (
                "cpp/node_impl.tera",
                include_str!("templates/cpp/node_impl.tera"),
            ),
            (
                "rust/package_xml.tera",
                include_str!("templates/rust/package_xml.tera"),
            ),
            (
                "rust/cargo_toml.tera",
                include_str!("templates/rust/cargo_toml.tera"),
            ),
            (
                "rust/interfaces_rs.tera",
                include_str!("templates/rust/interfaces_rs.tera"),
            ),
            (
                "rust/node_impl.tera",
                include_str!("templates/rust/node_impl.tera"),
            ),
            (
                "mock/interfaces_py.tera",
                include_str!("templates/mock/interfaces_py.tera"),
            ),
            (
                "mock/node_impl.tera",
                include_str!("templates/mock/node_impl.tera"),
            ),
            (
                "mock/run_py.tera",
                include_str!("templates/mock/run_py.tera"),
            ),
            (
                "mock/msg_types.tera",
                include_str!("templates/mock/msg_types.tera"),
            ),
            (
                "msgs/package_xml.tera",
                include_str!("templates/msgs/package_xml.tera"),
            ),
            (
                "msgs/cmakelists.tera",
                include_str!("templates/msgs/cmakelists.tera"),
            ),
            (
                "launch/system_launch_py.tera",
                include_str!("templates/launch/system_launch_py.tera"),
            ),
        ])
        .expect("埋め込みテンプレートの登録に失敗");
        tera
    })
}

/// プロジェクト全体からワークスペースの全生成ファイルを組み立てる。
/// `middleware:` フィールドに対応するアダプタへ委譲する（F-7）。
pub fn generate_workspace(project: &Project) -> Result<GeneratedWorkspace, String> {
    let adapter = middleware::adapter_for(&project.project.middleware)?;
    adapter.generate(project)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snake_case_examples() {
        assert_eq!(snake_case("SensorFusion"), "sensor_fusion");
        assert_eq!(snake_case("sensor_fusion"), "sensor_fusion");
        assert_eq!(snake_case("Node1"), "node1");
        assert_eq!(snake_case("My Robot-Ctrl"), "my_robot_ctrl");
        assert_eq!(snake_case("123abc"), "n123abc");
        assert_eq!(snake_case(""), "node");
    }

    #[test]
    fn pascal_case_examples() {
        assert_eq!(pascal_case("sensor_fusion"), "SensorFusion");
        assert_eq!(pascal_case("SensorFusion"), "SensorFusion");
        assert_eq!(pascal_case("my robot"), "MyRobot");
    }
}
