use std::collections::BTreeSet;
use std::path::PathBuf;

use tera::{Context, Map, Value};

use crate::codegen::middleware::ResolvedType;
use crate::codegen::{pascal_case, snake_case, templates, GeneratedFile};
use crate::model::{Language, NodeDef, ParamDef, Project};

use super::{GenContext, LanguageGenerator};

/// rclcpp 向け C++ コード生成（Phase 2）。
/// Python と同様、ノードごとに完結したディレクトリ
/// （src/<node>/interfaces.hpp + <node>.cpp）を生成する。
pub struct CppGenerator;

impl LanguageGenerator for CppGenerator {
    fn language(&self) -> Language {
        Language::Cpp
    }

    fn package_name(&self, project: &Project) -> String {
        format!("{}_cpp_nodes", snake_case(&project.project.name))
    }

    fn generate(&self, ctx: &GenContext, nodes: &[&NodeDef]) -> Result<Vec<GeneratedFile>, String> {
        let pkg = self.package_name(ctx.project);
        let root = PathBuf::from("src").join(&pkg);
        let mut files = Vec::new();

        // パッケージ全体の依存（CMakeLists / package.xml 用）
        let mut dep_pkgs: BTreeSet<String> = BTreeSet::new();
        let mut node_entries = Vec::new();

        for node in nodes {
            let node_name = &ctx.node_names[&node.id];
            let node_dir = root.join("src").join(node_name);
            let class_base = format!("{}Base", pascal_case(&node.label));

            // このノードが使う #include（重複排除・整列）
            let mut includes: BTreeSet<String> = BTreeSet::new();
            let mut inputs = Vec::new();
            let mut outputs = Vec::new();

            for port in &node.inputs {
                let resolved = ctx.adapter.resolve_type(ctx.project, &port.ty)?;
                dep_pkgs.insert(resolved.package.clone());
                includes.insert(cpp_include(&resolved));
                let mut m = Map::new();
                m.insert("name".into(), Value::String(port.name.clone()));
                m.insert("cpp_type".into(), Value::String(cpp_type(&resolved)));
                m.insert(
                    "topic".into(),
                    Value::String(ctx.topics.input_topic(&node.id, &port.name)),
                );
                inputs.push(Value::Object(m));
            }
            for port in &node.outputs {
                let resolved = ctx.adapter.resolve_type(ctx.project, &port.ty)?;
                dep_pkgs.insert(resolved.package.clone());
                includes.insert(cpp_include(&resolved));
                let mut m = Map::new();
                m.insert("name".into(), Value::String(port.name.clone()));
                m.insert("cpp_type".into(), Value::String(cpp_type(&resolved)));
                m.insert(
                    "topic".into(),
                    Value::String(ctx.topics.output_topic(&node.id, &port.name)),
                );
                outputs.push(Value::Object(m));
            }

            let params: Vec<Value> = node
                .params
                .iter()
                .map(|p| {
                    let mut m = Map::new();
                    m.insert("name".into(), Value::String(p.name.clone()));
                    m.insert("cpp_default".into(), Value::String(cpp_default(p)));
                    m.insert(
                        "cpp_type".into(),
                        Value::String(cpp_param_type(&p.ty).to_string()),
                    );
                    m.insert(
                        "accessor".into(),
                        Value::String(cpp_param_accessor(&p.ty).to_string()),
                    );
                    Value::Object(m)
                })
                .collect();

            let mut node_ctx = Map::new();
            node_ctx.insert("label".into(), Value::String(node.label.clone()));
            node_ctx.insert("node_name".into(), Value::String(node_name.clone()));
            node_ctx.insert("class_base".into(), Value::String(class_base.clone()));
            node_ctx.insert("class_name".into(), Value::String(pascal_case(&node.label)));
            node_ctx.insert("period_ms".into(), Value::from(node.period_ms));
            node_ctx.insert("offset_ms".into(), Value::from(node.offset_ms));
            node_ctx.insert("inputs".into(), Value::Array(inputs));
            node_ctx.insert("outputs".into(), Value::Array(outputs));
            node_ctx.insert("params".into(), Value::Array(params));
            let node_ctx = Value::Object(node_ctx);

            // インターフェース部（毎回再生成）
            let mut tctx = Context::new();
            tctx.insert("pkg", &pkg);
            tctx.insert("includes", &includes.iter().collect::<Vec<_>>());
            tctx.insert("node", &node_ctx);
            files.push(GeneratedFile {
                rel_path: node_dir.join("interfaces.hpp"),
                content: templates()
                    .render("cpp/interfaces_hpp.tera", &tctx)
                    .map_err(|e| format!("interfaces.hpp の生成に失敗: {e}"))?,
                protected: false,
            });

            // 実装部スケルトン（保護対象: 既存なら上書きしない）
            let mut ictx = Context::new();
            ictx.insert("pkg", &pkg);
            ictx.insert("node", &node_ctx);
            files.push(GeneratedFile {
                rel_path: node_dir.join(format!("{node_name}.cpp")),
                content: templates()
                    .render("cpp/node_impl.tera", &ictx)
                    .map_err(|e| format!("実装スケルトンの生成に失敗: {e}"))?,
                protected: true,
            });

            let mut e = Map::new();
            e.insert("node_name".into(), Value::String(node_name.clone()));
            node_entries.push(Value::Object(e));
        }

        // パッケージメタデータ
        let mut pctx = Context::new();
        pctx.insert("pkg", &pkg);
        pctx.insert("project_name", &ctx.project.project.name);
        pctx.insert("deps", &dep_pkgs.iter().collect::<Vec<_>>());
        pctx.insert("nodes", &node_entries);
        files.push(GeneratedFile {
            rel_path: root.join("CMakeLists.txt"),
            content: templates()
                .render("cpp/cmakelists.tera", &pctx)
                .map_err(|e| format!("CMakeLists.txt の生成に失敗: {e}"))?,
            protected: false,
        });
        files.push(GeneratedFile {
            rel_path: root.join("package.xml"),
            content: templates()
                .render("cpp/package_xml.tera", &pctx)
                .map_err(|e| format!("package.xml の生成に失敗: {e}"))?,
            protected: false,
        });

        Ok(files)
    }
}

/// 型解決結果 → #include 行（例: sensor_msgs/Imu → <sensor_msgs/msg/imu.hpp>）
fn cpp_include(resolved: &ResolvedType) -> String {
    format!(
        "#include <{}/msg/{}.hpp>",
        resolved.package,
        snake_case(&resolved.type_name)
    )
}

/// 型解決結果 → C++ 型名（例: sensor_msgs::msg::Imu）
fn cpp_type(resolved: &ResolvedType) -> String {
    format!("{}::msg::{}", resolved.package, resolved.type_name)
}

/// パラメータ型 → C++ 型
fn cpp_param_type(ty: &str) -> &'static str {
    match ty {
        "bool" => "bool",
        "int64" => "int64_t",
        "float64" => "double",
        _ => "std::string",
    }
}

/// パラメータ型 → rclcpp::Parameter のアクセサ
fn cpp_param_accessor(ty: &str) -> &'static str {
    match ty {
        "bool" => "as_bool",
        "int64" => "as_int",
        "float64" => "as_double",
        _ => "as_string",
    }
}

/// 既定値（文字列保持）を C++ リテラルへ変換。解釈できない場合は型の既定値。
fn cpp_default(param: &ParamDef) -> String {
    let raw = param.default.trim();
    match param.ty.as_str() {
        "bool" => match raw.to_ascii_lowercase().as_str() {
            "true" | "1" => "true".to_string(),
            _ => "false".to_string(),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cpp_include_and_type_mapping() {
        let laser = ResolvedType {
            package: "sensor_msgs".to_string(),
            type_name: "LaserScan".to_string(),
        };
        assert_eq!(
            cpp_include(&laser),
            "#include <sensor_msgs/msg/laser_scan.hpp>"
        );
        assert_eq!(cpp_type(&laser), "sensor_msgs::msg::LaserScan");

        let custom = ResolvedType {
            package: "demo_robot_msgs".to_string(),
            type_name: "FusedPose".to_string(),
        };
        assert_eq!(
            cpp_include(&custom),
            "#include <demo_robot_msgs/msg/fused_pose.hpp>"
        );
    }

    #[test]
    fn cpp_default_conversion() {
        let p = |ty: &str, d: &str| ParamDef {
            name: "p".to_string(),
            ty: ty.to_string(),
            default: d.to_string(),
        };
        assert_eq!(cpp_default(&p("float64", "0.7")), "0.7");
        assert_eq!(cpp_default(&p("float64", "2")), "2.0");
        assert_eq!(cpp_default(&p("int64", "42")), "42");
        assert_eq!(cpp_default(&p("bool", "true")), "true");
        assert_eq!(cpp_default(&p("string", "hi")), "\"hi\"");
    }
}
