use crate::models::constants::*;
use crate::models::types::encoded_data_type::EncodedDataType;
use crate::models::types::SizedEncoded;
use crate::models::TypeMap;
use anyhow::{anyhow, Result};
use genco::lang::Lang;
use genco::tokens::FormatInto;
use genco::Tokens;
use std::collections::HashMap;
use std::marker::PhantomData;
use std::str::FromStr;

#[derive(PartialEq, Debug, Clone)]
pub enum NativeType {
    Char,
    UInt8,
    UInt16,
    UInt32,
    UInt64,
    Int8,
    Int16,
    Int32,
    Int64,
    Float,
    Double,
    Reference(String),
}

impl NativeType {
    pub fn null(&self) -> Result<String> {
        Ok(match self {
            NativeType::Char => CHAR_NULL.to_string(),
            NativeType::UInt8 => U8_NULL.to_string(),
            NativeType::UInt16 => U16_NULL.to_string(),
            NativeType::UInt32 => U32_NULL.to_string(),
            NativeType::UInt64 => U64_NULL.to_string(),
            NativeType::Int8 => I8_NULL.to_string(),
            NativeType::Int16 => I16_NULL.to_string(),
            NativeType::Int32 => I32_NULL.to_string(),
            NativeType::Int64 => I64_NULL.to_string(),
            NativeType::Float => F32_NULL.to_string(),
            NativeType::Double => F64_NULL.to_string(),
            NativeType::Reference(_) => return Err(anyhow!("Cannot get null for reference type")),
        })
    }

    pub fn is_hashable(&self) -> bool {
        match self {
            NativeType::Char
            | NativeType::UInt8
            | NativeType::UInt16
            | NativeType::UInt32
            | NativeType::UInt64
            | NativeType::Int8
            | NativeType::Int16
            | NativeType::Int32
            | NativeType::Int64 => true,
            NativeType::Float | NativeType::Double | NativeType::Reference(_) => false,
        }
    }
}

impl SizedEncoded for NativeType {
    fn size(&self, types: &TypeMap) -> Result<usize> {
        Ok(match self {
            NativeType::Char => CHAR_SIZE,
            NativeType::UInt8 => U8_SIZE,
            NativeType::UInt16 => U16_SIZE,
            NativeType::UInt32 => U32_SIZE,
            NativeType::UInt64 => U64_SIZE,
            NativeType::Int8 => I8_SIZE,
            NativeType::Int16 => I16_SIZE,
            NativeType::Int32 => I32_SIZE,
            NativeType::Int64 => I64_SIZE,
            NativeType::Float => F32_SIZE,
            NativeType::Double => F64_SIZE,
            NativeType::Reference(name) => types
                .find_type(name)
                .ok_or(anyhow!("Unknown type {}", name))
                .and_then(|t| t.size(types))?,
        })
    }
}

impl FromStr for NativeType {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        Ok(match s {
            "char" => NativeType::Char,
            "uint8" => NativeType::UInt8,
            "uint16" => NativeType::UInt16,
            "uint32" => NativeType::UInt32,
            "uint64" => NativeType::UInt64,
            "int8" => NativeType::Int8,
            "int16" => NativeType::Int16,
            "int32" => NativeType::Int32,
            "int64" => NativeType::Int64,
            "float" => NativeType::Float,
            "double" => NativeType::Double,
            _ => NativeType::Reference(s.to_owned()),
        })
    }
}

#[derive(Debug, Copy, Clone)]
pub struct LanguagePrimitive<L: Lang> {
    pub name: &'static str,
    marker: PhantomData<L>,
}

impl<L: Lang> LanguagePrimitive<L> {
    pub fn new(name: &'static str) -> Self {
        Self {
            name,
            marker: PhantomData,
        }
    }
}

impl<L: Lang> FormatInto<L> for LanguagePrimitive<L> {
    fn format_into(self, tokens: &mut Tokens<L>) {
        tokens.append(self.name);
    }
}

impl<L: Lang> FormatInto<L> for &LanguagePrimitive<L> {
    fn format_into(self, tokens: &mut Tokens<L>) {
        tokens.append(self.name);
    }
}

pub trait ResolvableType {
    fn resolved(&self, encoded_types: &HashMap<String, EncodedDataType>) -> Result<Self>
    where
        Self: Sized;
}

pub trait PrimitiveConvertible<L: Lang>: ResolvableType {
    fn lang_primitive(
        &self,
        encoded_types: &HashMap<String, EncodedDataType>,
    ) -> Result<LanguagePrimitive<L>>;
}

impl ResolvableType for NativeType {
    fn resolved(&self, encoded_types: &HashMap<String, EncodedDataType>) -> Result<Self> {
        match self {
            NativeType::Reference(type_name) => encoded_types
                .get(type_name)
                .ok_or(anyhow!("Could not find encoded data type '{type_name}'"))?
                .primitive_type
                .resolved(encoded_types),
            _ => Ok(self.clone()),
        }
    }
}
