pub mod ros2_humble;

use crate::model::Project;

use super::GeneratedFile;

/// ポート型の解決結果（例: "sensor_msgs/Imu" → package=sensor_msgs, type_name=Imu）
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedType {
    pub package: String,
    pub type_name: String,
}

/// ミドルウェア抽象化レイヤ（F-7）。
/// ROS 2 Humble 以外への差し替えは本 trait の実装追加で行う。
pub trait MiddlewareAdapter {
    fn name(&self) -> &'static str;

    /// ポート/フィールドの型名をミドルウェア上の型に解決する。
    /// カスタム型はプロジェクト共通の msgs パッケージへマッピングされる。
    fn resolve_type(&self, project: &Project, ty: &str) -> Result<ResolvedType, String>;

    /// カスタム型定義から共通メッセージパッケージ一式を生成する
    fn msgs_package(&self, project: &Project) -> Result<Vec<GeneratedFile>, String>;

    /// 全ノードを起動する launch ファイルを生成する
    /// （nodes: (パッケージ名, ノード名) の一覧）
    fn launch_files(&self, nodes: &[(String, String)]) -> Result<Vec<GeneratedFile>, String>;

    /// 共通 msgs パッケージ名（例: my_robot_msgs）
    fn msgs_package_name(&self, project: &Project) -> String;
}
