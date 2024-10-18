use crate::models::types::primitive_type::NativeType;
use crate::models::types::SizedEncoded;
use crate::models::TypeMap;
use anyhow::{anyhow, Context, Result};
use hard_xml::XmlRead;

#[derive(XmlRead, PartialEq, Debug, Clone)]
#[xml(tag = "enum")]
pub struct EnumType {
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
    #[xml(child = "validValue")]
    pub values: Vec<ValidValue>,
}

#[derive(XmlRead, PartialEq, Debug, Clone)]
#[xml(tag = "validValue")]
pub struct ValidValue {
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
    pub value: String,
}

impl ValidValue {
    /// Parse the value of the enum as a u64 integer to be able to contain all possible SBE values.
    ///
    /// # Arguments
    /// - `char_encoding` - a flag to determine if the value should be parsed as a character
    ///
    /// # Returns
    /// The parsed value as a u64 integer.
    pub fn encoded_value(&self, char_encoding: bool) -> Result<u64> {
        if char_encoding {
            self.value
                .chars()
                .next()
                .ok_or(anyhow!("Empty enum value"))
                .map(|val| val as u64)
        } else {
            self.value
                .parse()
                .with_context(|| format!("Failed to parse enum value: {}", self.value))
        }
    }
}

impl SizedEncoded for EnumType {
    fn size(&self, types: &TypeMap) -> Result<usize> {
        self.encoding_type.size(types)
    }
}
