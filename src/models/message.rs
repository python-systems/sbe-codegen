use crate::models::types::MessageField;
use hard_xml::XmlRead;

#[derive(XmlRead, PartialEq, Debug, Clone)]
#[xml(tag = "sbe:message")]
pub struct MessageType {
    // blockType attributes
    #[xml(attr = "name")]
    pub name: String,
    #[xml(attr = "id")]
    pub id: u16,
    #[xml(attr = "blockLength")]
    pub block_length: Option<usize>,

    // Semantic attributes
    #[xml(attr = "semanticType")]
    pub semantic_type: Option<String>,
    #[xml(attr = "description")]
    pub description: Option<String>,

    // Version attributes
    #[xml(attr = "sinceVersion")]
    pub since_version: Option<usize>,
    #[xml(attr = "deprecated")]
    pub deprecated: Option<usize>,

    // Fields
    #[xml(child = "field", child = "group", child = "data")]
    pub fields: Vec<MessageField>,
}
