use crate::models::types::encoded_data_type::EncodedDataType;
use crate::models::types::field_type::FieldType;
use crate::models::types::group_type::GroupType;
use crate::models::types::primitive_type::{
    LanguagePrimitive, NativeType, PrimitiveConvertible, ResolvableType,
};
use crate::models::types::variable_data_type::VariableDataType;
use crate::models::types::{CharacterEncoding, MessageField, SizedEncoded};
use crate::models::TypeMap;
use anyhow::Result;
use convert_case::{Case, Casing};
use genco::lang::{Lang, Rust};

#[derive(Debug)]
pub struct FieldMetadata<L: Lang> {
    pub field_name: String,
    pub field_primitive_type: NativeType,
    pub type_size: usize,
    pub field_length: usize,
    pub lang_type: LanguagePrimitive<L>,
    pub encoding: Option<CharacterEncoding>,
}

impl FieldMetadata<Rust> {
    pub fn from(field_name: &str, encoded_type: &EncodedDataType, types: &TypeMap) -> Result<Self> {
        let field_name = field_name.to_case(Case::Snake);
        let field_primitive_type = encoded_type.primitive_type.resolved(&types.encoded_types)?;
        let field_size = encoded_type.size(types)?;
        let type_size = encoded_type.primitive_type.size(types)?;
        let field_length = field_size / type_size;
        let lang_type = field_primitive_type.lang_primitive(&types.encoded_types)?;
        let encoding = encoded_type.character_encoding;

        Ok(Self {
            field_name,
            field_primitive_type,
            type_size,
            field_length,
            lang_type,
            encoding,
        })
    }
}

pub fn variable_value_type(
    field_primitive_type: &NativeType,
    rust_type: &str,
    field_length: usize,
) -> String {
    match (field_primitive_type, field_length) {
        (NativeType::Char, 2..) => "String".to_owned(),
        (_, 2..) => format!("[{}; {}]", rust_type, field_length),
        (_, _) => rust_type.to_owned(),
    }
}

pub fn field_groups(
    group_type_fields: &[MessageField],
) -> (Vec<&FieldType>, Vec<&GroupType>, Vec<&VariableDataType>) {
    let fields = group_type_fields
        .iter()
        .filter_map(|field| match field {
            MessageField::Field(field_type) => Some(field_type),
            _ => None,
        })
        .collect::<Vec<&FieldType>>();

    let groups = group_type_fields
        .iter()
        .filter_map(|field| match field {
            MessageField::Group(group_type) => Some(group_type),
            _ => None,
        })
        .collect::<Vec<&GroupType>>();

    let var_data = group_type_fields
        .iter()
        .filter_map(|field| match field {
            MessageField::VariableData(var_data_type) => Some(var_data_type),
            _ => None,
        })
        .collect::<Vec<&VariableDataType>>();
    (fields, groups, var_data)
}
