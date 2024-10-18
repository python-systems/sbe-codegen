use crate::models::types::primitive_type::{NativeType, ResolvableType};
use crate::models::types::{CharacterEncoding, Presence, SizedEncoded};
use crate::models::TypeMap;
use anyhow::Result;
use hard_xml::XmlRead;
use std::collections::HashMap;

#[derive(XmlRead, PartialEq, Debug, Clone)]
#[xml(tag = "type")]
pub struct EncodedDataType {
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

    // Presence attributes
    #[xml(default, attr = "presence")]
    pub presence: Presence,

    #[xml(attr = "nullValue")]
    pub null_value: Option<String>,
    #[xml(attr = "minValue")]
    pub min_value: Option<String>,
    #[xml(attr = "maxValue")]
    pub max_value: Option<String>,
    #[xml(attr = "length")]
    pub length: Option<usize>,
    #[xml(attr = "primitiveType")]
    pub primitive_type: NativeType,
    #[xml(attr = "characterEncoding")]
    pub character_encoding: Option<CharacterEncoding>,

    #[xml(text)]
    pub default_value: Option<String>,
}

impl EncodedDataType {
    pub fn is_string(&self) -> bool {
        self.character_encoding.is_some()
    }

    pub fn is_hashable(&self, types: &HashMap<String, EncodedDataType>) -> Result<bool> {
        self.primitive_type.resolved(types).map(|t| t.is_hashable())
    }
}

impl SizedEncoded for EncodedDataType {
    fn size(&self, types: &TypeMap) -> Result<usize> {
        let size = self.primitive_type.size(types)?;

        Ok(size
            * match self.length {
                Some(length) => length,
                None => match self.primitive_type {
                    NativeType::Char => self
                        .default_value
                        .as_ref()
                        .map_or(1, |val| val.len().max(1)),
                    _ => 1,
                },
            })
    }
}
