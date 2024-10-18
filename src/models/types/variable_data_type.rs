use crate::models::types::composite_type::CompositeType;
use crate::models::types::primitive_type::{NativeType, ResolvableType};
use crate::models::types::Type;
use crate::models::TypeMap;
use anyhow::{anyhow, Result};
use hard_xml::XmlRead;
use std::collections::HashMap;

#[derive(XmlRead, PartialEq, Debug, Clone)]
#[xml(tag = "data")]
pub struct VariableDataType {
    #[xml(attr = "name")]
    pub name: String,
    #[xml(attr = "id")]
    pub id: u16,
    #[xml(attr = "type")]
    pub type_name: String,

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
}

impl VariableDataType {
    pub fn repr_type<'a>(
        &self,
        composite_types: &'a HashMap<String, CompositeType>,
    ) -> Result<&'a CompositeType> {
        composite_types.get(&self.type_name).ok_or(anyhow!(
            "Missing type '{}' for variable data '{}'",
            &self.type_name,
            &self.name
        ))
    }

    pub fn is_string(&self, composite_types: &HashMap<String, CompositeType>) -> Result<bool> {
        let repr_type = self.repr_type(composite_types)?;
        let value_type = match repr_type.fields[1] {
            Type::EncodedData(ref var_data_type) => var_data_type,
            _ => {
                return Err(anyhow!(
                "Only encoded data type expected for the value type in variable data encoding '{}'",
                &self.name
            ))
            }
        };

        Ok(value_type.is_string())
    }

    pub fn is_bytes(&self, types: &TypeMap) -> Result<bool> {
        let repr_type = self.repr_type(&types.composite_types)?;
        let value_type = match repr_type.fields[1] {
            Type::EncodedData(ref var_data_type) => var_data_type,
            _ => {
                return Err(anyhow!(
                "Only encoded data type expected for the value type in variable data encoding '{}'",
                &self.name
            ))
            }
        };

        Ok(matches!(
            value_type.primitive_type.resolved(&types.encoded_types)?,
            NativeType::Char | NativeType::UInt8
        ))
    }
}
