use crate::generator::common::field_groups;
use crate::generator::rust::codecs::composite_type::decoder::RustCompositeDecoderGenerator;
use crate::generator::rust::codecs::group_type::dimension_type;
use crate::generator::rust::constants::DECODER_FILE_NAME;
use crate::generator::write_file;
use crate::models::types::field_type::FieldType;
use crate::models::types::group_type::GroupType;
use crate::models::types::variable_data_type::VariableDataType;
use crate::models::types::{Presence, SizedEncoded, Type};
use crate::models::TypeMap;
use anyhow::{anyhow, Result};
use convert_case::{Case, Casing};
use genco::prelude::*;
use std::path::Path;

pub struct RustGroupDecoderGenerator<'a> {
    pub(crate) config: &'a rust::Config,
    pub(crate) path: &'a Path,
    pub(crate) types: &'a TypeMap,
    pub(crate) package: &'a str,
}

impl RustGroupDecoderGenerator<'_> {
    pub fn write_decoder(&self, group: &GroupType) -> Result<()> {
        let decoder_tokens: Tokens<Rust> = quote!($(self.generate_group_decoder(group)?));

        write_file(
            &self.path.join(DECODER_FILE_NAME),
            self.config,
            decoder_tokens,
        )?;

        Ok(())
    }

    pub fn generate_group_decoder(&self, group: &GroupType) -> Result<impl FormatInto<Rust>> {
        let name = group.name.as_str();
        let decoder_name = format!("{}Decoder", name.to_case(Case::UpperCamel));
        let (fields, groups, var_data) = field_groups(&group.fields);

        let mut offset = 0;

        let group_sizes = groups
            .iter()
            .map(|prev_group| {
                let prev_group_name = prev_group.name.to_case(Case::Snake);
                quote!($[' ']+ self.$(&prev_group_name)_size.take().ok_or(SbeError::MissingGroupSize($(quoted(prev_group_name))))?)
            });

        let var_data_field_sizes = var_data
            .iter()
            .map(|var_data_field| {
                let var_data_field_name = var_data_field.name.to_case(Case::Snake);
                quote!($[' ']+ self.$(&var_data_field_name)_size.take().ok_or(SbeError::MissingVarDataSize($(quoted(var_data_field_name))))?)
            });

        let dimension_type = dimension_type(group, &self.types.composite_types)?;
        let dimension_type_size = dimension_type.size(self.types)?;
        let dimension_type_decoder_name =
            format!("{}Decoder", dimension_type.name.to_case(Case::UpperCamel));

        Ok(quote! {
            use crate::error::*;
            use crate::$(self.package)::composites::*;
            use crate::$(self.package)::decoder::*;
            use crate::$(self.package)::enums::*;
            use crate::$(self.package)::groups::*;
            use crate::$(self.package)::sets::*;
            use crate::$(self.package)::var_data::*;
            use std::convert::TryFrom;

            #[derive(Debug)]
            pub struct $(&decoder_name)<'a> {
                buffer: ReadBuf<'a>,
                block_length: usize,
                num_in_group: usize,
                index: usize,
                size: usize,
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
                pub fn size(&self) -> Option<usize> {
                    if self.index == self.num_in_group {
                        Some(self.size)
                    } else {
                        None
                    }
                }

                #[inline]
                pub fn advance(&mut self) -> Result<()> {
                    if self.index >= self.num_in_group {
                        return Err(SbeError::GroupOutOfBounds($(quoted(name))));
                    }

                    let fields_size = self.block_length;
                    let groups_size = 0$(for group_size in group_sizes => $group_size);
                    let var_data_size = 0$(for var_data_field_size in var_data_field_sizes => $var_data_field_size);
                    let advancement = fields_size + groups_size + var_data_size;

                    self.index += 1;
                    self.size += advancement;
                    self.buffer = self.buffer.split_at(advancement)?.1;

                    Ok(())
                }

                $(self.generate_metadata()?)

                $(self.generate_fields(&fields, &mut offset)?)

                $(self.generate_groups(&groups, offset)?)

                $(self.generate_var_data_fields(&var_data, &groups, offset)?)
            }

            impl<'a> TryFrom<ReadBuf<'a>> for $(&decoder_name)<'a> {
                type Error = SbeError;

                fn try_from(buffer: ReadBuf<'a>) -> Result<Self> {
                    let (metadata_buffer, buffer) = buffer.split_at($dimension_type_size)?;
                    let metadata = $(&dimension_type_decoder_name)::from(metadata_buffer);
                    Ok(Self {
                        buffer,
                        block_length: metadata.block_length()? as usize,
                        num_in_group: metadata.num_in_group()? as usize,
                        index: 0,
                        size: $dimension_type_size,
                        $(for group in &groups => $['\r']$(group.name.to_case(Case::Snake))_size: None,)
                        $(for var in &var_data => $['\r']$(var.name.to_case(Case::Snake))_size: None,)
                    })
                }
            }
        })
    }

    fn generate_metadata(&self) -> Result<impl FormatInto<Rust>> {
        let metadata_decoder_tokens = quote! {
            #[inline]
            pub fn block_length(&self) -> usize {
                self.block_length
            }

            #[inline]
            pub fn num_in_group(&self) -> usize {
                self.num_in_group
            }
        };

        Ok(metadata_decoder_tokens)
    }

    pub fn generate_fields(
        &self,
        fields: &[&FieldType],
        offset: &mut usize,
    ) -> Result<impl FormatInto<Rust>> {
        let mut decoder_fields: Tokens<Rust> = quote!();
        let composite_decoder_gen = RustCompositeDecoderGenerator {
            config: self.config,
            path: self.path,
            types: self.types,
            package: self.package,
        };

        for field in fields {
            decoder_fields.line();
            decoder_fields.append(self.generate_field(offset, &composite_decoder_gen, field)?);
        }

        Ok(decoder_fields)
    }

    fn generate_field(
        &self,
        offset: &mut usize,
        composite_decoder_gen: &RustCompositeDecoderGenerator,
        field_type: &FieldType,
    ) -> Result<impl FormatInto<Rust>> {
        let repr_type = field_type.to_type(self.types)?;

        let field_tokens = if matches!(field_type.presence, Presence::Constant) {
            quote!($(self.generate_constant_enum(field_type, repr_type)?))
        } else {
            let tokens = quote!($(composite_decoder_gen.generate_decoder_field(&field_type.name, &repr_type, *offset)?));
            *offset += repr_type.size(self.types)?;

            tokens
        };

        Ok(field_tokens)
    }

    fn generate_constant_enum(
        &self,
        field_type: &FieldType,
        repr_type: Type,
    ) -> Result<impl FormatInto<Rust>> {
        let field_name = field_type.name.to_case(Case::Snake);
        let enum_type = match repr_type {
            Type::Enum(enum_type) => enum_type,
            _ => return Err(anyhow!("Constant field {} is not an enum", field_type.name)),
        };
        let enum_type_name = enum_type.name.to_case(Case::UpperCamel);

        let default_value = field_type.value_ref.as_ref().ok_or(anyhow!(
            "Constant field {} has no ref value",
            field_type.name
        ))?;

        let default_value = default_value.split('.').take(2).collect::<Vec<_>>()[1];
        let default_value = enum_type
            .values
            .iter()
            .find(|value| value.name == default_value)
            .ok_or(anyhow!(
                "Constant field {} has no value {}",
                field_type.name,
                default_value
            ))?;

        Ok(quote! {
            #[inline]
            pub fn $(&field_name)(&self) -> $(&enum_type_name) {
                $(&enum_type_name)::$(&default_value.name)
            }
        })
    }

    pub fn generate_groups(
        &self,
        groups: &[&GroupType],
        offset: usize,
    ) -> Result<impl FormatInto<Rust>> {
        let mut decoder_fields: Tokens<Rust> = quote!();

        for group_idx in 0..groups.len() {
            decoder_fields.line();
            decoder_fields.append(self.generate_group(
                groups[group_idx],
                &groups[..group_idx],
                offset,
            )?);
        }

        Ok(decoder_fields)
    }

    fn generate_group(
        &self,
        group: &GroupType,
        previous_groups: &[&GroupType],
        offset: usize,
    ) -> Result<impl FormatInto<Rust>> {
        let group_name = group.name.as_str();
        let func_name = group_name.to_case(Case::Snake);
        let decoder_name = format!("{}Decoder", group_name.to_case(Case::UpperCamel));

        let prev_group_sizes = previous_groups
            .iter()
            .map(|prev_group| {
                let prev_group_name = prev_group.name.to_case(Case::Snake);
                quote!($[' ']+ self.$(&prev_group_name)_size.ok_or(SbeError::MissingGroupSize($(quoted(prev_group_name))))?)
            });

        Ok(quote! {
            #[inline]
            pub fn $(&func_name)_decoder<T>(&mut self, action: impl FnOnce(&mut $(&decoder_name)) -> Result<T>) -> Result<T> {
                let offset = $offset$(for prev_group_size in prev_group_sizes => $prev_group_size);
                let buffer = self.buffer.split_at(offset)?.1;

                let mut decoder = buffer.try_into()?;

                let result = action(&mut decoder)?;
                self.$(func_name)_size = decoder.size();

                Ok(result)
            }
        })
    }

    pub fn generate_var_data_fields(
        &self,
        var_data_fields: &[&VariableDataType],
        groups: &[&GroupType],
        offset: usize,
    ) -> Result<impl FormatInto<Rust>> {
        let mut decoder_fields: Tokens<Rust> = quote!();

        for var_data_field_idx in 0..var_data_fields.len() {
            decoder_fields.line();
            decoder_fields.append(self.generate_var_data_field(
                var_data_fields[var_data_field_idx],
                &var_data_fields[..var_data_field_idx],
                groups,
                offset,
            )?);
        }

        Ok(decoder_fields)
    }

    fn generate_var_data_field(
        &self,
        var_data_field: &VariableDataType,
        previous_var_data_fields: &[&VariableDataType],
        groups: &[&GroupType],
        offset: usize,
    ) -> Result<impl FormatInto<Rust>> {
        let name = var_data_field.name.as_str();
        let func_name = name.to_case(Case::Snake);
        let decoder_name = format!("{}Decoder", name.to_case(Case::UpperCamel));

        let group_sizes = groups
            .iter()
            .map(|prev_group| {
                let prev_group_name = prev_group.name.to_case(Case::Snake);
                quote!($[' ']+ self.$(&prev_group_name)_size.ok_or(SbeError::MissingGroupSize($(quoted(prev_group_name))))?)
            });

        let group_size = quote!($offset$(for group_size in group_sizes => $group_size));

        let var_data_field_sizes = previous_var_data_fields
            .iter()
            .map(|prev_var_data_field| {
                let prev_var_data_field_name = prev_var_data_field.name.to_case(Case::Snake);
                quote!($[' ']+ self.$(&prev_var_data_field_name)_size.ok_or(SbeError::MissingVarDataSize($(quoted(prev_var_data_field_name))))?)
            });

        Ok(quote! {
            #[inline]
            pub fn $(&func_name)_decoder<T>(&mut self, action: impl FnOnce(&mut $(&decoder_name)) -> Result<T>) -> Result<T> {
                let offset = $group_size$(for var_data_field_size in var_data_field_sizes => $var_data_field_size);
                let buffer = self.buffer.split_at(offset)?.1;

                let mut decoder = buffer.into();

                let result = action(&mut decoder)?;
                self.$(func_name)_size = Some(decoder.size());

                Ok(result)
            }
        })
    }
}
