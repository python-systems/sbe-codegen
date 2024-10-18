use crate::models::message::MessageType;
use crate::models::types::{MessageField, Type};
use anyhow::{anyhow, Result};
use hard_xml::{XmlRead, XmlReader};
use std::collections::HashMap;

use crate::models::{ByteOrder, MessageTypeMap, TypeMap};

#[derive(XmlRead, PartialEq, Debug, Clone)]
#[xml(tag = "sbe:messageSchema")]
pub struct MessageSchema {
    #[xml(attr = "package")]
    pub package: String,
    #[xml(attr = "id")]
    pub id: u16,
    #[xml(attr = "version")]
    pub version: usize,
    #[xml(attr = "semanticVersion")]
    pub semantic_version: Option<String>,
    #[xml(attr = "description")]
    pub description: Option<String>,
    #[xml(attr = "byteOrder")]
    pub byte_order: Option<ByteOrder>,
    #[xml(attr = "headerType")]
    pub header_type: Option<String>,
    #[xml(child = "types")]
    pub types_section: Vec<TypesSection>,
    #[xml(child = "sbe:message")]
    pub message_types: Vec<MessageType>,
}

/// This conveys very similar information to the `MessageSchema` struct, which
/// is used to load information from the XML file. This structure contains data
/// with, for example, resolved defaults.
#[derive(PartialEq, Debug, Clone)]
pub struct ValidatedMessageSchema {
    pub package: String,
    pub id: u16,
    pub version: usize,
    pub semantic_version: String,
    pub description: Option<String>,
    pub byte_order: ByteOrder,
    pub types: TypeMap,
    pub message_types: MessageTypeMap,
}

#[derive(XmlRead, PartialEq, Debug, Clone)]
#[xml(tag = "types")]
pub struct TypesSection {
    #[xml(child = "type", child = "composite", child = "enum", child = "set")]
    pub types: Vec<Type>,
}

impl MessageSchema {
    pub fn load_from_string(content: &str) -> Result<Self> {
        let mut reader = XmlReader::new(content);
        let message_schema = Self::from_reader(&mut reader)?;
        Ok(message_schema)
    }

    fn find_nested_types(type_: &Type) -> HashMap<String, Type> {
        let mut result = HashMap::new();
        match type_ {
            Type::Composite(c) => {
                result.insert(c.name.clone(), type_.clone());
                result.extend(c.fields.iter().flat_map(Self::find_nested_types));
            }
            Type::Reference(_) => (),
            _ => {
                result.insert(type_.name().to_owned(), type_.clone());
            }
        };

        result
    }

    fn find_nested_message_types(message_field: &MessageField) -> HashMap<String, MessageField> {
        let mut result = HashMap::new();
        result.insert(message_field.name().to_owned(), message_field.clone());

        if let MessageField::Group(group_type) = message_field {
            result.extend(
                group_type
                    .fields
                    .iter()
                    .flat_map(Self::find_nested_message_types),
            );
        }

        result
    }

    pub fn types(&self) -> HashMap<String, Type> {
        self.types_section
            .iter()
            .flat_map(|section| section.types.iter())
            .flat_map(Self::find_nested_types)
            .collect()
    }

    pub fn message_types(&self) -> HashMap<String, MessageType> {
        self.message_types
            .iter()
            .map(|message_type| (message_type.name.clone(), message_type.clone()))
            .collect()
    }

    pub fn message_field_types(&self) -> HashMap<String, MessageField> {
        self.message_types
            .iter()
            .flat_map(|message_type| {
                message_type
                    .fields
                    .iter()
                    .flat_map(Self::find_nested_message_types)
            })
            .collect()
    }

    pub fn project_version(&self) -> String {
        let version = self
            .semantic_version
            .clone()
            .unwrap_or(format!("{}.0.0", self.version));

        // Cargo requires it to have exactly 3 parts
        let dot_count = version.chars().filter(|c| *c == '.').count();

        if dot_count == 0 {
            format!("{}.0.0", version)
        } else if dot_count == 1 {
            format!("{}.0", version)
        } else {
            version
        }
    }

    /// This transforms the message schema loaded from XML to the one we work with.
    /// It resolves defaults and looks up types.
    pub fn validate(self) -> Result<ValidatedMessageSchema> {
        let types = self.types();
        let message_types = self.message_types();
        let message_field_types = self.message_field_types();

        let package = self
            .package
            .split('.')
            .last()
            .ok_or(anyhow!("Package name was missing."))?
            .to_owned();

        let byte_order = self.byte_order.unwrap_or(ByteOrder::LittleEndian);
        let semantic_version = self.project_version();

        let header_type_name = self.header_type.unwrap_or("messageHeader".to_string());
        let header_type = types
            .get(&header_type_name)
            .cloned()
            .ok_or(anyhow!("Header type '{}' not found", header_type_name))?;

        let header_type = match header_type {
            Type::Composite(composite_type) => composite_type,
            _ => {
                return Err(anyhow!(
                    "Header type '{}' is not a composite type",
                    header_type_name
                ))
            }
        };

        Ok(ValidatedMessageSchema {
            package,
            id: self.id,
            version: self.version,
            semantic_version,
            description: self.description.clone(),
            byte_order,
            types: TypeMap::new(types, header_type),
            message_types: MessageTypeMap::new(message_types, message_field_types),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::MessageSchema;
    use crate::models::types::SizedEncoded;
    use std::path::Path;
    use xml_include::resolve_xml_includes;

    #[test]
    fn test_parse_example_message_schema() {
        let path = Path::new("./examples/example-schema.xml");
        let merged_content = resolve_xml_includes(path).unwrap();
        let schema = MessageSchema::load_from_string(&merged_content).unwrap();
        let validated_schema = schema.validate().unwrap();
        println!("{:#?}", validated_schema);
    }

    #[test]
    fn test_valid_composite_sizes() {
        let path = Path::new("./examples/example-schema.xml");
        let merged_content = resolve_xml_includes(path).unwrap();
        let schema = MessageSchema::load_from_string(&merged_content).unwrap();
        let validated_schema = schema.validate().unwrap();
        let type_map = validated_schema.types;

        assert_eq!(
            type_map
                .find_type("Engine")
                .unwrap()
                .size(&type_map)
                .unwrap(),
            10
        );
        assert_eq!(
            type_map
                .find_type("Booster")
                .unwrap()
                .size(&type_map)
                .unwrap(),
            2
        );
    }
}
