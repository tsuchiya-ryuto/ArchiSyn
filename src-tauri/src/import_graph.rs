//! 実行中システムのグラフダンプ（tools/introspect.py の JSON）から
//! .arcsyn プロジェクトを復元する（Phase 5.3 段階1: rqt_graph 方式）。

use std::collections::{BTreeMap, HashMap};

use serde::Deserialize;

use crate::codegen::{pascal_case, snake_case};
use crate::model::{
    EdgeDef, Endpoint, Language, NodeDef, ParamDef, PortDef, Project, ProjectMeta, Vec2, Viewport,
    ARCSYN_VERSION,
};

#[derive(Debug, Deserialize)]
struct GraphDump {
    version: String,
    nodes: Vec<DumpNode>,
}

#[derive(Debug, Deserialize)]
struct DumpNode {
    name: String,
    #[serde(default)]
    namespace: String,
    #[serde(default)]
    publishers: Vec<DumpPort>,
    #[serde(default)]
    subscriptions: Vec<DumpPort>,
    #[serde(default)]
    parameters: Vec<DumpParam>,
}

#[derive(Debug, Deserialize)]
struct DumpPort {
    topic: String,
    #[serde(rename = "type")]
    ty: String,
}

#[derive(Debug, Deserialize)]
struct DumpParam {
    name: String,
    #[serde(rename = "type")]
    ty: String,
    value: serde_json::Value,
}

#[derive(Debug)]
pub struct ImportResult {
    pub project: Project,
    pub warnings: Vec<String>,
}

/// "pkg/msg/Type" → "pkg/Type"（.arcsyn の型表記へ正規化）
fn normalize_type(ty: &str) -> String {
    let parts: Vec<&str> = ty.split('/').collect();
    match parts.as_slice() {
        [pkg, "msg", name] => format!("{pkg}/{name}"),
        _ => ty.to_string(),
    }
}

/// トピック名からポート名を作る（最後のセグメントを snake 化し、一意化）
fn port_name_from_topic(topic: &str, used: &mut Vec<String>) -> String {
    let base = snake_case(topic.rsplit('/').next().unwrap_or("port"));
    let mut name = base.clone();
    let mut i = 2;
    while used.contains(&name) {
        name = format!("{base}_{i}");
        i += 1;
    }
    used.push(name.clone());
    name
}

/// パラメータ値を .arcsyn の default（文字列）へ変換
fn param_default(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}

pub fn import_graph_json(text: &str, project_name: &str) -> Result<ImportResult, String> {
    let dump: GraphDump =
        serde_json::from_str(text).map_err(|e| format!("グラフ JSON の解析に失敗しました: {e}"))?;
    if dump.version != "1" {
        return Err(format!(
            "未対応のグラフダンプバージョンです: {}",
            dump.version
        ));
    }
    if dump.nodes.is_empty() {
        return Err(
            "グラフにノードがありません（対象システムの起動中に introspect.py を実行してください）"
                .to_string(),
        );
    }

    let mut warnings = vec![
        "言語は取得できないため全ノードを python に仮設定しました".to_string(),
        "実行周期は取得できないため 100 ms に仮設定しました".to_string(),
    ];

    // (topic → 発行者[(node_id, port_name)]) を作りながらノードを変換する
    let mut nodes = Vec::new();
    let mut publishers_by_topic: HashMap<String, Vec<(String, String)>> = HashMap::new();
    let mut sub_ports: Vec<(String, String, String)> = Vec::new(); // (node_id, port, topic)

    for (index, dumped) in dump.nodes.iter().enumerate() {
        let id = format!("n{}", index + 1);
        let mut used = Vec::new();
        let mut outputs = Vec::new();
        for p in &dumped.publishers {
            let name = port_name_from_topic(&p.topic, &mut used);
            publishers_by_topic
                .entry(p.topic.clone())
                .or_default()
                .push((id.clone(), name.clone()));
            outputs.push(PortDef {
                name,
                ty: normalize_type(&p.ty),
            });
        }
        let mut used = Vec::new();
        let mut inputs = Vec::new();
        for s in &dumped.subscriptions {
            let name = port_name_from_topic(&s.topic, &mut used);
            sub_ports.push((id.clone(), name.clone(), s.topic.clone()));
            inputs.push(PortDef {
                name,
                ty: normalize_type(&s.ty),
            });
        }

        let namespace = match dumped.namespace.trim_matches('/') {
            "" => None,
            ns => Some(ns.to_string()),
        };

        nodes.push(NodeDef {
            id,
            label: pascal_case(&dumped.name),
            language: Language::Python,
            namespace,
            period_ms: 100,
            position: Vec2 { x: 0.0, y: 0.0 }, // 後段のレイアウトで確定
            size: None,
            inputs,
            outputs,
            params: dumped
                .parameters
                .iter()
                .map(|p| ParamDef {
                    name: p.name.clone(),
                    ty: p.ty.clone(),
                    default: param_default(&p.value),
                })
                .collect(),
        });
    }

    // エッジ復元: 購読トピックと同じトピックの発行者を接続
    let mut edges = Vec::new();
    for (target_id, target_port, topic) in &sub_ports {
        match publishers_by_topic.get(topic) {
            Some(pubs) => {
                for (source_id, source_port) in pubs {
                    edges.push(EdgeDef {
                        id: format!("e{}", edges.len() + 1),
                        source: Endpoint {
                            node: source_id.clone(),
                            port: source_port.clone(),
                        },
                        target: Endpoint {
                            node: target_id.clone(),
                            port: target_port.clone(),
                        },
                    });
                }
            }
            None => warnings.push(format!(
                "トピック {topic} の発行者が見つかりません（購読のみ検出）"
            )),
        }
    }

    layout(&mut nodes, &edges);

    Ok(ImportResult {
        project: Project {
            arcsyn_version: ARCSYN_VERSION.to_string(),
            project: ProjectMeta {
                name: snake_case(project_name),
                middleware: "ros2_humble".to_string(),
            },
            custom_types: Vec::new(),
            nodes,
            edges,
            viewport: Viewport::default(),
        },
        warnings,
    })
}

/// 簡易階層レイアウト: エッジ方向に沿って層を割り当て、左→右に配置する
fn layout(nodes: &mut [NodeDef], edges: &[EdgeDef]) {
    let mut layer: HashMap<String, usize> = nodes.iter().map(|n| (n.id.clone(), 0)).collect();
    // 最長経路ベース（サイクルがあっても反復回数で打ち切る）
    for _ in 0..nodes.len() {
        for e in edges {
            if let (Some(&s), Some(&t)) = (layer.get(&e.source.node), layer.get(&e.target.node)) {
                if t < s + 1 && s + 1 < nodes.len() {
                    layer.insert(e.target.node.clone(), s + 1);
                }
            }
        }
    }

    let mut rows: BTreeMap<usize, usize> = BTreeMap::new();
    for node in nodes.iter_mut() {
        let l = layer[&node.id];
        let row = rows.entry(l).or_insert(0);
        node.position = Vec2 {
            x: 80.0 + (l as f64) * 320.0,
            y: 80.0 + (*row as f64) * 160.0,
        };
        *row += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"{
      "version": "1",
      "nodes": [
        {
          "name": "imu_driver",
          "namespace": "/",
          "publishers": [{"topic": "/imu_driver/imu", "type": "sensor_msgs/msg/Imu"}],
          "subscriptions": [],
          "parameters": []
        },
        {
          "name": "sensor_fusion",
          "namespace": "/front",
          "publishers": [{"topic": "/front/sensor_fusion/fused", "type": "demo_robot_msgs/msg/FusedPose"}],
          "subscriptions": [{"topic": "/imu_driver/imu", "type": "sensor_msgs/msg/Imu"}],
          "parameters": [{"name": "alpha", "type": "float64", "value": 0.7}]
        },
        {
          "name": "controller",
          "namespace": "/",
          "publishers": [],
          "subscriptions": [{"topic": "/front/sensor_fusion/fused", "type": "demo_robot_msgs/msg/FusedPose"}],
          "parameters": []
        }
      ]
    }"#;

    #[test]
    fn imports_nodes_ports_and_edges() {
        let r = import_graph_json(SAMPLE, "demo").unwrap();
        let p = &r.project;
        assert_eq!(p.nodes.len(), 3);
        assert_eq!(p.edges.len(), 2);

        let fusion = p.nodes.iter().find(|n| n.label == "SensorFusion").unwrap();
        assert_eq!(fusion.namespace.as_deref(), Some("front"));
        assert_eq!(fusion.inputs[0].name, "imu");
        assert_eq!(fusion.inputs[0].ty, "sensor_msgs/Imu");
        assert_eq!(fusion.outputs[0].ty, "demo_robot_msgs/FusedPose");
        assert_eq!(fusion.params[0].name, "alpha");
        assert_eq!(fusion.params[0].default, "0.7");

        // エッジ: imu_driver → sensor_fusion → controller
        let edge = &p.edges[0];
        assert_eq!(edge.source.port, "imu");
        assert_eq!(edge.target.port, "imu");
    }

    #[test]
    fn layout_layers_follow_dataflow() {
        let r = import_graph_json(SAMPLE, "demo").unwrap();
        let x = |label: &str| {
            r.project
                .nodes
                .iter()
                .find(|n| n.label == label)
                .unwrap()
                .position
                .x
        };
        assert!(x("ImuDriver") < x("SensorFusion"));
        assert!(x("SensorFusion") < x("Controller"));
    }

    #[test]
    fn warns_on_missing_publisher() {
        let json = r#"{"version":"1","nodes":[
          {"name":"c","namespace":"/","publishers":[],
           "subscriptions":[{"topic":"/orphan","type":"std_msgs/msg/Bool"}],"parameters":[]}]}"#;
        let r = import_graph_json(json, "x").unwrap();
        assert!(r.warnings.iter().any(|w| w.contains("/orphan")));
    }

    #[test]
    fn rejects_empty_graph() {
        assert!(import_graph_json(r#"{"version":"1","nodes":[]}"#, "x").is_err());
    }
}
