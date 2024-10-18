use crate::generator::common::field_groups;
use crate::generator::rust::codecs::composite_type::encoder::RustCompositeEncoderGenerator;
use crate::generator::rust::codecs::group_type::dimension_type;
use crate::generator::rust::constants::ENCODER_FILE_NAME;
use crate::generator::write_file;
use crate::models::types::field_type::FieldType;
use crate::models::types::group_type::GroupType;
use crate::models::types::primitive_type::PrimitiveConvertible;
use crate::models::types::variable_data_type::VariableDataType;
use crate::models::types::{Presence, SizedEncoded, Type};
use crate::models::TypeMap;
use anyhow::{anyhow, Result};
use convert_case::{Case, Casing};
use genco::prelude::*;
use std::path::Path;

pub struct RustGroupEncoderGenerator<'a> {
    pub(crate) config: &'a rust::Config,
    pub(crate) path: &'a Path,
    pub(crate) types: &'a TypeMap,
    pub(crate) package: &'a str,
}

impl RustGroupEncoderGenerator<'_> {
    pub fn write_encoder(&self, group: &GroupType) -> Result<()> {
        let encoder_tokens: Tokens<Rust> = quote!($(self.generate_group_encoder(group)?));

        write_file(
            &self.path.join(ENCODER_FILE_NAME),
            self.config,
            encoder_tokens,
        )?;

        Ok(())
    }

    pub fn generate_group_encoder(&self, group: &GroupType) -> Result<impl FormatInto<Rust>> {
        let name = group.name.as_str();
        let encoder_name = format!("{}Encoder", name.to_case(Case::UpperCamel));
        let (fields, groups, var_data) = field_groups(&group.fields);

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

        let block_length_type = match &dimension_type.fields.first() {
            Some(Type::EncodedData(block_length_type)) => block_length_type,
            _ => {
                return Err(anyhow!(
                    "Only encoded data type expected for the block length type in group '{}'",
                    group.name
                ))
            }
        };
        let num_in_group_type = match &dimension_type.fields.last() {
            Some(Type::EncodedData(num_in_group_type)) => num_in_group_type,
            _ => {
                return Err(anyhow!(
                    "Only encoded data type expected for the num in group type in group '{}'",
                    group.name
                ))
            }
        };

        let block_length_primitive = block_length_type
            .primitive_type
            .lang_primitive(&self.types.encoded_types)?;
        let block_length_primitive_size = block_length_type.primitive_type.size(self.types)?;
        let num_in_group_primitive = num_in_group_type
            .primitive_type
            .lang_primitive(&self.types.encoded_types)?;

        let offset_prefix = quote!(self.size +);
        let mut offset = 0;

        let field_tokens = self.generate_fields(&fields, &offset_prefix, &mut offset)?;

        let offset_tokens = quote!($offset_prefix$offset);

        let group_tokens = self.generate_groups(&groups, &offset_tokens)?;
        let var_data_tokens = self.generate_var_data_fields(&var_data, &groups, &offset_tokens)?;

        Ok(quote! {
            use crate::error::*;
            use crate::$(self.package)::composites::*;
            use crate::$(self.package)::encoder::*;
            use crate::$(self.package)::enums::*;
            use crate::$(self.package)::groups::*;
            use crate::$(self.package)::sets::*;
            use crate::$(self.package)::var_data::*;
            use std::convert::TryFrom;

            #[derive(Debug)]
            pub struct $(&encoder_name)<'a> {
                buffer: WriteBuf<'a>,
                block_length: $(&block_length_primitive),
                num_in_group: $(&num_in_group_primitive),
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

            impl $(&encoder_name)<'_> {
                #[inline]
                pub fn size(&self) -> usize {
                    self.size
                }

                #[inline]
                pub fn advance(&mut self) -> Result<()> {
                    let fields_size = self.block_length as usize;
                    let groups_size = 0$(for group_size in group_sizes => $group_size);
                    let var_data_size = 0$(for var_data_field_size in var_data_field_sizes => $var_data_field_size);

                    self.num_in_group = self.num_in_group.checked_add(1).ok_or(SbeError::GroupOutOfBounds("num_in_group"))?;
                    self.size += fields_size + groups_size + var_data_size;

                    Ok(())
                }

                #[inline]
                pub fn finalize(mut self) -> Result<()> {
                    self.buffer.put_$(block_length_primitive)_at(0, self.block_length)?;
                    self.buffer.put_$(num_in_group_primitive)_at($block_length_primitive_size, self.num_in_group)
                }

                $field_tokens

                $group_tokens

                $var_data_tokens
            }

            impl<'a> TryFrom<WriteBuf<'a>> for $(&encoder_name)<'a> {
                type Error = SbeError;

                fn try_from(buffer: WriteBuf<'a>) -> Result<Self> {
                    Ok(Self {
                        buffer,
                        block_length: $(offset),
                        num_in_group: 0,
                        size: $(dimension_type_size),
                        $(for group in &groups => $['\r']$(group.name.to_case(Case::Snake))_size: None,)
                        $(for var in &var_data => $['\r']$(var.name.to_case(Case::Snake))_size: None,)
                    })
                }
            }
        })
    }

    pub fn generate_fields(
        &self,
        fields: &[&FieldType],
        offset_prefix: &Tokens<Rust>,
        offset: &mut usize,
    ) -> Result<impl FormatInto<Rust>> {
        let mut encoder_fields: Tokens<Rust> = quote!();
        let composite_encoder_gen = RustCompositeEncoderGenerator {
            config: self.config,
            path: self.path,
            types: self.types,
            package: self.package,
        };

        for field in fields {
            encoder_fields.line();
            encoder_fields.append(self.generate_field(
                field,
                &composite_encoder_gen,
                offset_prefix,
                offset,
            )?);
        }

        Ok(encoder_fields)
    }

    fn generate_field(
        &self,
        field_type: &FieldType,
        composite_encoder_gen: &RustCompositeEncoderGenerator,
        offset_prefix: &Tokens<Rust>,
        offset: &mut usize,
    ) -> Result<impl FormatInto<Rust>> {
        let repr_type = field_type.to_type(self.types)?;
        let field_offset = quote!($offset_prefix$(*offset));

        Ok(match field_type.presence {
            Presence::Constant => quote!(),
            _ => {
                let tokens = composite_encoder_gen.generate_encoder_field(
                    &field_type.name,
                    &repr_type,
                    field_offset,
                )?;
                *offset += repr_type.size(self.types)?;
                quote!($tokens)
            }
        })
    }

    pub fn generate_groups(
        &self,
        groups: &[&GroupType],
        offset: &Tokens<Rust>,
    ) -> Result<impl FormatInto<Rust>> {
        let mut encoder_fields: Tokens<Rust> = quote!();

        for group_idx in 0..groups.len() {
            encoder_fields.line();
            encoder_fields.append(self.generate_group(
                groups[group_idx],
                &groups[..group_idx],
                offset,
            )?);
        }

        Ok(encoder_fields)
    }

    fn generate_group(
        &self,
        group: &GroupType,
        previous_groups: &[&GroupType],
        offset: &Tokens<Rust>,
    ) -> Result<impl FormatInto<Rust>> {
        let group_name = group.name.as_str();
        let func_name = group_name.to_case(Case::Snake);
        let encoder_name = format!("{}Encoder", group_name.to_case(Case::UpperCamel));

        let prev_group_sizes = previous_groups
            .iter()
            .map(|prev_group| {
                let prev_group_name = prev_group.name.to_case(Case::Snake);
                quote!($[' ']+ self.$(&prev_group_name)_size.ok_or(SbeError::MissingGroupSize($(quoted(prev_group_name))))?)
            });

        Ok(quote! {
            #[inline]
            pub fn $(&func_name)_encoder<T>(&mut self, action: impl FnOnce(&mut $(&encoder_name)) -> Result<T>) -> Result<T> {
                let offset = $offset$(for prev_group_size in prev_group_sizes => $prev_group_size);
                let buffer = self.buffer.split_at_mut(offset)?.1;

                let mut encoder = buffer.try_into()?;

                let result = action(&mut encoder)?;

                self.$(func_name)_size = Some(encoder.size());
                encoder.finalize()?;

                Ok(result)
            }
        })
    }

    pub fn generate_var_data_fields(
        &self,
        var_data_fields: &[&VariableDataType],
        groups: &[&GroupType],
        offset: &Tokens<Rust>,
    ) -> Result<impl FormatInto<Rust>> {
        let mut encoder_fields: Tokens<Rust> = quote!();

        for var_data_field_idx in 0..var_data_fields.len() {
            encoder_fields.line();
            encoder_fields.append(self.generate_var_data_field(
                var_data_fields[var_data_field_idx],
                &var_data_fields[..var_data_field_idx],
                groups,
                offset,
            )?);
        }

        Ok(encoder_fields)
    }

    fn generate_var_data_field(
        &self,
        var_data_field: &VariableDataType,
        previous_var_data_fields: &[&VariableDataType],
        groups: &[&GroupType],
        offset: &Tokens<Rust>,
    ) -> Result<impl FormatInto<Rust>> {
        let name = var_data_field.name.as_str();
        let func_name = name.to_case(Case::Snake);
        let encoder_name = format!("{}Encoder", name.to_case(Case::UpperCamel));

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
            pub fn $(&func_name)_encoder<T>(&mut self, action: impl FnOnce(&mut $(&encoder_name)) -> Result<T>) -> Result<T> {
                let offset = $group_size$(for var_data_field_size in var_data_field_sizes => $var_data_field_size);
                let buffer = self.buffer.split_at_mut(offset)?.1;

                let mut encoder = buffer.try_into()?;

                let result = action(&mut encoder)?;
                self.$(func_name)_size = Some(encoder.size());
                encoder.finalize()?;

                Ok(result)
            }
        })
    }
}
