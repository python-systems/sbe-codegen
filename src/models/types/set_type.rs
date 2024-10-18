use anyhow::Result;

use crate::models::types::primitive_type::NativeType;
use crate::models::types::SizedEncoded;
use crate::models::TypeMap;
use hard_xml::XmlRead;

#[derive(XmlRead, PartialEq, Debug, Clone)]
#[xml(tag = "set")]
pub struct SetType {
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

    #[xml(attr = "encodingType")]
    pub encoding_type: NativeType,
    #[xml(child = "choice")]
    pub choices: Vec<Choice>,
}

#[derive(XmlRead, PartialEq, Debug, Clone)]
#[xml(tag = "choice")]
pub struct Choice {
    #[xml(attr = "name")]
    pub name: String,

    // Version attributes
    #[xml(attr = "sinceVersion")]
    pub since_version: Option<usize>,
    #[xml(attr = "deprecated")]
    pub deprecated: Option<usize>,

    #[xml(attr = "description")]
    pub description: Option<String>,
    #[xml(text)]
    pub value: u64,
}

impl SizedEncoded for SetType {
    fn size(&self, types: &TypeMap) -> Result<usize> {
        self.encoding_type.size(types)
    }
}
