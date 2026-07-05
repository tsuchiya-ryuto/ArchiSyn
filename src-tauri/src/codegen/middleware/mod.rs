pub mod mock_pubsub;
pub mod ros2_humble;

use crate::model::Project;

use super::GeneratedWorkspace;

/// ミドルウェア抽象化レイヤ（F-7）。
/// ワークスペース生成の主体はアダプタであり、対応言語・パッケージ構成・
/// 通信の配線方法はアダプタごとに決める。
/// 新しいミドルウェアの追加手順は doc/adding_middleware.md を参照。
pub trait MiddlewareAdapter {
    /// .arcsyn の `middleware:` フィールドに書く識別子（例: ros2_humble）
    fn name(&self) -> &'static str;

    /// GUI に表示する説明
    fn description(&self) -> &'static str;

    /// プロジェクト全体からワークスペースの全生成ファイルを組み立てる
    fn generate(&self, project: &Project) -> Result<GeneratedWorkspace, String>;
}

/// 利用可能なアダプタの一覧（レジストリ）
pub fn adapters() -> Vec<Box<dyn MiddlewareAdapter>> {
    vec![
        Box::new(ros2_humble::Ros2HumbleAdapter),
        Box::new(mock_pubsub::MockPubSubAdapter),
    ]
}

/// `middleware:` フィールドの値からアダプタを解決する
pub fn adapter_for(name: &str) -> Result<Box<dyn MiddlewareAdapter>, String> {
    adapters()
        .into_iter()
        .find(|a| a.name() == name)
        .ok_or_else(|| {
            let available: Vec<_> = adapters().iter().map(|a| a.name().to_string()).collect();
            format!(
                "未対応のミドルウェアです: {name}（対応: {}）",
                available.join(", ")
            )
        })
}

/// ポート/フィールドの型名の解決結果
/// （例: "sensor_msgs/Imu" → package=sensor_msgs, type_name=Imu）
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedType {
    pub package: String,
    pub type_name: String,
}

/// ROS 系アダプタが言語ジェネレータへ提供する型解決インタフェース。
/// （将来 Jazzy 等へ差し替える際も言語ジェネレータを再利用できるようにする）
pub trait RosTypeResolver {
    /// ポート/フィールドの型名をメッセージ型に解決する。
    /// カスタム型はプロジェクト共通の msgs パッケージへマッピングされる。
    fn resolve_type(&self, project: &Project, ty: &str) -> Result<ResolvedType, String>;

    /// 共通 msgs パッケージ名（例: my_robot_msgs）
    fn msgs_package_name(&self, project: &Project) -> String;
}
