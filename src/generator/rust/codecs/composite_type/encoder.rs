use std::path::Path;

use anyhow::{anyhow, Result};
use convert_case::{Case, Casing};
use genco::prelude::*;

use crate::generator::common::FieldMetadata;
use crate::generator::rust::constants::ENCODER_FILE_NAME;
use crate::generator::write_file;
use crate::models::types::composite_type::CompositeType;
use crate::models::types::encoded_data_type::EncodedDataType;
use crate::models::types::enum_type::EnumType;
use crate::models::types::primitive_type::{NativeType, PrimitiveConvertible};
use crate::models::types::set_type::SetType;
use crate::models::types::{CharacterEncoding, Presence, SizedEncoded, Type};
use crate::models::TypeMap;

pub struct RustCompositeEncoderGenerator<'a> {
    pub(crate) config: &'a rust::Config,
    pub(crate) path: &'a Path,
    pub(crate) types: &'a TypeMap,
    pub(crate) package: &'a str,
}

impl RustCompositeEncoderGenerator<'_> {
    pub fn write_encoder(&self, composite_type: &CompositeType) -> Result<()> {
        let decoder_tokens: Tokens<Rust> =
            quote!($(self.generate_composite_encoder(composite_type)?));

        write_file(
            &self.path.join(ENCODER_FILE_NAME),
            self.config,
            decoder_tokens,
        )?;

        Ok(())
    }

    fn generate_composite_encoder(
        &self,
        composite_type: &CompositeType,
    ) -> Result<impl FormatInto<Rust>> {
        let name = format!("{}Encoder", composite_type.name.to_case(Case::UpperCamel));
        let encoder_name = name.as_str();
        let fields = composite_type.fields.as_slice();

        Ok(quote! {
            use crate::error::*;
            use crate::$(self.package)::composites::*;
            use crate::$(self.package)::encoder::*;
            use crate::$(self.package)::enums::*;
            use crate::$(self.package)::sets::*;
            use std::convert::TryFrom;
            use std::str::FromStr;

            #[derive(Debug, Default)]
            pub struct $encoder_name<'a> {
                buffer: WriteBuf<'a>,
            }

            impl<'a> From<WriteBuf<'a>> for $encoder_name<'a> {
                fn from(buffer: WriteBuf<'a>) -> Self {
                    Self { buffer }
                }
            }

            impl<'a> $encoder_name<'a> {
                $(self.generate_encoder_fields(fields)?)
            }
        })
    }

    fn generate_encoder_fields(&self, fields: &[Type]) -> Result<impl FormatInto<Rust>> {
        let mut offset: usize = 0;

        let mut encoder_fields: Tokens<Rust> = quote!();

        for field in fields {
            encoder_fields.append(self.generate_encoder_field(
                field.name(),
                field,
                quote!($offset),
            )?);
            encoder_fields.line();

            offset += field.size(self.types)?;
        }

        Ok(encoder_fields)
    }

    pub(crate) fn generate_encoder_field(
        &self,
        field_name: &str,
        field: &Type,
        offset: Tokens<Rust>,
    ) -> Result<impl FormatInto<Rust>> {
        Ok(quote! {
            $(match field {
                Type::EncodedData(encoded_type) => $(
                    self.generate_encoded_field_encoder(field_name, encoded_type, offset)?
                ),
                Type::Enum(enum_type) => $(
                    self.generate_enum_field_encoder(field_name, enum_type, offset)?
                ),
                Type::Set(set_type) => $(
                    self.generate_set_field_encoder(field_name, set_type, offset)?
                ),
                Type::Composite(composite_type) => $(
                    self.generate_composite_field_encoder(field_name, composite_type, offset)?
                ),
                Type::Reference(reference_type) => {$({
                    let referenced_type = self
                        .types
                        .find_type(&reference_type.type_name)
                        .ok_or(anyhow!("Referenced type {} not found", reference_type.type_name))?;
                    self.generate_encoder_field(field_name, &referenced_type, offset)?
                })}
            })
        })
    }

    fn generate_encoded_field_encoder(
        &self,
        field_name: &str,
        encoded_type: &EncodedDataType,
        offset: Tokens<Rust>,
    ) -> Result<impl FormatInto<Rust>> {
        Ok(quote! {
            $(match encoded_type.presence {
                Presence::Constant => (),
                Presence::Required => $(self.generate_encoded_variable_field(field_name, encoded_type, offset)?),
                Presence::Optional => $(self.generate_encoded_optional_field(field_name, encoded_type, offset)?),
            })
        })
    }

    fn generate_encoded_variable_field(
        &self,
        field_name: &str,
        encoded_type: &EncodedDataType,
        offset: Tokens<Rust>,
    ) -> Result<impl FormatInto<Rust>> {
        let metadata = FieldMetadata::from(field_name, encoded_type, self.types)?;

        let value_type = value_type(
            &metadata.field_primitive_type,
            metadata.lang_type.name,
            metadata.field_length,
        );

        let string_field = string_encoder(field_name, metadata.encoding, metadata.field_length);

        let array_field: Tokens<Rust> = quote! {
            for (idx, part) in value.iter().enumerate() {
                self.buffer.put_$(&metadata.lang_type)_at(offset + idx * $(metadata.type_size), *part)?;
            }

            Ok(())
        };

        let simple_field: Tokens<Rust> = quote! {
            $(if let Some(min) = &encoded_type.min_value {
                $(min_check(field_name, min))
            })

            $(if let Some(max) = &encoded_type.max_value {
                $(max_check(field_name, max))
            })

            self.buffer.put_$(metadata.lang_type)_at(offset, value)
        };

        Ok(quote! {
            #[inline]
            pub fn $(&metadata.field_name)(&mut self, value: $value_type) -> Result<()> {
                let offset = $offset;

                $(match (metadata.field_primitive_type, metadata.field_length) {
                    (NativeType::Char, 2..) => $string_field,
                    (_, 2..) => $array_field,
                    (_, _) => $simple_field
                })
            }
        })
    }

    fn generate_encoded_optional_field(
        &self,
        field_name: &str,
        encoded_type: &EncodedDataType,
        offset: Tokens<Rust>,
    ) -> Result<impl FormatInto<Rust>> {
        let metadata = FieldMetadata::from(field_name, encoded_type, self.types)?;

        let null_value = encoded_type
            .null_value
            .clone()
            .unwrap_or(metadata.field_primitive_type.null()?);
        let value_type = value_type(
            &metadata.field_primitive_type,
            metadata.lang_type.name,
            metadata.field_length,
        );

        let string_field = string_encoder(field_name, metadata.encoding, metadata.field_length);

        let array_field: Tokens<Rust> = quote! {
            let value = value.unwrap_or(&[$(&null_value); $(metadata.field_length)]);

            for (idx, part) in value.iter().enumerate() {
                self.buffer.put_$(&metadata.lang_type)_at(offset + idx * $(metadata.type_size), *part)?;
            }

            Ok(())
        };

        let simple_field: Tokens<Rust> = quote! {
            $(if let Some(min) = &encoded_type.min_value {
                if let Some(value) = value {
                    $(min_check(field_name, min))
                }
            })

            $(if let Some(max) = &encoded_type.max_value {
                if let Some(value) = value {
                    $(max_check(field_name, max))
                }
            })

            let value = value.unwrap_or($(&null_value));
            self.buffer.put_$(metadata.lang_type)_at(offset, value)
        };

        Ok(quote! {
            #[inline]
            pub fn $(&metadata.field_name)(&mut self, value: Option<$value_type>) -> Result<()> {
                let offset = $offset;

                $(match (metadata.field_primitive_type, metadata.field_length) {
                    (NativeType::Char, 2..) => $string_field,
                    (_, 2..) => $array_field,
                    (_, _) => $simple_field
                })
            }
        })
    }

    fn generate_enum_field_encoder(
        &self,
        field_name: &str,
        enum_type: &EnumType,
        offset: Tokens<Rust>,
    ) -> Result<impl FormatInto<Rust>> {
        let enum_type_name = enum_type.name.to_case(Case::UpperCamel);
        let field_name = field_name.to_case(Case::Snake);
        let field_type = enum_type
            .encoding_type
            .lang_primitive(&self.types.encoded_types)?;

        Ok(quote! {
            #[inline]
            pub fn $(field_name)(&mut self, value: $enum_type_name) -> Result<()> {
                self.buffer.put_$(field_type)_at($offset, value as u8)
            }
        })
    }

    fn generate_set_field_encoder(
        &self,
        field_name: &str,
        set_type: &SetType,
        offset: Tokens<Rust>,
    ) -> Result<impl FormatInto<Rust>> {
        let set_type_name = set_type.name.to_case(Case::UpperCamel);
        let field_name = field_name.to_case(Case::Snake);
        let field_type = set_type
            .encoding_type
            .lang_primitive(&self.types.encoded_types)?;

        Ok(quote! {
            #[inline]
            pub fn $(field_name)(&mut self, value: $set_type_name) -> Result<()> {
                self.buffer.put_$(field_type)_at($offset, value.0)
            }
        })
    }

    fn generate_composite_field_encoder(
        &self,
        field_name: &str,
        composite_type: &CompositeType,
        offset: Tokens<Rust>,
    ) -> Result<impl FormatInto<Rust>> {
        let composite_type_name = composite_type.name.to_case(Case::UpperCamel);
        let field_name = field_name.to_case(Case::Snake);
        let encoder_name = format!("{}Encoder", composite_type_name);

        Ok(quote! {
            #[inline]
            pub fn $(field_name)_encoder<T>(&mut self, action: impl FnOnce(&mut $(&encoder_name)) -> Result<T>) -> Result<T> {
                let buffer = self.buffer.split_at_mut($offset)?.1;

                let mut encoder = buffer.into();

                action(&mut encoder)
            }
        })
    }
}

fn value_type(field_primitive_type: &NativeType, rust_type: &str, field_length: usize) -> String {
    match (field_primitive_type, field_length) {
        (NativeType::Char, 2..) => "&str".to_owned(),
        (_, 2..) => format!("&[{}; {}]", rust_type, field_length),
        (_, _) => rust_type.to_owned(),
    }
}

fn string_encoder(
    field_name: &str,
    encoding: Option<CharacterEncoding>,
    max_length: usize,
) -> impl FormatInto<Rust> {
    quote! {
        $(if let Some(CharacterEncoding::Ascii) = encoding {
            if !value.is_ascii() {
                return Err(SbeError::InvalidStringValue(value.to_string()));
            }
            $['\n']
        })
        let encoded = value.as_bytes();

        if encoded.len() > $max_length {
            return Err(SbeError::ValueOutOfBounds {
                field_name: $(quoted(field_name)),
                message: format!(
                    "string '{}' length {} > {} (max)",
                    value, encoded.len(), $max_length
                )
            });
        }

        self.buffer.put_bytes_at(offset, encoded)
    }
}

fn min_check(field_name: &str, min: &str) -> impl FormatInto<Rust> {
    quote! {
        if value < $min {
            return Err(SbeError::ValueOutOfBounds {
                field_name: $(quoted(field_name)),
                message: format!(
                    "{} < {} (min)",
                    value, $min
                )
            });
        }
    }
}

fn max_check(field_name: &str, max: &str) -> impl FormatInto<Rust> {
    quote! {
        if value > $max {
            return Err(SbeError::ValueOutOfBounds {
                field_name: $(quoted(field_name)),
                message: format!(
                    "{} > {} (max)",
                    value, $max
                )
            });
        }
    }
}
