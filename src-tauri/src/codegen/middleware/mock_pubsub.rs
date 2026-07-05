use std::collections::BTreeMap;
use std::path::PathBuf;

use tera::{Context, Map, Value};

use crate::codegen::language::python::python_default;
use crate::codegen::{
    build_node_names, pascal_case, templates, GeneratedFile, GeneratedWorkspace, TopicMap,
};
use crate::model::{Language, Project};

use super::MiddlewareAdapter;

/// ミドルウェアなしの純 Pub/Sub アダプタ（Phase 3 の第2アダプタ）。
/// 全ノードを1プロセスの Python + インメモリバスで動かすワークスペースを生成する。
/// ROS 不要でアーキテクチャの挙動を素振りできる。
pub struct MockPubSubAdapter;

impl MiddlewareAdapter for MockPubSubAdapter {
    fn name(&self) -> &'static str {
        "mock_pubsub"
    }

    fn description(&self) -> &'static str {
        "純 Pub/Sub モック（ROS 不要。全ノードを 1 プロセスの Python で実行）"
    }

    fn generate(&self, project: &Project) -> Result<GeneratedWorkspace, String> {
        let node_names = build_node_names(project);
        let topics = TopicMap::build(project, &node_names);
        let mut ws = GeneratedWorkspace::default();

        if project.nodes.iter().any(|n| n.language != Language::Python) {
            ws.warnings.push(
                "mock_pubsub はすべてのノードを Python で生成します（cpp / rust の言語設定は無視されます）"
                    .to_string(),
            );
        }

        // 型モジュール（カスタム型 dataclass + 参照される外部型のスタブ）
        ws.files.push(self.msg_types_file(project)?);

        // 固定のバス実装
        ws.files.push(GeneratedFile {
            rel_path: PathBuf::from("mockbus.py"),
            content: include_str!("../templates/mock/mockbus.py").to_string(),
            protected: false,
        });

        ws.files.push(GeneratedFile {
            rel_path: PathBuf::from("nodes").join("__init__.py"),
            content: String::new(),
            protected: false,
        });

        // ノードごとの interfaces / 実装部
        let mut run_nodes = Vec::new();
        for node in &project.nodes {
            let node_name = &node_names[&node.id];
            let node_dir = PathBuf::from("nodes").join(node_name);
            let class_base = format!("{}Base", pascal_case(&node.label));

            let ports = |list: &[crate::model::PortDef], input: bool| -> Vec<Value> {
                list.iter()
                    .map(|p| {
                        let mut m = Map::new();
                        m.insert("name".into(), Value::String(p.name.clone()));
                        m.insert("py_type".into(), Value::String(mock_py_type(&p.ty)));
                        let topic = if input {
                            topics.input_topic(&node.id, node_name, &p.name)
                        } else {
                            topics.output_topic(node_name, &p.name)
                        };
                        m.insert("topic".into(), Value::String(topic));
                        Value::Object(m)
                    })
                    .collect()
            };

            let params: Vec<Value> = node
                .params
                .iter()
                .map(|p| {
                    let mut m = Map::new();
                    m.insert("name".into(), Value::String(p.name.clone()));
                    m.insert("py_default".into(), Value::String(python_default(p)));
                    Value::Object(m)
                })
                .collect();

            let mut node_ctx = Map::new();
            node_ctx.insert("label".into(), Value::String(node.label.clone()));
            node_ctx.insert("node_name".into(), Value::String(node_name.clone()));
            node_ctx.insert("class_base".into(), Value::String(class_base));
            node_ctx.insert("class_name".into(), Value::String(pascal_case(&node.label)));
            node_ctx.insert("period_ms".into(), Value::from(node.period_ms));
            node_ctx.insert("inputs".into(), Value::Array(ports(&node.inputs, true)));
            node_ctx.insert("outputs".into(), Value::Array(ports(&node.outputs, false)));
            node_ctx.insert("params".into(), Value::Array(params));
            let node_ctx = Value::Object(node_ctx);

            let mut tctx = Context::new();
            tctx.insert("node", &node_ctx);
            ws.files.push(GeneratedFile {
                rel_path: node_dir.join("interfaces.py"),
                content: templates()
                    .render("mock/interfaces_py.tera", &tctx)
                    .map_err(|e| format!("interfaces の生成に失敗: {e}"))?,
                protected: false,
            });
            ws.files.push(GeneratedFile {
                rel_path: node_dir.join(format!("{node_name}.py")),
                content: templates()
                    .render("mock/node_impl.tera", &tctx)
                    .map_err(|e| format!("実装スケルトンの生成に失敗: {e}"))?,
                protected: true,
            });
            ws.files.push(GeneratedFile {
                rel_path: node_dir.join("__init__.py"),
                content: String::new(),
                protected: false,
            });

            run_nodes.push(node_ctx);
        }

        // ランナー
        let mut rctx = Context::new();
        rctx.insert("nodes", &run_nodes);
        rctx.insert("project_name", &project.project.name);
        ws.files.push(GeneratedFile {
            rel_path: PathBuf::from("run.py"),
            content: templates()
                .render("mock/run_py.tera", &rctx)
                .map_err(|e| format!("run.py の生成に失敗: {e}"))?,
            protected: false,
        });

        Ok(ws)
    }
}

impl MockPubSubAdapter {
    /// カスタム型 → dataclass、参照される外部型（pkg/Type）→ 空のスタブ dataclass
    fn msg_types_file(&self, project: &Project) -> Result<GeneratedFile, String> {
        // 外部型スタブの収集（ポートとカスタム型フィールドの両方から）
        let mut stubs: BTreeMap<String, String> = BTreeMap::new(); // Type -> origin
        let mut collect = |ty: &str| {
            if let Some((pkg, name)) = ty.split_once('/') {
                stubs
                    .entry(name.to_string())
                    .or_insert_with(|| format!("{pkg}/{name}"));
            }
        };
        for node in &project.nodes {
            for p in node.inputs.iter().chain(node.outputs.iter()) {
                collect(&p.ty);
            }
        }
        for t in &project.custom_types {
            for f in &t.fields {
                collect(&f.ty);
            }
        }

        let stub_ctxs: Vec<Value> = stubs
            .iter()
            .map(|(name, origin)| {
                let mut m = Map::new();
                m.insert("name".into(), Value::String(name.clone()));
                m.insert("origin".into(), Value::String(origin.clone()));
                Value::Object(m)
            })
            .collect();

        let type_ctxs: Vec<Value> = project
            .custom_types
            .iter()
            .map(|t| {
                let fields: Vec<Value> = t
                    .fields
                    .iter()
                    .map(|f| {
                        let mut m = Map::new();
                        m.insert("name".into(), Value::String(f.name.clone()));
                        let (py_type, py_default) = mock_field_type(&f.ty);
                        m.insert("py_type".into(), Value::String(py_type));
                        m.insert("py_default".into(), Value::String(py_default));
                        Value::Object(m)
                    })
                    .collect();
                let mut m = Map::new();
                m.insert("name".into(), Value::String(t.name.clone()));
                m.insert("fields".into(), Value::Array(fields));
                Value::Object(m)
            })
            .collect();

        let mut ctx = Context::new();
        ctx.insert("stubs", &stub_ctxs);
        ctx.insert("types", &type_ctxs);
        Ok(GeneratedFile {
            rel_path: PathBuf::from("msg_types.py"),
            content: templates()
                .render("mock/msg_types.tera", &ctx)
                .map_err(|e| format!("msg_types.py の生成に失敗: {e}"))?,
            protected: false,
        })
    }
}

/// ポート型 → Python クラス名（pkg/Type はスタブの Type、それ以外はそのまま）
fn mock_py_type(ty: &str) -> String {
    match ty.split_once('/') {
        Some((_, name)) => name.to_string(),
        None => ty.to_string(),
    }
}

/// カスタム型フィールドの型 → (Python 型注釈, 既定値リテラル)
fn mock_field_type(ty: &str) -> (String, String) {
    match ty {
        "bool" => ("bool".to_string(), "False".to_string()),
        "int8" | "uint8" | "int16" | "uint16" | "int32" | "uint32" | "int64" | "uint64"
        | "byte" | "char" => ("int".to_string(), "0".to_string()),
        "float32" | "float64" => ("float".to_string(), "0.0".to_string()),
        "string" | "wstring" => ("str".to_string(), "\"\"".to_string()),
        other => {
            let name = mock_py_type(other);
            (format!("{name} | None"), "None".to_string())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mock_type_mapping() {
        assert_eq!(mock_py_type("sensor_msgs/Imu"), "Imu");
        assert_eq!(mock_py_type("FusedPose"), "FusedPose");
        assert_eq!(
            mock_field_type("float64"),
            ("float".to_string(), "0.0".to_string())
        );
        assert_eq!(
            mock_field_type("geometry_msgs/Vector3"),
            ("Vector3 | None".to_string(), "None".to_string())
        );
    }
}
