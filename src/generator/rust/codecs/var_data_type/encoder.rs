use crate::generator::rust::codecs::var_data_type::repr_type_metadata;
use crate::generator::rust::constants::ENCODER_FILE_NAME;
use crate::generator::write_file;
use crate::models::types::variable_data_type::VariableDataType;
use crate::models::types::CharacterEncoding;
use crate::models::TypeMap;
use anyhow::Result;
use convert_case::{Case, Casing};
use genco::prelude::*;
use std::path::Path;

pub struct RustVariableDataEncoderGenerator<'a> {
    pub(crate) config: &'a rust::Config,
    pub(crate) path: &'a Path,
    pub(crate) types: &'a TypeMap,
    pub(crate) package: &'a str,
}

impl RustVariableDataEncoderGenerator<'_> {
    pub fn write_encoder(&self, var_data: &VariableDataType) -> Result<()> {
        let decoder_tokens: Tokens<Rust> = quote!($(self.generate_var_data_encoder(var_data)?));

        write_file(
            &self.path.join(ENCODER_FILE_NAME),
            self.config,
            decoder_tokens,
        )?;

        Ok(())
    }

    pub fn generate_var_data_encoder(
        &self,
        var_data: &VariableDataType,
    ) -> Result<impl FormatInto<Rust>> {
        let name = var_data.name.as_str();
        let encoder_name = format!("{}Encoder", name.to_case(Case::UpperCamel));
        let repr_type = var_data.repr_type(&self.types.composite_types)?;

        let (length_type_metadata, value_type_metadata) =
            repr_type_metadata(name, repr_type, self.types)?;

        let length_type_primitive = length_type_metadata.lang_type;
        let length_type_size = length_type_metadata.type_size;

        let value_type_primitive = value_type_metadata.lang_type;
        let value_type_encoding = value_type_metadata.encoding;
        let value_type_size = value_type_metadata.type_size;

        Ok(quote!(
            use crate::error::*;
            use crate::$(self.package)::composites::*;
            use crate::$(self.package)::encoder::*;
            use std::convert::TryFrom;
            use std::ops::Index;

            #[derive(Debug)]
            pub struct $(&encoder_name)<'a> {
                buffer: WriteBuf<'a>,
                length: $(&length_type_primitive),
            }

            impl $(&encoder_name)<'_> {
                #[inline]
                pub fn size(&self) -> usize {
                    (self.length * $value_type_size + $length_type_size) as usize
                }

                #[inline]
                pub fn put_at(&mut self, index: $(&length_type_primitive), value: $(&value_type_primitive)) -> Result<()> {
                    $(if let Some(CharacterEncoding::Ascii) = value_type_encoding {
                        if !value.is_ascii() {
                            return Err(SbeError::InvalidStringValue(value.to_string()));
                        }
                    })

                    if index >= self.length {
                        self.length = index.checked_add(1).ok_or(SbeError::VarDataOutOfBounds($(quoted(name))))?;
                    }

                    let offset = index as usize * $value_type_size + $length_type_size;
                    self.buffer.put_$(&value_type_primitive)_at(offset, value)
                }

                $(if value_type_primitive.name == "u8" {
                    #[inline]
                    pub fn put_slice_at(&mut self, index: $(&length_type_primitive), value: &[u8]) -> Result<()> {
                        $(if let Some(CharacterEncoding::Ascii) = value_type_encoding {
                            if !value.is_ascii() {
                                return Err(SbeError::InvalidStringValue(std::str::from_utf8(value)?.to_string()));
                            }
                        })

                        let data_end = index as usize + value.len();

                        if data_end >= self.length as usize {
                            self.length = data_end.try_into().map_err(|_| SbeError::VarDataOutOfBounds($(quoted(name))))?;
                        }

                        let offset = index as usize * $value_type_size + $length_type_size;
                        self.buffer.put_bytes_at(offset, value)
                    }
                })

                #[inline]
                pub fn finalize(mut self) -> Result<()> {
                    self.buffer.put_$(length_type_primitive)_at(0, self.length)
                }
            }

            impl<'a> TryFrom<WriteBuf<'a>> for $(&encoder_name)<'a> {
                type Error = SbeError;

                fn try_from(buffer: WriteBuf<'a>) -> Result<Self> {
                    Ok(Self {
                        buffer,
                        length: 0,
                    })
                }
            }
        ))
    }
}
