use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EdgeDef {
    pub id: String,
    pub source: Endpoint,
    pub target: Endpoint,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Endpoint {
    pub node: String,
    pub port: String,
}
