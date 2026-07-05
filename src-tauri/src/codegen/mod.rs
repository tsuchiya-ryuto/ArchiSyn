//! コード生成エンジン。
//! MiddlewareAdapter（ミドルウェア抽象）と LanguageGenerator（言語別生成）の
//! 組み合わせで、.arcsyn プロジェクトから ROS 2 ワークスペースを生成する。

pub mod language;
pub mod middleware;

use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;
use std::sync::OnceLock;

use tera::Tera;

use crate::model::{Language, Project};
use language::{cpp::CppGenerator, python::PythonGenerator, GenContext, LanguageGenerator};
use middleware::{ros2_humble::Ros2HumbleAdapter, MiddlewareAdapter};

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
/// 出力ポートは `<node_name>/<port>` で publish し、
/// 接続された入力ポートは接続元のトピックを subscribe する。
pub struct TopicMap {
    inputs: HashMap<(String, String), String>,
}

impl TopicMap {
    pub fn build(project: &Project, node_names: &HashMap<String, String>) -> Self {
        let mut inputs = HashMap::new();
        for edge in &project.edges {
            if let Some(source_name) = node_names.get(&edge.source.node) {
                inputs
                    .entry((edge.target.node.clone(), edge.target.port.clone()))
                    .or_insert_with(|| format!("{source_name}/{}", edge.source.port));
            }
        }
        Self { inputs }
    }

    pub fn output_topic(&self, node_name: &str, port: &str) -> String {
        format!("{node_name}/{port}")
    }

    /// 未接続の入力は自ノード名のトピックにフォールバックする
    pub fn input_topic(&self, node_id: &str, node_name: &str, port: &str) -> String {
        self.inputs
            .get(&(node_id.to_string(), port.to_string()))
            .cloned()
            .unwrap_or_else(|| format!("{node_name}/{port}"))
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

/// プロジェクト全体からワークスペースの全生成ファイルを組み立てる
pub fn generate_workspace(project: &Project) -> Result<GeneratedWorkspace, String> {
    let adapter = Ros2HumbleAdapter;
    let node_names = build_node_names(project);
    let topics = TopicMap::build(project, &node_names);
    let ctx = GenContext {
        project,
        adapter: &adapter,
        node_names: &node_names,
        topics: &topics,
    };

    let mut ws = GeneratedWorkspace::default();

    // カスタム型 → 共通 msgs パッケージ
    ws.files.extend(adapter.msgs_package(project)?);

    // 言語別パッケージ（Phase 1: Python / Phase 2: C++）
    let generators: Vec<Box<dyn LanguageGenerator>> =
        vec![Box::new(PythonGenerator), Box::new(CppGenerator)];
    let mut launch_nodes: Vec<(String, String)> = Vec::new(); // (package, node_name)

    for generator in &generators {
        let nodes: Vec<_> = project
            .nodes
            .iter()
            .filter(|n| n.language == generator.language())
            .collect();
        if nodes.is_empty() {
            continue;
        }
        ws.files.extend(generator.generate(&ctx, &nodes)?);
        let pkg = generator.package_name(project);
        for node in &nodes {
            launch_nodes.push((pkg.clone(), node_names[&node.id].clone()));
        }
    }

    // 未対応言語のノードは警告してスキップ
    for node in &project.nodes {
        if node.language == Language::Rust {
            ws.warnings.push(format!(
                "ノード「{}」の言語 Rust は未対応のためスキップしました（Phase 2 後半で対応予定）",
                node.label
            ));
        }
    }

    // launch ファイル
    if !launch_nodes.is_empty() {
        ws.files.extend(adapter.launch_files(&launch_nodes)?);
    }

    Ok(ws)
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
