//! コード生成の統合テスト。
//! サンプルプロジェクト（2ノード Pub/Sub + カスタム型）から
//! 要求仕様の「出力イメージ」どおりのワークスペースが生成されることを確認する。

use archisyn_lib::codegen::generate_workspace;
use archisyn_lib::fs::safe_write::write_files;
use archisyn_lib::model::*;

fn demo_project() -> Project {
    Project {
        arcsyn_version: ARCSYN_VERSION.to_string(),
        project: ProjectMeta {
            name: "demo_robot".to_string(),
            middleware: "ros2_humble".to_string(),
        },
        custom_types: vec![CustomType {
            name: "FusedPose".to_string(),
            fields: vec![
                TypeField {
                    name: "position".to_string(),
                    ty: "geometry_msgs/Vector3".to_string(),
                },
                TypeField {
                    name: "confidence".to_string(),
                    ty: "float64".to_string(),
                },
            ],
        }],
        nodes: vec![
            NodeDef {
                id: "n1".to_string(),
                label: "SensorFusion".to_string(),
                language: Language::Python,
                namespace: None,
                period_ms: 50,
                position: Vec2 { x: 0.0, y: 0.0 },
                size: None,
                inputs: vec![PortDef {
                    name: "imu".to_string(),
                    ty: "sensor_msgs/Imu".to_string(),
                }],
                outputs: vec![PortDef {
                    name: "fused".to_string(),
                    ty: "FusedPose".to_string(),
                }],
                params: vec![ParamDef {
                    name: "alpha".to_string(),
                    ty: "float64".to_string(),
                    default: "0.7".to_string(),
                }],
            },
            NodeDef {
                id: "n2".to_string(),
                label: "Controller".to_string(),
                language: Language::Python,
                namespace: None,
                period_ms: 100,
                position: Vec2 { x: 300.0, y: 0.0 },
                size: None,
                inputs: vec![PortDef {
                    name: "pose".to_string(),
                    ty: "FusedPose".to_string(),
                }],
                outputs: vec![PortDef {
                    name: "cmd".to_string(),
                    ty: "geometry_msgs/Twist".to_string(),
                }],
                params: vec![],
            },
        ],
        edges: vec![EdgeDef {
            id: "e1".to_string(),
            source: Endpoint {
                node: "n1".to_string(),
                port: "fused".to_string(),
            },
            target: Endpoint {
                node: "n2".to_string(),
                port: "pose".to_string(),
            },
        }],
        launch: LaunchSettings::default(),
        viewport: Viewport::default(),
    }
}

fn content_of<'a>(files: &'a [archisyn_lib::codegen::GeneratedFile], rel: &str) -> &'a str {
    &files
        .iter()
        .find(|f| f.rel_path.to_string_lossy() == rel)
        .unwrap_or_else(|| panic!("{rel} が生成されていません"))
        .content
}

#[test]
fn generates_expected_workspace_layout() {
    let ws = generate_workspace(&demo_project()).unwrap();
    let paths: Vec<String> = ws
        .files
        .iter()
        .map(|f| f.rel_path.to_string_lossy().to_string())
        .collect();

    for expected in [
        "src/demo_robot_msgs/package.xml",
        "src/demo_robot_msgs/CMakeLists.txt",
        "src/demo_robot_msgs/msg/FusedPose.msg",
        "src/demo_robot_py_nodes/package.xml",
        "src/demo_robot_py_nodes/setup.py",
        "src/demo_robot_py_nodes/setup.cfg",
        "src/demo_robot_py_nodes/resource/demo_robot_py_nodes",
        "src/demo_robot_py_nodes/demo_robot_py_nodes/sensor_fusion/__init__.py",
        "src/demo_robot_py_nodes/demo_robot_py_nodes/sensor_fusion/interfaces.py",
        "src/demo_robot_py_nodes/demo_robot_py_nodes/sensor_fusion/sensor_fusion.py",
        "src/demo_robot_py_nodes/demo_robot_py_nodes/controller/interfaces.py",
        "src/demo_robot_py_nodes/demo_robot_py_nodes/controller/controller.py",
        "launch/system.launch.py",
    ] {
        assert!(
            paths.contains(&expected.to_string()),
            "{expected} がない: {paths:?}"
        );
    }
    assert!(ws.warnings.is_empty(), "警告なしのはず: {:?}", ws.warnings);
}

#[test]
fn interface_wires_topics_via_edges() {
    let ws = generate_workspace(&demo_project()).unwrap();
    let fusion = content_of(
        &ws.files,
        "src/demo_robot_py_nodes/demo_robot_py_nodes/sensor_fusion/interfaces.py",
    );
    let controller = content_of(
        &ws.files,
        "src/demo_robot_py_nodes/demo_robot_py_nodes/controller/interfaces.py",
    );

    // 出力: 自ノード名/ポート名 のトピックへ publish
    assert!(fusion.contains(
        "create_publisher(\n            FusedPose, \"/sensor_fusion/fused\", 10\n        )"
    ));
    // 入力: エッジで接続された接続元のトピックを subscribe
    assert!(controller.contains("FusedPose, \"/sensor_fusion/fused\", self._handle_pose, 10"));
    // 未接続入力はフォールバックトピック
    assert!(fusion.contains("Imu, \"/sensor_fusion/imu\", self._handle_imu, 10"));
    // パラメータと周期
    assert!(fusion.contains("declare_parameter(\"alpha\", 0.7)"));
    assert!(fusion.contains("create_timer(0.05, self.on_update)"));
    // 実装部はノードディレクトリ内の interfaces を import する
    let impl_file = content_of(
        &ws.files,
        "src/demo_robot_py_nodes/demo_robot_py_nodes/controller/controller.py",
    );
    assert!(
        impl_file.contains("from demo_robot_py_nodes.controller.interfaces import ControllerBase")
    );
}

#[test]
fn msg_file_and_launch_content() {
    let ws = generate_workspace(&demo_project()).unwrap();
    assert_eq!(
        content_of(&ws.files, "src/demo_robot_msgs/msg/FusedPose.msg"),
        "geometry_msgs/Vector3 position\nfloat64 confidence\n"
    );
    let launch = content_of(&ws.files, "launch/system.launch.py");
    assert!(launch.contains("executable=\"sensor_fusion\""));
    assert!(launch.contains("executable=\"controller\""));
}

#[test]
fn implementation_files_are_protected_on_regeneration() {
    let root = std::env::temp_dir().join("arcsyn_codegen_test/protect");
    std::fs::remove_dir_all(&root).ok();
    std::fs::create_dir_all(&root).unwrap();

    let ws = generate_workspace(&demo_project()).unwrap();
    let first = write_files(&root, &ws.files).unwrap();
    assert!(first.skipped.is_empty());

    // 実装部にユーザーの変更を加える
    let impl_path =
        root.join("src/demo_robot_py_nodes/demo_robot_py_nodes/controller/controller.py");
    std::fs::write(&impl_path, "# user implementation\n").unwrap();

    // 再生成しても実装部は上書きされない
    let second = write_files(&root, &ws.files).unwrap();
    assert!(second.skipped.iter().any(|p| p.ends_with("controller.py")));
    assert_eq!(
        std::fs::read_to_string(&impl_path).unwrap(),
        "# user implementation\n"
    );
    // インターフェース部は再生成される
    assert!(second
        .written
        .iter()
        .any(|p| p.ends_with("controller/interfaces.py")));
}

#[test]
fn mixed_language_workspace_generates_cpp_package() {
    // Controller を C++ に切り替えた混在構成
    let mut project = demo_project();
    project.nodes[1].language = Language::Cpp;

    let ws = generate_workspace(&project).unwrap();
    assert!(ws.warnings.is_empty(), "{:?}", ws.warnings);
    let paths: Vec<String> = ws
        .files
        .iter()
        .map(|f| f.rel_path.to_string_lossy().to_string())
        .collect();

    for expected in [
        // Python 側は SensorFusion のみ
        "src/demo_robot_py_nodes/demo_robot_py_nodes/sensor_fusion/sensor_fusion.py",
        // C++ 側（ノード完結型）
        "src/demo_robot_cpp_nodes/CMakeLists.txt",
        "src/demo_robot_cpp_nodes/package.xml",
        "src/demo_robot_cpp_nodes/src/controller/interfaces.hpp",
        "src/demo_robot_cpp_nodes/src/controller/controller.cpp",
    ] {
        assert!(
            paths.contains(&expected.to_string()),
            "{expected} がない: {paths:?}"
        );
    }
    // Python 側に controller は含まれない
    assert!(!paths
        .iter()
        .any(|p| p.contains("demo_robot_py_nodes/controller")));

    // C++ interfaces: 型・トピック・保護
    let hpp = content_of(
        &ws.files,
        "src/demo_robot_cpp_nodes/src/controller/interfaces.hpp",
    );
    assert!(hpp.contains("#include <demo_robot_msgs/msg/fused_pose.hpp>"));
    assert!(hpp.contains("create_subscription<demo_robot_msgs::msg::FusedPose>"));
    assert!(hpp.contains("\"/sensor_fusion/fused\""));
    assert!(hpp.contains("std::chrono::milliseconds(100)"));

    let cpp_impl = ws
        .files
        .iter()
        .find(|f| f.rel_path.ends_with("controller/controller.cpp"))
        .unwrap();
    assert!(cpp_impl.protected, "C++ 実装部は保護対象のはず");

    // CMakeLists: 実行ファイルと依存
    let cmake = content_of(&ws.files, "src/demo_robot_cpp_nodes/CMakeLists.txt");
    assert!(cmake.contains("add_executable(controller src/controller/controller.cpp)"));
    assert!(cmake.contains("demo_robot_msgs"));

    // launch には両言語のノードが載る
    let launch = content_of(&ws.files, "launch/system.launch.py");
    assert!(launch.contains("package=\"demo_robot_py_nodes\""));
    assert!(launch.contains("package=\"demo_robot_cpp_nodes\""));
}

#[test]
fn rust_nodes_generate_cargo_package() {
    // Controller を Rust に切り替えた構成
    let mut project = demo_project();
    project.nodes[1].language = Language::Rust;

    let ws = generate_workspace(&project).unwrap();
    // Rust ノードには underlay の注意警告が付く
    assert!(ws.warnings.iter().any(|w| w.contains("ros2_rust")));

    let paths: Vec<String> = ws
        .files
        .iter()
        .map(|f| f.rel_path.to_string_lossy().to_string())
        .collect();
    for expected in [
        "src/demo_robot_rust_nodes/Cargo.toml",
        "src/demo_robot_rust_nodes/package.xml",
        "src/demo_robot_rust_nodes/src/controller/interfaces.rs",
        "src/demo_robot_rust_nodes/src/controller/controller.rs",
    ] {
        assert!(
            paths.contains(&expected.to_string()),
            "{expected} がない: {paths:?}"
        );
    }

    let cargo = content_of(&ws.files, "src/demo_robot_rust_nodes/Cargo.toml");
    assert!(cargo.contains("name = \"controller\""));
    assert!(cargo.contains("path = \"src/controller/controller.rs\""));
    assert!(cargo.contains("demo_robot_msgs = \"*\""));

    let interfaces = content_of(
        &ws.files,
        "src/demo_robot_rust_nodes/src/controller/interfaces.rs",
    );
    assert!(interfaces.contains("pub struct ControllerInterfaces"));
    assert!(interfaces.contains("\"/sensor_fusion/fused\""));
    assert!(interfaces.contains("demo_robot_msgs::msg::FusedPose"));
    assert!(interfaces.contains("Duration::from_millis(100)"));

    let impl_file = ws
        .files
        .iter()
        .find(|f| f.rel_path.ends_with("controller/controller.rs"))
        .unwrap();
    assert!(impl_file.protected, "Rust 実装部は保護対象のはず");

    // package.xml は ament_cargo
    let pkg_xml = content_of(&ws.files, "src/demo_robot_rust_nodes/package.xml");
    assert!(pkg_xml.contains("<build_type>ament_cargo</build_type>"));
}

#[test]
fn unknown_middleware_is_rejected() {
    let mut project = demo_project();
    project.project.middleware = "unknown_mw".to_string();
    let err = generate_workspace(&project).unwrap_err();
    assert!(err.contains("未対応のミドルウェア"), "{err}");
    assert!(err.contains("ros2_humble"), "対応一覧を含むべき: {err}");
}

#[test]
fn mock_pubsub_generates_runnable_python_workspace() {
    let mut project = demo_project();
    project.project.middleware = "mock_pubsub".to_string();
    // 言語設定は無視される（警告付き）
    project.nodes[1].language = Language::Cpp;

    let ws = generate_workspace(&project).unwrap();
    assert!(ws.warnings.iter().any(|w| w.contains("mock_pubsub")));

    let paths: Vec<String> = ws
        .files
        .iter()
        .map(|f| f.rel_path.to_string_lossy().to_string())
        .collect();
    for expected in [
        "run.py",
        "mockbus.py",
        "msg_types.py",
        "nodes/sensor_fusion/interfaces.py",
        "nodes/sensor_fusion/sensor_fusion.py",
        "nodes/controller/interfaces.py",
        "nodes/controller/controller.py",
    ] {
        assert!(
            paths.contains(&expected.to_string()),
            "{expected} がない: {paths:?}"
        );
    }

    // カスタム型は dataclass、外部型はスタブ
    let types = content_of(&ws.files, "msg_types.py");
    assert!(types.contains("class FusedPose:"));
    assert!(types.contains("confidence: float = 0.0"));
    assert!(types.contains("class Imu:"));

    // トピック配線はエッジ由来（ROS 版と同じ規約）
    let controller = content_of(&ws.files, "nodes/controller/interfaces.py");
    assert!(controller.contains("bus.subscribe(\"/sensor_fusion/fused\""));

    // 実装部は保護対象
    assert!(
        ws.files
            .iter()
            .find(|f| f.rel_path.ends_with("controller/controller.py"))
            .unwrap()
            .protected
    );
}

#[test]
fn launch_includes_namespace_and_parameters() {
    let mut project = demo_project();
    project.nodes[0].namespace = Some("front".to_string());

    let ws = generate_workspace(&project).unwrap();
    let launch = content_of(&ws.files, "launch/system.launch.py");
    // namespace とパラメータが launch に反映される
    assert!(launch.contains("namespace=\"front\""), "{launch}");
    assert!(launch.contains("\"alpha\": 0.7"), "{launch}");
    // namespace 付きノードのトピックは /ns/node/port になり、
    // 別 namespace の購読側も接続元の絶対トピックを subscribe する
    let fusion = content_of(
        &ws.files,
        "src/demo_robot_py_nodes/demo_robot_py_nodes/sensor_fusion/interfaces.py",
    );
    assert!(
        fusion.contains("\"/front/sensor_fusion/fused\""),
        "{fusion}"
    );
    let controller = content_of(
        &ws.files,
        "src/demo_robot_py_nodes/demo_robot_py_nodes/controller/interfaces.py",
    );
    assert!(
        controller.contains("\"/front/sensor_fusion/fused\""),
        "{controller}"
    );
}

#[test]
fn launch_args_and_configs_generate_multiple_launch_files() {
    let mut project = demo_project();
    project.launch = LaunchSettings {
        args: vec![LaunchArgDef {
            name: "use_sim_time".to_string(),
            default: "false".to_string(),
        }],
        configs: vec![
            LaunchConfigDef {
                name: "Sensors".to_string(),
                nodes: vec!["n1".to_string()],
            },
            LaunchConfigDef {
                name: "empty".to_string(),
                nodes: vec!["nx".to_string()], // 存在しない id
            },
        ],
    };

    let ws = generate_workspace(&project).unwrap();
    let paths: Vec<String> = ws
        .files
        .iter()
        .map(|f| f.rel_path.to_string_lossy().to_string())
        .collect();
    assert!(paths.contains(&"launch/system.launch.py".to_string()));
    assert!(paths.contains(&"launch/sensors.launch.py".to_string()));
    assert!(!paths.contains(&"launch/empty.launch.py".to_string()));
    assert!(ws.warnings.iter().any(|w| w.contains("empty")));

    // 引数の宣言と全ノードへのパラメータ受け渡し
    let system = content_of(&ws.files, "launch/system.launch.py");
    assert!(system.contains("DeclareLaunchArgument(\"use_sim_time\", default_value=\"false\")"));
    assert!(system.contains("\"use_sim_time\": LaunchConfiguration(\"use_sim_time\")"));
    assert!(system.contains("\"alpha\": 0.7"));

    // 構成 launch には選択ノードのみ
    let sensors = content_of(&ws.files, "launch/sensors.launch.py");
    assert!(sensors.contains("executable=\"sensor_fusion\""));
    assert!(!sensors.contains("executable=\"controller\""));
}
