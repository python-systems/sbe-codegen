use crate::models::types::SizedEncoded;
use crate::models::TypeMap;
use hard_xml::XmlRead;

#[derive(XmlRead, PartialEq, Debug, Clone)]
#[xml(tag = "ref")]
pub struct ReferenceType {
    #[xml(attr = "name")]
    pub name: String,

    // Version attributes
    #[xml(attr = "sinceVersion")]
    pub since_version: Option<usize>,
    #[xml(attr = "deprecated")]
    pub deprecated: Option<usize>,

    // Alignment attributes
    #[xml(attr = "offset")]
    pub offset: Option<usize>,

    #[xml(attr = "type")]
    pub type_name: String,
}

impl SizedEncoded for ReferenceType {
    fn size(&self, types: &TypeMap) -> anyhow::Result<usize> {
        types
            .find_type(&self.type_name)
            .ok_or(anyhow::anyhow!("Unknown type {}", self.type_name))
            .and_then(|t| t.size(types))
    }
}
