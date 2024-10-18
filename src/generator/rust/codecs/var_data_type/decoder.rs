use crate::generator::rust::codecs::var_data_type::repr_type_metadata;
use crate::generator::rust::constants::DECODER_FILE_NAME;
use crate::generator::write_file;
use crate::models::types::variable_data_type::VariableDataType;
use crate::models::TypeMap;
use anyhow::Result;
use convert_case::{Case, Casing};
use genco::prelude::*;
use std::path::Path;

pub struct RustVariableDataDecoderGenerator<'a> {
    pub(crate) config: &'a rust::Config,
    pub(crate) path: &'a Path,
    pub(crate) types: &'a TypeMap,
    pub(crate) package: &'a str,
}

impl RustVariableDataDecoderGenerator<'_> {
    pub fn write_decoder(&self, var_data: &VariableDataType) -> Result<()> {
        let decoder_tokens: Tokens<Rust> = quote!($(self.generate_var_data_decoder(var_data)?));

        write_file(
            &self.path.join(DECODER_FILE_NAME),
            self.config,
            decoder_tokens,
        )?;

        Ok(())
    }

    pub fn generate_var_data_decoder(
        &self,
        var_data: &VariableDataType,
    ) -> Result<impl FormatInto<Rust>> {
        let name = var_data.name.as_str();
        let decoder_name = format!("{}Decoder", name.to_case(Case::UpperCamel));
        let repr_type = var_data.repr_type(&self.types.composite_types)?;

        let (length_type_metadata, value_type_metadata) =
            repr_type_metadata(name, repr_type, self.types)?;

        let length_type_primitive = length_type_metadata.lang_type;
        let length_type_size = length_type_metadata.type_size;

        let value_type_primitive = value_type_metadata.lang_type;
        let value_type_size = value_type_metadata.type_size;

        Ok(quote! {
            use crate::error::*;
            use crate::$(self.package)::composites::*;
            use crate::$(self.package)::decoder::*;
            use std::convert::TryFrom;

            #[derive(Debug, Default)]
            pub struct $(&decoder_name)<'a> {
                buffer: ReadBuf<'a>,
                index: usize
            }

            impl Iterator for $(&decoder_name)<'_> {
                type Item = $(value_type_primitive.name);

                fn next(&mut self) -> Option<Self::Item> {
                    let index = self.index;

                    let result = if index < self.length() {
                        self.get_at(index).ok()
                    } else {
                        None
                    };

                    self.index += 1;

                    result
                }
            }

            impl $(&decoder_name)<'_> {
                #[inline]
                pub fn length(&self) -> usize {
                    // TODO: Max length check
                    self.buffer.get_$(length_type_primitive)_at(0).unwrap_or(0) as usize
                }

                #[inline]
                pub fn size(&self) -> usize {
                    self.length() * $value_type_size + $length_type_size
                }

                #[inline]
                pub fn get_at(&self, index: usize) -> Result<$(&value_type_primitive)> {
                    let offset = index * $value_type_size + $length_type_size;

                    self.buffer.get_$(&value_type_primitive)_at(offset)
                }

                $(if value_type_primitive.name == "u8" {
                    #[inline]
                    pub fn get_slice_at(&self, index: usize, length: usize) -> Result<&[u8]> {
                        let offset = index * $value_type_size + $length_type_size;

                        self.buffer.get_slice_at(offset, length)
                    }
                })
            }

            impl<'a> From<ReadBuf<'a>> for $(&decoder_name)<'a> {
                fn from(buffer: ReadBuf<'a>) -> Self {
                    Self { buffer, index: 0 }
                }
            }
        })
    }
}
