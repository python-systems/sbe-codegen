// Made in accordance with http://fixprotocol.io/2016/sbe/sbe.xsd

use crate::models::message::MessageType;
use crate::models::types::composite_type::CompositeType;
use crate::models::types::encoded_data_type::EncodedDataType;
use crate::models::types::enum_type::EnumType;
use crate::models::types::group_type::GroupType;
use crate::models::types::set_type::SetType;
use crate::models::types::variable_data_type::VariableDataType;
use crate::models::types::{MessageField, Type};
use anyhow::anyhow;
use std::collections::HashMap;
use std::str::FromStr;

pub mod constants;
pub mod message;
pub mod schema;
pub mod types;

#[derive(PartialEq, Debug, Copy, Clone)]
pub enum ByteOrder {
    LittleEndian,
    BigEndian,
}

impl FromStr for ByteOrder {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> anyhow::Result<Self, Self::Err> {
        match s {
            "littleEndian" => Ok(ByteOrder::LittleEndian),
            "bigEndian" => Ok(ByteOrder::BigEndian),
            _ => Err(anyhow!("Invalid byte order: {}", s)),
        }
    }
}

#[derive(PartialEq, Debug, Clone)]
pub struct TypeMap {
    pub header_type: CompositeType,
    pub encoded_types: HashMap<String, EncodedDataType>,
    pub set_types: HashMap<String, SetType>,
    pub enum_types: HashMap<String, EnumType>,
    pub composite_types: HashMap<String, CompositeType>,
}

impl TypeMap {
    pub fn new(types: HashMap<String, Type>, header_type: CompositeType) -> Self {
        let mut encoded_types = HashMap::new();
        let mut set_types = HashMap::new();
        let mut enum_types = HashMap::new();
        let mut composite_types = HashMap::new();

        for (name, type_) in types {
            match type_ {
                Type::EncodedData(encoded_type) => {
                    encoded_types.insert(name, encoded_type);
                }
                Type::Set(set_type) => {
                    set_types.insert(name, set_type);
                }
                Type::Enum(enum_type) => {
                    enum_types.insert(name, enum_type);
                }
                Type::Composite(composite_type) => {
                    composite_types.insert(name, composite_type);
                }
                Type::Reference(_) => (),
            }
        }

        Self {
            header_type,
            encoded_types,
            set_types,
            enum_types,
            composite_types,
        }
    }

    pub fn find_type(&self, name: &str) -> Option<Type> {
        self.encoded_types
            .get(name)
            .map(|res| Type::EncodedData(res.clone()))
            .or_else(|| self.set_types.get(name).map(|res| Type::Set(res.clone())))
            .or_else(|| self.enum_types.get(name).map(|res| Type::Enum(res.clone())))
            .or_else(|| {
                self.composite_types
                    .get(name)
                    .map(|res| Type::Composite(res.clone()))
            })
    }

    pub fn iter_values(&self) -> impl Iterator<Item = Type> + '_ {
        self.encoded_types
            .values()
            .map(|encoded_type| Type::EncodedData(encoded_type.clone()))
            .chain(
                self.set_types
                    .values()
                    .map(|set_type| Type::Set(set_type.clone())),
            )
            .chain(
                self.enum_types
                    .values()
                    .map(|enum_type| Type::Enum(enum_type.clone())),
            )
            .chain(
                self.composite_types
                    .values()
                    .map(|composite_type| Type::Composite(composite_type.clone())),
            )
    }
}

#[derive(PartialEq, Debug, Clone)]
pub struct MessageTypeMap {
    pub message_types: HashMap<String, MessageType>,
    pub group_types: HashMap<String, GroupType>,
    pub variable_data_types: HashMap<String, VariableDataType>,
}

impl MessageTypeMap {
    pub fn new(
        message_types: HashMap<String, MessageType>,
        message_field_types: HashMap<String, MessageField>,
    ) -> Self {
        let mut group_types = HashMap::new();
        let mut variable_data_types = HashMap::new();

        for (name, field_type) in message_field_types {
            match field_type {
                MessageField::Field(_) => (),
                MessageField::Group(group_type) => {
                    group_types.insert(name, group_type.clone());
                }
                MessageField::VariableData(variable_data_type) => {
                    variable_data_types.insert(name, variable_data_type.clone());
                }
            }
        }

        Self {
            message_types,
            group_types,
            variable_data_types,
        }
    }

    pub fn iter_values(&self) -> impl Iterator<Item = MessageField> + '_ {
        self.group_types
            .values()
            .map(|group_type| MessageField::Group(group_type.clone()))
            .chain(
                self.variable_data_types.values().map(|variable_data_type| {
                    MessageField::VariableData(variable_data_type.clone())
                }),
            )
    }
}
