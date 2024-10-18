pub mod composite_type;
pub mod encoded_data_type;
pub mod enum_type;
pub mod field_type;
pub mod group_type;
pub mod primitive_type;
pub mod reference_type;
pub mod set_type;
pub mod variable_data_type;

use crate::models::types::composite_type::CompositeType;
use crate::models::types::encoded_data_type::EncodedDataType;
use crate::models::types::enum_type::EnumType;
use crate::models::types::field_type::FieldType;
use crate::models::types::group_type::GroupType;
use crate::models::types::primitive_type::NativeType;
use crate::models::types::set_type::SetType;
use crate::models::types::variable_data_type::VariableDataType;
use crate::models::TypeMap;
use anyhow::{anyhow, Result};
use hard_xml::XmlRead;
use reference_type::ReferenceType;
use std::str::FromStr;

#[derive(XmlRead, PartialEq, Debug, Clone)]
pub enum Type {
    #[xml(tag = "type")]
    EncodedData(EncodedDataType),
    #[xml(tag = "set")]
    Set(SetType),
    #[xml(tag = "enum")]
    Enum(EnumType),
    #[xml(tag = "composite")]
    Composite(CompositeType),
    #[xml(tag = "ref")]
    Reference(ReferenceType),
}

impl Type {
    pub fn name(&self) -> &str {
        match self {
            Type::EncodedData(t) => &t.name,
            Type::Set(t) => &t.name,
            Type::Enum(t) => &t.name,
            Type::Composite(t) => &t.name,
            Type::Reference(t) => &t.name,
        }
    }

    pub fn presence(&self, types: &TypeMap) -> Result<Presence> {
        Ok(match self {
            Type::EncodedData(encoded_data_type) => encoded_data_type.presence,
            Type::Reference(reference_type) => {
                let referenced_type = types.find_type(&reference_type.type_name).ok_or(anyhow!(
                    "Referenced type {} not found",
                    reference_type.type_name
                ))?;

                referenced_type.presence(types)?
            }
            _ => Presence::Required,
        })
    }
}

impl SizedEncoded for Type {
    fn size(&self, types: &TypeMap) -> Result<usize> {
        match self {
            Type::EncodedData(t) => match t.presence {
                Presence::Constant => Ok(0usize),
                _ => t.size(types),
            },
            Type::Set(t) => t.size(types),
            Type::Enum(t) => t.size(types),
            Type::Composite(t) => t.size(types),
            Type::Reference(t) => t.size(types),
        }
    }
}

impl TryFrom<&FieldType> for Type {
    type Error = anyhow::Error;

    fn try_from(value: &FieldType) -> Result<Self> {
        Ok(Self::EncodedData(EncodedDataType {
            name: value.name.clone(),
            since_version: value.since_version,
            deprecated: value.deprecated,
            semantic_type: value.semantic_type.clone(),
            description: value.description.clone(),
            offset: value.offset,
            presence: value.presence,
            null_value: None,
            min_value: None,
            max_value: None,
            length: None,
            primitive_type: NativeType::from_str(value.type_name.as_str())?,
            character_encoding: None,
            default_value: value.value_ref.clone(),
        }))
    }
}

#[derive(Default, PartialEq, Debug, Copy, Clone)]
pub enum Presence {
    Constant,
    #[default]
    Required,
    Optional,
}

impl FromStr for Presence {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        match s {
            "constant" => Ok(Presence::Constant),
            "required" => Ok(Presence::Required),
            "optional" => Ok(Presence::Optional),
            _ => Err(anyhow!("Invalid presence: {}", s)),
        }
    }
}

#[derive(XmlRead, PartialEq, Debug, Clone)]
pub enum MessageField {
    #[xml(tag = "field")]
    Field(FieldType),
    #[xml(tag = "group")]
    Group(GroupType),
    #[xml(tag = "data")]
    VariableData(VariableDataType),
}

impl MessageField {
    pub fn name(&self) -> &str {
        match self {
            MessageField::Field(t) => &t.name,
            MessageField::Group(t) => &t.name,
            MessageField::VariableData(t) => &t.name,
        }
    }
}

#[derive(PartialEq, Debug, Copy, Clone)]
pub enum CharacterEncoding {
    Ascii,
    Utf8,
}

impl FromStr for CharacterEncoding {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        match s {
            "ASCII" => Ok(CharacterEncoding::Ascii),
            "UTF-8" => Ok(CharacterEncoding::Utf8),
            _ => Err(anyhow!("Invalid character encoding: {}", s)),
        }
    }
}

pub trait SizedEncoded {
    fn size(&self, types: &TypeMap) -> Result<usize>;
}
