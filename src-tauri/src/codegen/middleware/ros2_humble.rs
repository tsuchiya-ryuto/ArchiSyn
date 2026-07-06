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
        let mut launch_nodes: Vec<tera::Value> = Vec::new();

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
                launch_nodes.push(tera::Value::Object(m));
            }
        }

        // Rust ノードを含む場合はビルド環境の注意を添える
        if project.nodes.iter().any(|n| n.language == Language::Rust) {
            ws.warnings.push(
                "Rust ノードのビルドには ros2_rust underlay が必要です（docker/humble-rust.Dockerfile を使用してください）"
                    .to_string(),
            );
        }

        // launch ファイル
        if !launch_nodes.is_empty() {
            ws.files.extend(self.launch_files(&launch_nodes)?);
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

    /// 全ノードを起動する launch ファイルを生成する
    /// （nodes: pkg / node_name / namespace? / params を持つ Tera オブジェクト列）
    fn launch_files(&self, nodes: &[tera::Value]) -> Result<Vec<GeneratedFile>, String> {
        let mut ctx = Context::new();
        ctx.insert("nodes", nodes);
        Ok(vec![GeneratedFile {
            rel_path: PathBuf::from("launch").join("system.launch.py"),
            content: templates()
                .render("launch/system_launch_py.tera", &ctx)
                .map_err(|e| format!("launch ファイルの生成に失敗: {e}"))?,
            protected: false,
        }])
    }
}
