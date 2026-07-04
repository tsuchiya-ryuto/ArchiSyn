use std::collections::BTreeSet;
use std::path::PathBuf;

use tera::{Context, Map, Value};

use crate::codegen::{pascal_case, snake_case, templates, GeneratedFile};
use crate::model::{Language, NodeDef, ParamDef, Project};

use super::{GenContext, LanguageGenerator};

/// rclpy 向け Python コード生成（Phase 1）
pub struct PythonGenerator;

impl LanguageGenerator for PythonGenerator {
    fn language(&self) -> Language {
        Language::Python
    }

    fn package_name(&self, project: &Project) -> String {
        format!("{}_py_nodes", snake_case(&project.project.name))
    }

    fn generate(&self, ctx: &GenContext, nodes: &[&NodeDef]) -> Result<Vec<GeneratedFile>, String> {
        let pkg = self.package_name(ctx.project);
        let root = PathBuf::from("src").join(&pkg);
        let interfaces_file = format!("{pkg}_interfaces.py");
        let mut files = Vec::new();

        // 依存パッケージと import 文（ファイル全体で重複排除・整列）
        let mut dep_pkgs: BTreeSet<String> = BTreeSet::new();
        let mut imports: BTreeSet<String> = BTreeSet::new();
        let mut node_ctxs = Vec::new();
        let mut entry_points = Vec::new();

        for node in nodes {
            let node_name = &ctx.node_names[&node.id];
            let class_base = format!("{}Base", pascal_case(&node.label));
            let mut inputs = Vec::new();
            let mut outputs = Vec::new();

            for port in &node.inputs {
                let resolved = ctx.adapter.resolve_type(ctx.project, &port.ty)?;
                dep_pkgs.insert(resolved.package.clone());
                imports.insert(format!(
                    "from {}.msg import {}",
                    resolved.package, resolved.type_name
                ));
                let mut m = Map::new();
                m.insert("name".into(), Value::String(port.name.clone()));
                m.insert("msg_class".into(), Value::String(resolved.type_name));
                m.insert(
                    "topic".into(),
                    Value::String(ctx.topics.input_topic(&node.id, node_name, &port.name)),
                );
                inputs.push(Value::Object(m));
            }
            for port in &node.outputs {
                let resolved = ctx.adapter.resolve_type(ctx.project, &port.ty)?;
                dep_pkgs.insert(resolved.package.clone());
                imports.insert(format!(
                    "from {}.msg import {}",
                    resolved.package, resolved.type_name
                ));
                let mut m = Map::new();
                m.insert("name".into(), Value::String(port.name.clone()));
                m.insert("msg_class".into(), Value::String(resolved.type_name));
                m.insert(
                    "topic".into(),
                    Value::String(ctx.topics.output_topic(node_name, &port.name)),
                );
                outputs.push(Value::Object(m));
            }

            let params: Vec<Value> = node
                .params
                .iter()
                .map(|p| {
                    let mut m = Map::new();
                    m.insert("name".into(), Value::String(p.name.clone()));
                    m.insert("py_default".into(), Value::String(python_default(p)));
                    m.insert(
                        "py_type".into(),
                        Value::String(python_type(&p.ty).to_string()),
                    );
                    Value::Object(m)
                })
                .collect();

            let mut m = Map::new();
            m.insert("label".into(), Value::String(node.label.clone()));
            m.insert("node_name".into(), Value::String(node_name.clone()));
            m.insert("class_base".into(), Value::String(class_base.clone()));
            m.insert("class_name".into(), Value::String(pascal_case(&node.label)));
            m.insert(
                "period_s".into(),
                Value::String(format_period_s(node.period_ms)),
            );
            m.insert("period_ms".into(), Value::from(node.period_ms));
            m.insert("inputs".into(), Value::Array(inputs));
            m.insert("outputs".into(), Value::Array(outputs));
            m.insert("params".into(), Value::Array(params));
            node_ctxs.push(Value::Object(m));

            let mut e = Map::new();
            e.insert("node_name".into(), Value::String(node_name.clone()));
            entry_points.push(Value::Object(e));
        }

        // interfaces（毎回再生成）
        let mut tctx = Context::new();
        tctx.insert("pkg", &pkg);
        tctx.insert("imports", &imports.iter().collect::<Vec<_>>());
        tctx.insert("nodes", &node_ctxs);
        files.push(GeneratedFile {
            rel_path: root.join("interfaces").join(&interfaces_file),
            content: templates()
                .render("python/interfaces.tera", &tctx)
                .map_err(|e| format!("interfaces の生成に失敗: {e}"))?,
            protected: false,
        });
        files.push(GeneratedFile {
            rel_path: root.join("interfaces").join("__init__.py"),
            content: String::new(),
            protected: false,
        });

        // 実装部スケルトン（保護対象: 既存なら上書きしない）
        for (node, node_ctx) in nodes.iter().zip(&node_ctxs) {
            let node_name = &ctx.node_names[&node.id];
            let mut ictx = Context::new();
            ictx.insert("pkg", &pkg);
            ictx.insert(
                "interfaces_module",
                &format!(
                    "{pkg}.interfaces.{}",
                    interfaces_file.trim_end_matches(".py")
                ),
            );
            ictx.insert("node", node_ctx);
            files.push(GeneratedFile {
                rel_path: root.join(&pkg).join(format!("{node_name}.py")),
                content: templates()
                    .render("python/node_impl.tera", &ictx)
                    .map_err(|e| format!("実装スケルトンの生成に失敗: {e}"))?,
                protected: true,
            });
        }
        files.push(GeneratedFile {
            rel_path: root.join(&pkg).join("__init__.py"),
            content: String::new(),
            protected: false,
        });

        // パッケージメタデータ
        let mut pctx = Context::new();
        pctx.insert("pkg", &pkg);
        pctx.insert("project_name", &ctx.project.project.name);
        pctx.insert("deps", &dep_pkgs.iter().collect::<Vec<_>>());
        pctx.insert("nodes", &entry_points);
        files.push(GeneratedFile {
            rel_path: root.join("package.xml"),
            content: templates()
                .render("python/package_xml.tera", &pctx)
                .map_err(|e| format!("package.xml の生成に失敗: {e}"))?,
            protected: false,
        });
        files.push(GeneratedFile {
            rel_path: root.join("setup.py"),
            content: templates()
                .render("python/setup_py.tera", &pctx)
                .map_err(|e| format!("setup.py の生成に失敗: {e}"))?,
            protected: false,
        });
        files.push(GeneratedFile {
            rel_path: root.join("setup.cfg"),
            content: templates()
                .render("python/setup_cfg.tera", &pctx)
                .map_err(|e| format!("setup.cfg の生成に失敗: {e}"))?,
            protected: false,
        });
        files.push(GeneratedFile {
            rel_path: root.join("resource").join(&pkg),
            content: String::new(),
            protected: false,
        });

        Ok(files)
    }
}

/// パラメータ型 → Python 型ヒント
fn python_type(ty: &str) -> &'static str {
    match ty {
        "bool" => "bool",
        "int64" => "int",
        "float64" => "float",
        _ => "str",
    }
}

/// 既定値（文字列保持）を Python リテラルへ変換。解釈できない場合は型の既定値。
fn python_default(param: &ParamDef) -> String {
    let raw = param.default.trim();
    match param.ty.as_str() {
        "bool" => match raw.to_ascii_lowercase().as_str() {
            "true" | "1" => "True".to_string(),
            _ => "False".to_string(),
        },
        "int64" => raw
            .parse::<i64>()
            .map(|v| v.to_string())
            .unwrap_or_else(|_| "0".to_string()),
        "float64" => raw
            .parse::<f64>()
            .map(|v| {
                if v.fract() == 0.0 {
                    format!("{v:.1}")
                } else {
                    v.to_string()
                }
            })
            .unwrap_or_else(|_| "0.0".to_string()),
        _ => format!("\"{}\"", raw.replace('\\', "\\\\").replace('"', "\\\"")),
    }
}

/// 周期 [ms] → 秒（Python の float リテラル）
fn format_period_s(period_ms: u32) -> String {
    let s = period_ms as f64 / 1000.0;
    if s.fract() == 0.0 {
        format!("{s:.1}")
    } else {
        s.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn param(ty: &str, default: &str) -> ParamDef {
        ParamDef {
            name: "p".to_string(),
            ty: ty.to_string(),
            default: default.to_string(),
        }
    }

    #[test]
    fn python_default_conversion() {
        assert_eq!(python_default(&param("float64", "0.7")), "0.7");
        assert_eq!(python_default(&param("float64", "2")), "2.0");
        assert_eq!(python_default(&param("float64", "abc")), "0.0");
        assert_eq!(python_default(&param("int64", "42")), "42");
        assert_eq!(python_default(&param("bool", "true")), "True");
        assert_eq!(python_default(&param("bool", "no")), "False");
        assert_eq!(python_default(&param("string", "hi\"x")), "\"hi\\\"x\"");
    }

    #[test]
    fn period_formatting() {
        assert_eq!(format_period_s(50), "0.05");
        assert_eq!(format_period_s(1000), "1.0");
        assert_eq!(format_period_s(100), "0.1");
    }
}
