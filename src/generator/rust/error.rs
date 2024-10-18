use crate::generator::rust::RustGenerator;
use crate::generator::write_file;
use anyhow::Result;
use genco::prelude::*;

impl RustGenerator {
    pub fn write_error_module(&self) -> Result<()> {
        let error_module_content: Tokens<Rust> = quote! {
            use std::fmt::Debug;
            use std::str::Utf8Error;
            use std::string::FromUtf8Error;
            use thiserror::Error;

            #[derive(Error, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
            pub enum SbeError {
                #[error("invalid ascii string: {0}")]
                InvalidStringValue(String),
                #[error("invalid enum value for '{type_name}': {value}")]
                InvalidEnumValue {
                    type_name: &'static str,
                    value: String,
                },
                #[error("value out of bounds for field '{field_name}': {message}")]
                ValueOutOfBounds {
                    field_name: &'static str,
                    message: String,
                },
                #[error("wrong slice size: {0}")]
                WrongSliceSize(String),
                #[error("missing group size: {0}")]
                MissingGroupSize(&'static str),
                #[error("missing var data size: {0}")]
                MissingVarDataSize(&'static str),
                #[error("group out of bounds: {0}")]
                GroupOutOfBounds(&'static str),
                #[error("var data out of bounds: {0}")]
                VarDataOutOfBounds(&'static str),
                #[error("received message had wrong type: {0}, expected {1}")]
                WrongMessageType(u16, u16),
                #[error("codec out of bounds: {0} > {1}")]
                CodecOutOfBounds(usize, usize),
            }

            impl From<Utf8Error> for SbeError {
                fn from(error: Utf8Error) -> Self {
                    Self::InvalidStringValue(error.to_string())
                }
            }

            impl From<FromUtf8Error> for SbeError {
                fn from(error: FromUtf8Error) -> Self {
                    Self::InvalidStringValue(error.to_string())
                }
            }

            pub type Result<T> = std::result::Result<T, SbeError>;
        };

        write_file(
            &self.path.join("src/error.rs"),
            &self.config,
            error_module_content,
        )
    }
}
