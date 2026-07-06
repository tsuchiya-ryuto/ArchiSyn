use std::collections::BTreeSet;
use std::path::PathBuf;

use tera::Context;

use crate::codegen::language::python::python_default;
use crate::codegen::language::{
    cpp::CppGenerator, python::PythonGenerator, rust::RustGenerator, GenContext, LanguageGenerator,
};
use crate::codegen::{
    build_node_names, snake_case, templates, GeneratedFile, GeneratedWorkspace, TopicMap,
};
use crate::model::{Language, Project};

use super::{MiddlewareAdapter, ResolvedType, RosTypeResolver};

/// ROS 2 Humble 向けアダプタ（Phase 1〜2）
pub struct Ros2HumbleAdapter;

/// .msg のプリミティブ型（パッケージ解決が不要なもの）
const PRIMITIVES: &[&str] = &[
    "bool", "byte", "char", "int8", "uint8", "int16", "uint16", "int32", "uint32", "int64",
    "uint64", "float32", "float64", "string", "wstring",
];

impl MiddlewareAdapter for Ros2HumbleAdapter {
    fn name(&self) -> &'static str {
        "ros2_humble"
    }

    fn description(&self) -> &'static str {
        "ROS 2 Humble（Python / C++ / Rust ノードの colcon ワークスペースを生成）"
    }

    fn generate(&self, project: &Project) -> Result<GeneratedWorkspace, String> {
        let node_names = build_node_names(project);
        let topics = TopicMap::build(project, &node_names);
        let ctx = GenContext {
            project,
            adapter: self,
            node_names: &node_names,
            topics: &topics,
        };

        let mut ws = GeneratedWorkspace::default();

        // カスタム型 → 共通 msgs パッケージ
        ws.files.extend(self.msgs_package(project)?);

        // 言語別パッケージ（ノード単位の言語切替 F-6）
        let generators: Vec<Box<dyn LanguageGenerator>> = vec![
            Box::new(PythonGenerator),
            Box::new(CppGenerator),
            Box::new(RustGenerator),
        ];
        let mut launch_nodes: Vec<(String, tera::Value)> = Vec::new(); // (node_id, entry)

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
                let params: Vec<tera::Value> = node
                    .params
                    .iter()
                    .map(|p| {
                        let mut m = tera::Map::new();
                        m.insert("name".into(), tera::Value::String(p.name.clone()));
                        // launch は Python なので Python リテラルをそのまま使う
                        m.insert("value".into(), tera::Value::String(python_default(p)));
                        tera::Value::Object(m)
                    })
                    .collect();
                let mut m = tera::Map::new();
                m.insert("pkg".into(), tera::Value::String(pkg.clone()));
                m.insert(
                    "node_name".into(),
                    tera::Value::String(node_names[&node.id].clone()),
                );
                if let Some(ns) = node.namespace.as_deref().map(|s| s.trim_matches('/')) {
                    if !ns.is_empty() {
                        m.insert("namespace".into(), tera::Value::String(ns.to_string()));
                    }
                }
                m.insert("params".into(), tera::Value::Array(params));
                launch_nodes.push((node.id.clone(), tera::Value::Object(m)));
            }
        }

        // Rust ノードを含む場合はビルド環境の注意を添える
        if project.nodes.iter().any(|n| n.language == Language::Rust) {
            ws.warnings.push(
                "Rust ノードのビルドには ros2_rust underlay が必要です（docker/humble-rust.Dockerfile を使用してください）"
                    .to_string(),
            );
        }

        // スケジューリング設計の反映（プロセス統合 + RT prefix）
        self.apply_scheduling(project, &node_names, &mut launch_nodes, &mut ws.warnings);

        // launch ファイル（system = 全ノード + 起動構成ごとの launch）
        if !launch_nodes.is_empty() {
            let args: Vec<tera::Value> = project
                .launch
                .args
                .iter()
                .map(|a| {
                    let mut m = tera::Map::new();
                    m.insert("name".into(), tera::Value::String(a.name.clone()));
                    m.insert("default".into(), tera::Value::String(a.default.clone()));
                    tera::Value::Object(m)
                })
                .collect();

            let all: Vec<tera::Value> = launch_nodes.iter().map(|(_, v)| v.clone()).collect();
            ws.files.push(self.launch_file("system", &all, &args)?);

            for config in &project.launch.configs {
                let name = snake_case(&config.name);
                if name == "system" {
                    ws.warnings
                        .push("起動構成名 system は予約されているためスキップしました".to_string());
                    continue;
                }
                for id in &config.nodes {
                    if !launch_nodes.iter().any(|(node_id, _)| node_id == id) {
                        ws.warnings.push(format!(
                            "起動構成「{}」のノード {id} が見つかりません（未生成の言語の可能性）",
                            config.name
                        ));
                    }
                }
                let selected: Vec<tera::Value> = launch_nodes
                    .iter()
                    .filter(|(id, _)| config.nodes.contains(id))
                    .map(|(_, v)| v.clone())
                    .collect();
                if selected.is_empty() {
                    ws.warnings.push(format!(
                        "起動構成「{}」にノードが無いためスキップしました",
                        config.name
                    ));
                    continue;
                }
                ws.files.push(self.launch_file(&name, &selected, &args)?);
            }
        }

        Ok(ws)
    }
}

impl RosTypeResolver for Ros2HumbleAdapter {
    fn msgs_package_name(&self, project: &Project) -> String {
        format!("{}_msgs", snake_case(&project.project.name))
    }

    fn resolve_type(&self, project: &Project, ty: &str) -> Result<ResolvedType, String> {
        if let Some((pkg, name)) = ty.split_once('/') {
            return Ok(ResolvedType {
                package: pkg.to_string(),
                type_name: name.to_string(),
            });
        }
        if project.custom_types.iter().any(|t| t.name == ty) {
            return Ok(ResolvedType {
                package: self.msgs_package_name(project),
                type_name: ty.to_string(),
            });
        }
        Err(format!(
            "型「{ty}」を解決できません（カスタム型に未定義。pkg/Type 形式か型エディタで定義してください）"
        ))
    }
}

impl Ros2HumbleAdapter {
    /// カスタム型定義から共通メッセージパッケージ一式を生成する
    fn msgs_package(&self, project: &Project) -> Result<Vec<GeneratedFile>, String> {
        if project.custom_types.is_empty() {
            return Ok(Vec::new());
        }
        let pkg = self.msgs_package_name(project);
        let root = PathBuf::from("src").join(&pkg);
        let mut files = Vec::new();

        // 依存パッケージ（フィールドが参照する外部 msg パッケージ）
        let mut deps: BTreeSet<String> = BTreeSet::new();
        for ty in &project.custom_types {
            for field in &ty.fields {
                if let Some((dep_pkg, _)) = field.ty.split_once('/') {
                    deps.insert(dep_pkg.to_string());
                }
            }
        }

        // .msg ファイル（フィールド型は同一パッケージ内カスタム型なら裸名で参照）
        let mut msg_files = Vec::new();
        for ty in &project.custom_types {
            let mut lines = Vec::new();
            for field in &ty.fields {
                let field_ty = if PRIMITIVES.contains(&field.ty.as_str())
                    || field.ty.contains('/')
                    || project.custom_types.iter().any(|t| t.name == field.ty)
                {
                    field.ty.clone()
                } else {
                    return Err(format!(
                        "カスタム型 {} のフィールド {} の型「{}」を解決できません",
                        ty.name, field.name, field.ty
                    ));
                };
                lines.push(format!("{field_ty} {}", field.name));
            }
            let file_name = format!("{}.msg", ty.name);
            files.push(GeneratedFile {
                rel_path: root.join("msg").join(&file_name),
                content: lines.join("\n") + "\n",
                protected: false,
            });
            msg_files.push(file_name);
        }

        let mut ctx = Context::new();
        ctx.insert("pkg", &pkg);
        ctx.insert("deps", &deps.iter().collect::<Vec<_>>());
        ctx.insert("msg_files", &msg_files);

        files.push(GeneratedFile {
            rel_path: root.join("package.xml"),
            content: templates()
                .render("msgs/package_xml.tera", &ctx)
                .map_err(|e| format!("package.xml の生成に失敗: {e}"))?,
            protected: false,
        });
        files.push(GeneratedFile {
            rel_path: root.join("CMakeLists.txt"),
            content: templates()
                .render("msgs/cmakelists.tera", &ctx)
                .map_err(|e| format!("CMakeLists.txt の生成に失敗: {e}"))?,
            protected: false,
        });
        Ok(files)
    }

    /// スケジューリング設計を launch エントリへ反映する。
    /// - 全メンバーが Python の複数ノードプロセス → runner（process_<name>）1エントリに統合
    /// - それ以外のプロセス → 個別起動のまま RT prefix（chrt / taskset）を付与
    fn apply_scheduling(
        &self,
        project: &Project,
        node_names: &std::collections::HashMap<String, String>,
        launch_nodes: &mut Vec<(String, tera::Value)>,
        warnings: &mut Vec<String>,
    ) {
        let py_pkg = PythonGenerator.package_name(project);
        for proc in &project.scheduling.processes {
            let prefix = build_rt_prefix(proc);
            let members: Vec<&crate::model::NodeDef> = proc
                .nodes
                .iter()
                .filter_map(|id| project.nodes.iter().find(|n| n.id == *id))
                .collect();
            let all_python = members.iter().all(|n| n.language == Language::Python)
                && members.len() == proc.nodes.len();

            if members.len() >= 2 && all_python {
                // 統合: メンバーのエントリを取り除き runner を追加
                if members.iter().any(|n| !n.params.is_empty()) {
                    warnings.push(format!(
                        "プロセス「{}」は統合起動のため、ノード個別のパラメータはコード内の既定値を使用します",
                        proc.name
                    ));
                }
                launch_nodes.retain(|(id, _)| !proc.nodes.contains(id));
                let mut m = tera::Map::new();
                m.insert("pkg".into(), tera::Value::String(py_pkg.clone()));
                m.insert(
                    "node_name".into(),
                    tera::Value::String(format!("process_{}", snake_case(&proc.name))),
                );
                m.insert("params".into(), tera::Value::Array(Vec::new()));
                // Node(name=...) はプロセス内の全ノードをリネームしてしまうため
                // 統合 runner では name を指定しない
                m.insert("no_rename".into(), tera::Value::Bool(true));
                if let Some(pfx) = &prefix {
                    m.insert("prefix".into(), tera::Value::String(pfx.clone()));
                }
                launch_nodes.push((format!("__proc_{}", proc.name), tera::Value::Object(m)));
            } else {
                if members.len() >= 2 {
                    warnings.push(format!(
                        "プロセス「{}」は Python 以外のノードを含むため統合されません（個別起動 + prefix のみ適用）",
                        proc.name
                    ));
                }
                if let Some(pfx) = &prefix {
                    for (id, entry) in launch_nodes.iter_mut() {
                        if proc.nodes.contains(id) {
                            if let tera::Value::Object(m) = entry {
                                m.insert("prefix".into(), tera::Value::String(pfx.clone()));
                            }
                        }
                    }
                }
            }
            let _ = node_names;
        }
    }

    /// launch ファイルを1つ生成する
    /// （nodes: pkg / node_name / namespace? / params を持つ Tera オブジェクト列、
    ///   args: launch 引数。全ノードに同名パラメータとして渡される）
    fn launch_file(
        &self,
        name: &str,
        nodes: &[tera::Value],
        args: &[tera::Value],
    ) -> Result<GeneratedFile, String> {
        let mut ctx = Context::new();
        ctx.insert("nodes", nodes);
        ctx.insert("args", args);
        Ok(GeneratedFile {
            rel_path: PathBuf::from("launch").join(format!("{name}.launch.py")),
            content: templates()
                .render("launch/system_launch_py.tera", &ctx)
                .map_err(|e| format!("launch ファイルの生成に失敗: {e}"))?,
            protected: false,
        })
    }
}

/// RT 優先度 / CPU 割当から launch の prefix（chrt / taskset）を作る
fn build_rt_prefix(proc: &crate::model::ProcessDef) -> Option<String> {
    let mut parts = Vec::new();
    if let Some(priority) = proc.priority {
        parts.push(format!("chrt -f {priority}"));
    }
    if !proc.cpu_affinity.is_empty() {
        let cpus: Vec<String> = proc.cpu_affinity.iter().map(|c| c.to_string()).collect();
        parts.push(format!("taskset -c {}", cpus.join(",")));
    }
    if parts.is_empty() {
        None
    } else {
        Some(parts.join(" "))
    }
}
