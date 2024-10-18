use crate::generator::common::field_groups;
use crate::generator::rust::codecs::composite_type::decoder::RustCompositeDecoderGenerator;
use crate::generator::rust::codecs::group_type::decoder::RustGroupDecoderGenerator;
use crate::generator::rust::constants::DECODER_FILE_NAME;
use crate::generator::write_file;
use crate::models::message::MessageType;
use crate::models::types::{SizedEncoded, Type};
use crate::models::TypeMap;
use anyhow::Result;
use convert_case::{Case, Casing};
use genco::prelude::*;
use genco::tokens::FormatInto;
use std::path::Path;

pub struct RustMessageDecoderGenerator<'a> {
    pub(crate) config: &'a rust::Config,
    pub(crate) path: &'a Path,
    pub(crate) types: &'a TypeMap,
    pub(crate) package: &'a str,
}

impl RustMessageDecoderGenerator<'_> {
    pub fn write_decoder(&self, message: &MessageType) -> Result<()> {
        let decoder_tokens: Tokens<Rust> = quote!($(self.generate_message_decoder(message)?));

        write_file(
            &self.path.join(DECODER_FILE_NAME),
            self.config,
            decoder_tokens,
        )?;

        Ok(())
    }

    fn generate_message_decoder(&self, message: &MessageType) -> Result<impl FormatInto<Rust>> {
        let name = message.name.as_str();
        let decoder_name = format!("{}Decoder", name.to_case(Case::UpperCamel));
        let (fields, groups, var_data) = field_groups(&message.fields);

        let group_decoder_gen = RustGroupDecoderGenerator {
            config: self.config,
            path: self.path,
            types: self.types,
            package: self.package,
        };

        let fields_size: usize = fields
            .iter()
            .map(|field| {
                let repr_type = field.to_type(self.types)?;
                repr_type.size(self.types)
            })
            .sum::<Result<usize>>()?;

        let mut offset = 0;

        Ok(quote! {
            use crate::error::*;
            use crate::$(self.package)::composites::*;
            use crate::$(self.package)::decoder::*;
            use crate::$(self.package)::enums::*;
            use crate::$(self.package)::groups::*;
            use crate::$(self.package)::sets::*;
            use crate::$(self.package)::var_data::*;
            use std::convert::TryFrom;
            use super::$(name.to_case(Case::ScreamingSnake))_ID;

            #[derive(Debug)]
            pub struct $(&decoder_name)<'a> {
                buffer: ReadBuf<'a>,
                $(for group in &groups {
                    $(group.name.to_case(Case::Snake))_size: Option<usize>,
                    $['\r']
                })
                $(for var in &var_data {
                    $(var.name.to_case(Case::Snake))_size: Option<usize>,
                    $['\r']
                })
            }

            impl $(&decoder_name)<'_> {
                #[inline]
                pub const fn id() -> u16 {
                    $(name.to_case(Case::ScreamingSnake))_ID
                }

                #[inline]
                pub fn size(&self) -> Option<usize> {
                    Some($fields_size
                    $(for group in &groups {
                        $[' ']+ self.$(group.name.to_case(Case::Snake))_size?
                    })
                    $(for var in &var_data {
                        $[' ']+ self.$(var.name.to_case(Case::Snake))_size?
                    }))
                }

                $(self.generate_header(&mut offset)?)

                $(group_decoder_gen.generate_fields(&fields, &mut offset)?)

                $(group_decoder_gen.generate_groups(&groups, offset)?)

                $(group_decoder_gen.generate_var_data_fields(&var_data, &groups, offset)?)
            }

            impl<'a> TryFrom<ReadBuf<'a>> for $(&decoder_name)<'a> {
                type Error = SbeError;

                fn try_from(buffer: ReadBuf<'a>) -> Result<Self> {
                    let msg = Self {
                        buffer,
                        $(for group in &groups => $['\r']$(group.name.to_case(Case::Snake))_size: None,)
                        $(for var in &var_data => $['\r']$(var.name.to_case(Case::Snake))_size: None,)
                    };

                    let current_message_id = msg.message_header_decoder(|decoder| decoder.template_id())?;
                    if current_message_id != Self::id() {
                        return Err(SbeError::WrongMessageType(current_message_id, Self::id()));
                    }

                    Ok(msg)
                }
            }
        })
    }

    fn generate_header(&self, offset: &mut usize) -> Result<impl FormatInto<Rust>> {
        let composite_decoder_gen = RustCompositeDecoderGenerator {
            config: self.config,
            path: self.path,
            types: self.types,
            package: self.package,
        };

        let message_header_type = Type::Composite(self.types.header_type.clone());

        let header_decoder = composite_decoder_gen.generate_decoder_field(
            "message_header",
            &message_header_type,
            *offset,
        )?;
        *offset += message_header_type.size(self.types)?;

        Ok(header_decoder)
    }
}
