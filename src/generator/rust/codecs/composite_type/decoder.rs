use crate::generator::common::{variable_value_type, FieldMetadata};
use crate::generator::rust::codecs::composite_type::default_value;
use crate::generator::rust::constants::DECODER_FILE_NAME;
use crate::generator::write_file;
use crate::models::types::composite_type::CompositeType;
use crate::models::types::encoded_data_type::EncodedDataType;
use crate::models::types::enum_type::EnumType;
use crate::models::types::primitive_type::{LanguagePrimitive, NativeType, PrimitiveConvertible};
use crate::models::types::set_type::SetType;
use crate::models::types::{Presence, SizedEncoded, Type};
use crate::models::TypeMap;
use anyhow::{anyhow, Result};
use convert_case::{Case, Casing};
use genco::prelude::*;
use std::path::Path;

pub struct RustCompositeDecoderGenerator<'a> {
    pub(crate) config: &'a rust::Config,
    pub(crate) path: &'a Path,
    pub(crate) types: &'a TypeMap,
    pub(crate) package: &'a str,
}

impl RustCompositeDecoderGenerator<'_> {
    pub fn write_decoder(&self, composite_type: &CompositeType) -> Result<()> {
        let decoder_tokens: Tokens<Rust> =
            quote!($(self.generate_composite_decoder(composite_type)?));

        write_file(
            &self.path.join(DECODER_FILE_NAME),
            self.config,
            decoder_tokens,
        )?;

        Ok(())
    }

    fn generate_composite_decoder(
        &self,
        composite_type: &CompositeType,
    ) -> Result<impl FormatInto<Rust>> {
        let name = composite_type.name.as_str();
        let decoder_name = format!("{}Decoder", name.to_case(Case::UpperCamel));
        let fields = composite_type.fields.as_slice();

        Ok(quote! {
            use crate::error::*;
            use crate::$(self.package)::composites::*;
            use crate::$(self.package)::decoder::*;
            use crate::$(self.package)::enums::*;
            use crate::$(self.package)::sets::*;
            use std::convert::TryFrom;

            #[derive(Debug, Default)]
            pub struct $(&decoder_name)<'a> {
                buffer: ReadBuf<'a>,
            }

            impl<'a> $(&decoder_name)<'a> {
                $(self.generate_decoder_fields(fields)?)
            }

            impl<'a> From<ReadBuf<'a>> for $(&decoder_name)<'a> {
                fn from(buffer: ReadBuf<'a>) -> Self {
                    Self { buffer }
                }
            }
        })
    }

    fn generate_decoder_fields(&self, fields: &[Type]) -> Result<impl FormatInto<Rust>> {
        let mut offset: usize = 0;

        let mut decoder_fields: Tokens<Rust> = quote!();

        for field in fields {
            decoder_fields.append(self.generate_decoder_field(field.name(), field, offset)?);
            decoder_fields.line();

            offset += field.size(self.types)?;
        }

        Ok(decoder_fields)
    }

    pub(crate) fn generate_decoder_field(
        &self,
        field_name: &str,
        field: &Type,
        offset: usize,
    ) -> Result<impl FormatInto<Rust>> {
        Ok(quote! {
            $(match field {
                Type::EncodedData(encoded_type) => $(
                    self.generate_encoded_field_decoder(field_name, encoded_type, offset)?
                ),
                Type::Enum(enum_type) => $(
                    self.generate_enum_field_decoder(field_name, enum_type, offset)?
                ),
                Type::Set(set_type) => $(
                    self.generate_set_field_decoder(field_name, set_type, offset)?
                ),
                Type::Composite(composite_type) => $(
                    self.generate_composite_field_decoder(field_name, composite_type, offset)?
                ),
                Type::Reference(reference_type) => {$({
                    let referenced_type = self
                        .types
                        .find_type(&reference_type.type_name)
                        .ok_or(anyhow!("Referenced type {} not found", reference_type.type_name))?;
                    self.generate_decoder_field(field_name, &referenced_type, offset)?
                })}
            })
        })
    }

    fn generate_enum_field_decoder(
        &self,
        field_name: &str,
        enum_type: &EnumType,
        offset: usize,
    ) -> Result<impl FormatInto<Rust>> {
        let enum_type_name = enum_type.name.to_case(Case::UpperCamel);
        let field_name = field_name.to_case(Case::Snake);
        let field_type: LanguagePrimitive<Rust> = enum_type
            .encoding_type
            .lang_primitive(&self.types.encoded_types)?;

        Ok(quote! {
            #[inline]
            pub fn $(field_name)(&self) -> Result<$(&enum_type_name)> {
                $(&enum_type_name)::try_from(self.buffer.get_$(field_type)_at($offset)?)
            }
        })
    }

    fn generate_set_field_decoder(
        &self,
        field_name: &str,
        set_type: &SetType,
        offset: usize,
    ) -> Result<impl FormatInto<Rust>> {
        let set_type_name = set_type.name.to_case(Case::UpperCamel);
        let field_name = field_name.to_case(Case::Snake);
        let field_type = set_type
            .encoding_type
            .lang_primitive(&self.types.encoded_types)?;

        Ok(quote! {
            #[inline]
            pub fn $(field_name)(&self) -> Result<$(&set_type_name)> {
                Ok($(&set_type_name)(self.buffer.get_$(field_type)_at($offset)?))
            }
        })
    }

    fn generate_composite_field_decoder(
        &self,
        field_name: &str,
        composite_type: &CompositeType,
        offset: usize,
    ) -> Result<impl FormatInto<Rust>> {
        let composite_type_name = composite_type.name.to_case(Case::UpperCamel);
        let field_name = field_name.to_case(Case::Snake);
        let decoder_name = format!("{}Decoder", composite_type_name);

        Ok(quote! {
            #[inline]
            pub fn $(field_name)_decoder<T>(&self, action: impl FnOnce(&mut $(&decoder_name)) -> Result<T>) -> Result<T> {
                let buffer = self.buffer.split_at($offset)?.1;

                let mut decoder = buffer.into();

                action(&mut decoder)
            }
        })
    }

    fn generate_encoded_field_decoder(
        &self,
        field_name: &str,
        encoded_type: &EncodedDataType,
        offset: usize,
    ) -> Result<impl FormatInto<Rust>> {
        Ok(quote! {
            $(match encoded_type.presence {
                Presence::Constant => $(self.generate_encoded_constant_field(field_name, encoded_type)?),
                Presence::Required => $(self.generate_encoded_variable_field(field_name, encoded_type, offset)?),
                Presence::Optional => $(self.generate_encoded_optional_field(field_name, encoded_type, offset)?),
            })
        })
    }

    fn generate_encoded_constant_field(
        &self,
        field_name: &str,
        encoded_type: &EncodedDataType,
    ) -> Result<impl FormatInto<Rust>> {
        let metadata = FieldMetadata::from(field_name, encoded_type, self.types)?;

        let value = encoded_type.default_value.as_ref().ok_or(anyhow!(
            "Constant field {} has no default value",
            metadata.field_name
        ))?;
        let value_type = match (&metadata.field_primitive_type, metadata.field_length) {
            (NativeType::Char, 2..) => "&'static str".to_owned(),
            // TODO: Constant arrays?
            // (_, 2..) => format!("[{}; {}]", rust_type, field_length),
            (_, _) => metadata.lang_type.name.to_owned(),
        };

        let default_value = default_value(value, &metadata.field_primitive_type);

        Ok(quote! {
            #[inline]
            pub fn $(metadata.field_name)(&self) -> Result<$value_type> {
                Ok($default_value)
            }
        })
    }

    fn generate_encoded_variable_field(
        &self,
        field_name: &str,
        encoded_type: &EncodedDataType,
        offset: usize,
    ) -> Result<impl FormatInto<Rust>> {
        let metadata = FieldMetadata::from(field_name, encoded_type, self.types)?;

        let value_type = variable_value_type(
            &metadata.field_primitive_type,
            metadata.lang_type.name,
            metadata.field_length,
        );

        let string_field = quote! {
            $(field_to_string(metadata.field_length))

            Ok(value)
        };

        let array_field: Tokens<Rust> = quote! {
            let mut value = [0_$(&metadata.lang_type); $(metadata.field_length)];

            for (idx, value) in value.iter_mut().enumerate() {
                *value = self.buffer.get_$(&metadata.lang_type)_at(offset + idx * $(metadata.type_size))?;
            }

            Ok(value)
        };

        let simple_field: Tokens<Rust> = quote! {
            let value = self.buffer.get_$(&metadata.lang_type)_at(offset)?;

            $(bounds_check(&metadata.field_name, encoded_type.min_value.as_ref(), encoded_type.max_value.as_ref()))

            Ok(value)
        };

        Ok(quote! {
            #[inline]
            pub fn $(metadata.field_name)(&self) -> Result<$value_type> {
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
        offset: usize,
    ) -> Result<impl FormatInto<Rust>> {
        let metadata = FieldMetadata::from(field_name, encoded_type, self.types)?;

        let value_type = variable_value_type(
            &metadata.field_primitive_type,
            metadata.lang_type.name,
            metadata.field_length,
        );

        let string_field: Tokens<Rust> = quote! {
            $(field_to_string(metadata.field_length))

            Ok(if !($(null_value_condition("value", &metadata.field_primitive_type, encoded_type)?)) {
                None
            } else {
                Some(value.to_string())
            })
        };

        let array_field: Tokens<Rust> = quote! {
            let mut value = [0_$(&metadata.lang_type); $(metadata.field_length)];
            let mut is_null = true;

            for (idx, value) in value.iter_mut().enumerate() {
                let part = self.buffer.get_$(&metadata.lang_type)_at(offset + idx * $(metadata.type_size))?;
                *value = part;
                is_null &= $(null_value_condition("part", &metadata.field_primitive_type, encoded_type)?);
            }

            Ok(if !is_null {
                Some(value)
            } else {
                None
            })
        };

        let simple_field: Tokens<Rust> = quote! {
            let value = self.buffer.get_$(&metadata.lang_type)_at(offset)?;

            if $(null_value_condition("value", &metadata.field_primitive_type, encoded_type)?) {
                return Ok(None);
            }

            $(bounds_check(&metadata.field_name, encoded_type.min_value.as_ref(), encoded_type.max_value.as_ref()))

            Ok(Some(value))
        };

        Ok(quote! {
            #[inline]
            pub fn $(metadata.field_name)(&self) -> Result<Option<$value_type>> {
                let offset = $offset;

                $(match (metadata.field_primitive_type, metadata.field_length) {
                    (NativeType::Char, 2..) => $string_field,
                    (_, 2..) => $array_field,
                    (_, _) => $simple_field
                })
            }
        })
    }
}

/// Converts value from a field of given length on offset `offset` to string.
/// Outputs a variable called `value`.
fn field_to_string(field_length: usize) -> impl FormatInto<Rust> {
    quote! {
        let src = self.buffer.get_slice_at(offset, $(field_length))?;

        // The encoding can be either UTF-8 or ASCII. Any valid ASCII string
        // is also a valid UTF-8 string, so we do not need to handle that.
        let value = std::str::from_utf8(src)?.to_string();
    }
}

fn bounds_check(
    field_name: &str,
    min: Option<&String>,
    max: Option<&String>,
) -> impl FormatInto<Rust> {
    quote! {
        $(if let Some(min) = min {
            if value < $min {
                return Err(SbeError::ValueOutOfBounds {
                    field_name: $(quoted(field_name)),
                    message: format!(
                        "{} < {} (min)",
                        value, $min
                    )
                });
            }
        })

        $(if let Some(max) = max {
            if value > $max {
                return Err(SbeError::ValueOutOfBounds {
                    field_name: $(quoted(field_name)),
                    message: format!(
                        "{} > {} (max)",
                        value, $max
                    )
                });
            }
        })
    }
}

fn null_value_condition(
    variable: &str,
    field_type: &NativeType,
    encoded_type: &EncodedDataType,
) -> Result<impl FormatInto<Rust>> {
    let null_value = encoded_type
        .null_value
        .clone()
        .unwrap_or(field_type.null()?);

    if null_value.contains("NAN") {
        Ok(quote! {
            $(variable).is_nan()
        })
    } else {
        Ok(quote! {
            $(variable) == $(&null_value)
        })
    }
}
