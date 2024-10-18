use crate::generator::common::field_groups;
use crate::generator::rust::codecs::composite_type::encoder::RustCompositeEncoderGenerator;
use crate::generator::rust::codecs::group_type::encoder::RustGroupEncoderGenerator;
use crate::generator::rust::constants::ENCODER_FILE_NAME;
use crate::generator::write_file;
use crate::models::message::MessageType;
use crate::models::types::{SizedEncoded, Type};
use crate::models::TypeMap;
use anyhow::Result;
use convert_case::{Case, Casing};
use genco::lang::{rust, Rust};
use genco::prelude::*;
use genco::tokens::FormatInto;
use std::path::Path;

pub struct RustMessageEncoderGenerator<'a> {
    pub(crate) config: &'a rust::Config,
    pub(crate) path: &'a Path,
    pub(crate) types: &'a TypeMap,
    pub(crate) package: &'a str,
}

impl RustMessageEncoderGenerator<'_> {
    pub fn write_encoder(&self, message: &MessageType) -> Result<()> {
        let encoder_tokens: Tokens<Rust> = quote!($(self.generate_message_encoder(message)?));

        write_file(
            &self.path.join(ENCODER_FILE_NAME),
            self.config,
            encoder_tokens,
        )?;

        Ok(())
    }

    fn generate_message_encoder(&self, message: &MessageType) -> Result<impl FormatInto<Rust>> {
        let name = message.name.as_str();
        let encoder_name = format!("{}Encoder", name.to_case(Case::UpperCamel));
        let (fields, groups, var_data) = field_groups(&message.fields);

        let group_encoder_gen = RustGroupEncoderGenerator {
            config: self.config,
            path: self.path,
            types: self.types,
            package: self.package,
        };

        let offset_prefix = quote!();
        let mut header_offset = 0;

        let header_tokens = self.generate_header(&mut header_offset)?;

        let mut field_offset = header_offset;

        let field_tokens =
            group_encoder_gen.generate_fields(&fields, &offset_prefix, &mut field_offset)?;

        let offset_tokens = quote!($field_offset);

        let group_tokens = group_encoder_gen.generate_groups(&groups, &offset_tokens)?;
        let var_data_tokens =
            group_encoder_gen.generate_var_data_fields(&var_data, &groups, &offset_tokens)?;

        Ok(quote! {
            use crate::error::*;
            use crate::$(self.package)::composites::*;
            use crate::$(self.package)::encoder::*;
            use crate::$(self.package)::enums::*;
            use crate::$(self.package)::groups::*;
            use crate::$(self.package)::sets::*;
            use crate::$(self.package)::var_data::*;
            use std::convert::TryFrom;
            use super::$(name.to_case(Case::ScreamingSnake))_ID;
            use crate::$(self.package)::{SCHEMA_ID, SCHEMA_VERSION};

            #[derive(Debug)]
            pub struct $(&encoder_name)<'a> {
                buffer: WriteBuf<'a>,
                $(for group in &groups {
                    $(group.name.to_case(Case::Snake))_size: Option<usize>,
                    $['\r']
                })
                $(for var in &var_data {
                    $(var.name.to_case(Case::Snake))_size: Option<usize>,
                    $['\r']
                })
            }

            impl $(&encoder_name)<'_> {
                #[inline]
                pub const fn id() -> u16 {
                    $(name.to_case(Case::ScreamingSnake))_ID
                }

                #[inline]
                pub const fn block_length() -> usize {
                    $(field_offset - header_offset)
                }

                #[inline]
                pub fn size(&self) -> Option<usize> {
                    let header_size = $(header_offset);
                    let fields_size = Self::block_length();
                    let groups_size = 0$(for group in &groups => $[' ']+ self.$(group.name.to_case(Case::Snake))_size?);
                    let var_data_size = 0$(for var in &var_data => $[' ']+ self.$(var.name.to_case(Case::Snake))_size?);

                    Some(header_size + fields_size + groups_size + var_data_size)
                }

                $header_tokens

                $field_tokens

                $group_tokens

                $var_data_tokens
            }

            impl<'a> TryFrom<WriteBuf<'a>> for $(&encoder_name)<'a> {
                type Error = SbeError;

                fn try_from(buffer: WriteBuf<'a>) -> Result<Self> {
                    let mut msg = Self {
                        buffer,
                        $(for group in &groups => $['\r']$(group.name.to_case(Case::Snake))_size: None,)
                        $(for var in &var_data => $['\r']$(var.name.to_case(Case::Snake))_size: None,)
                    };

                    msg.message_header_encoder(|encoder| {
                        encoder.block_length(Self::block_length() as _)?;
                        encoder.template_id(Self::id())?;
                        encoder.schema_id(SCHEMA_ID)?;
                        encoder.version(SCHEMA_VERSION)
                    })?;

                    Ok(msg)
                }
            }
        })
    }

    fn generate_header(&self, offset: &mut usize) -> Result<impl FormatInto<Rust>> {
        let composite_encoder_gen = RustCompositeEncoderGenerator {
            config: self.config,
            path: self.path,
            types: self.types,
            package: self.package,
        };

        let message_header_type = Type::Composite(self.types.header_type.clone());

        let header_encoder = composite_encoder_gen.generate_encoder_field(
            "message_header",
            &message_header_type,
            quote!($(*offset)),
        )?;
        *offset += message_header_type.size(self.types)?;

        Ok(header_encoder)
    }
}
