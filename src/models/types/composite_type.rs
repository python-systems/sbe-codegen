use crate::models::types::{SizedEncoded, Type};
use crate::models::TypeMap;
use anyhow::Result;
use hard_xml::XmlRead;

#[derive(XmlRead, PartialEq, Debug, Clone)]
#[xml(tag = "composite")]
pub struct CompositeType {
    #[xml(attr = "name")]
    pub name: String,

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

    #[xml(
        child = "type",
        child = "set",
        child = "enum",
        child = "composite",
        child = "ref"
    )]
    pub fields: Vec<Type>,
}

impl SizedEncoded for CompositeType {
    fn size(&self, types: &TypeMap) -> Result<usize> {
        self.fields.iter().map(|f| f.size(types)).sum()
    }
}
