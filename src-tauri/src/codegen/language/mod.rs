pub mod cpp;
pub mod python;

use std::collections::HashMap;

use crate::model::{Language, NodeDef, Project};

use super::middleware::MiddlewareAdapter;
use super::{GeneratedFile, TopicMap};

/// 言語別ジェネレータへ渡す共有コンテキスト
pub struct GenContext<'a> {
    pub project: &'a Project,
    pub adapter: &'a dyn MiddlewareAdapter,
    /// ノード id → 一意な snake_case 名
    pub node_names: &'a HashMap<String, String>,
    pub topics: &'a TopicMap,
}

/// ノード単位の出力言語（F-6）ごとのコード生成器
pub trait LanguageGenerator {
    fn language(&self) -> Language;

    /// この言語のノード用パッケージ名（例: my_robot_py_nodes）
    fn package_name(&self, project: &Project) -> String;

    /// 対象言語のノード群からパッケージ一式を生成する
    fn generate(&self, ctx: &GenContext, nodes: &[&NodeDef]) -> Result<Vec<GeneratedFile>, String>;
}
