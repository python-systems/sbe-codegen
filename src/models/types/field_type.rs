use crate::models::types::{Presence, Type};
use crate::models::TypeMap;
use anyhow::Result;
use hard_xml::XmlRead;

#[derive(XmlRead, PartialEq, Debug, Clone)]
#[xml(tag = "field")]
pub struct FieldType {
    #[xml(attr = "name")]
    pub name: String,
    #[xml(attr = "id")]
    pub id: u16,

    // Version attributes
    #[xml(attr = "sinceVersion")]
    pub since_version: Option<usize>,
    #[xml(attr = "deprecated")]
    pub deprecated: Option<usize>,

    // Semantic attributes
    #[xml(attr = "semanticType")]
    pub semantic_type: Option<String>,
    #[xml(attr = "description")]
    pub description: Option<String>,

    // Alignment attributes
    #[xml(attr = "offset")]
    pub offset: Option<usize>,

    // Presence attributes
    #[xml(default, attr = "presence")]
    pub presence: Presence,

    #[xml(attr = "type")]
    pub type_name: String,
    #[xml(attr = "epoch")]
    pub epoch: Option<String>,
    #[xml(attr = "valueRef")]
    pub value_ref: Option<String>,
}

impl FieldType {
    pub fn to_type(&self, types: &TypeMap) -> Result<Type> {
        let resolved_type = types.find_type(&self.type_name).unwrap_or(self.try_into()?);

        Ok(resolved_type)
    }
}
