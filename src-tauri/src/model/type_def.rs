use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CustomType {
    pub name: String,
    pub fields: Vec<TypeField>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TypeField {
    pub name: String,
    #[serde(rename = "type")]
    pub ty: String,
}
