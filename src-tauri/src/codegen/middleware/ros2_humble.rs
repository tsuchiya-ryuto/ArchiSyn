use std::collections::BTreeSet;
use std::path::PathBuf;

use tera::Context;

use crate::codegen::{snake_case, templates, GeneratedFile};
use crate::model::Project;

use super::{MiddlewareAdapter, ResolvedType};

/// ROS 2 Humble 向けアダプタ（Phase 1）
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

    fn launch_files(&self, nodes: &[(String, String)]) -> Result<Vec<GeneratedFile>, String> {
        let entries: Vec<_> = nodes
            .iter()
            .map(|(pkg, name)| {
                let mut m = tera::Map::new();
                m.insert("pkg".into(), tera::Value::String(pkg.clone()));
                m.insert("node_name".into(), tera::Value::String(name.clone()));
                tera::Value::Object(m)
            })
            .collect();
        let mut ctx = Context::new();
        ctx.insert("nodes", &entries);
        Ok(vec![GeneratedFile {
            rel_path: PathBuf::from("launch").join("system.launch.py"),
            content: templates()
                .render("launch/system_launch_py.tera", &ctx)
                .map_err(|e| format!("launch ファイルの生成に失敗: {e}"))?,
            protected: false,
        }])
    }
}
