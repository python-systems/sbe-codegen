use crate::generator::common::{variable_value_type, FieldMetadata};
use crate::generator::python::constants::COMPOSITE_MODULE_NAME;
use crate::generator::python::module::ModuleGenerator;
use crate::generator::write_file;
use crate::models::types::composite_type::CompositeType;
use crate::models::types::encoded_data_type::EncodedDataType;
use crate::models::types::primitive_type::NativeType;
use crate::models::types::{Presence, Type};
use crate::models::TypeMap;
use anyhow::{anyhow, Result};
use convert_case::{Case, Casing};
use genco::lang::Rust;
use genco::tokens::FormatInto;
use genco::{quote, Tokens};
use std::fs::create_dir_all;
use std::path::Path;

impl ModuleGenerator<'_> {
    pub fn write_composite_codecs(&self) -> Result<()> {
        let module_path = self.path.join(COMPOSITE_MODULE_NAME);
        create_dir_all(&module_path)?;

        let mut module_tokens: Tokens<Rust> = quote!();

        for (name, composite_type) in &self.schema.types.composite_types {
            self.write_composite_codec(&module_path, composite_type)?;

            module_tokens.append(quote! {
                pub mod $(name.to_case(Case::Snake));
                pub use self::$(name.to_case(Case::Snake))::$(name.to_case(Case::UpperCamel));
            });
            module_tokens.push();
        }

        write_file(&module_path.join("mod.rs"), &self.config, module_tokens)?;

        Ok(())
    }

    fn write_composite_codec(
        &self,
        module_path: &Path,
        composite_type: &CompositeType,
    ) -> Result<()> {
        let name = composite_type.name.to_case(Case::UpperCamel);
        let rust_decoder_name = format!("Rust{}Decoder", &name);
        let rust_encoder_name = format!("Rust{}Encoder", &name);

        let fields = &composite_type.fields;
        let optional_fields = fields
            .iter()
            .filter(|field| matches!(field.presence(&self.schema.types), Ok(Presence::Optional)));
        let mandatory_fields = fields
            .iter()
            .filter(|field| matches!(field.presence(&self.schema.types), Ok(Presence::Required)));
        let fields_ordered = mandatory_fields.chain(optional_fields);

        // Floats are not hashable
        let hashable_fields = fields
            .iter()
            .filter(|field| !matches!(field.presence(&self.schema.types), Ok(Presence::Constant)))
            .filter(|field| match field {
                Type::EncodedData(encoded) => encoded
                    .is_hashable(&self.schema.types.encoded_types)
                    .unwrap_or(false),
                _ => true,
            });

        let composite_tokens: Tokens<Rust> = quote! {
            use pyo3::{pyclass, pymethods};
            use rust_codecs::$(&self.schema.package)::composites::$(&name)Decoder as $(&rust_decoder_name);
            use rust_codecs::$(&self.schema.package)::composites::$(&name)Encoder as $(&rust_encoder_name);
            use crate::$(&self.schema.package)::composites::*;
            use crate::$(&self.schema.package)::enums::*;
            use crate::$(&self.schema.package)::sets::*;
            use crate::$(&self.schema.package)::groups::*;
            use rust_codecs::error::*;
            use std::convert::TryFrom;
            use std::hash::{DefaultHasher, Hash, Hasher};

            #[pyclass(subclass, eq)]
            #[derive(Debug, Clone, PartialEq)]
            pub struct $(&name) {
                $(self.generate_composite_fields(composite_type)?)
            }

            impl TryFrom<&mut $(&rust_decoder_name)<'_>> for $(&name) {
                type Error = SbeError;

                #[inline]
                fn try_from(value: &mut $(&rust_decoder_name)) -> Result<Self> {
                    Ok(Self {
                        // Fields
                        $(for field in &composite_type.fields {
                            $['\r']
                            $(self.generate_composite_field_from(field.name(), field)?)
                        })
                    })
                }
            }

            impl $(&name) {
                #[inline]
                pub fn write(&self, encoder: &mut $(&rust_encoder_name)) -> Result<()> {
                    $(for field in &composite_type.fields {
                        $['\r']
                        $(self.generate_composite_field_write(field.name(), field)?)
                    })
                    Ok(())
                }
            }

            #[pymethods]
            impl $(&name) {
                #[new]
                fn py_new(
                    $(for field in fields_ordered {
                        $(field.name().to_case(Case::Snake)): $(get_type_name(field, &self.schema.types)?),
                    })
                ) -> Self {
                    Self {
                        $(for field in &composite_type.fields {
                            $(if !matches!(field.presence(&self.schema.types)?, Presence::Constant) {
                                $['\r']
                                $(field.name().to_case(Case::Snake)),
                            })
                        })
                    }
                }

                fn __hash__(&self) -> u64 {
                    let mut hasher = DefaultHasher::new();
                    self.hash(&mut hasher);
                    hasher.finish()
                }

                $(self.generate_constant_fields(&composite_type.fields)?)
            }

            impl Hash for $(&name) {
                fn hash<H: Hasher>(&self, state: &mut H) {
                    $(for field in hashable_fields {
                        $['\r']
                        self.$(field.name().to_case(Case::Snake)).hash(state);
                    })
                }
            }
        };

        let file_path = module_path.join(format!("{}.rs", name.to_case(Case::Snake)));
        write_file(&file_path, &self.config, composite_tokens)?;

        Ok(())
    }

    pub fn generate_composite_field_from(
        &self,
        field_name: &str,
        field: &Type,
    ) -> Result<impl FormatInto<Rust>> {
        let field_name = field_name.to_case(Case::Snake);
        if matches!(field.presence(&self.schema.types)?, Presence::Constant) {
            return Ok(quote!());
        }

        Ok(quote! {
            $(match field {
                Type::EncodedData(_) | Type::Set(_) => {
                    $(&field_name): value.$(&field_name)()?.into(),
                }
                Type::Enum(_) => {
                    $(&field_name): value.$(&field_name)()?.try_into()?,
                }
                Type::Composite(_) => {
                    $(&field_name): value.$(&field_name)_decoder(|decoder| { decoder.try_into() })?,
                }
                Type::Reference(t) => $({
                    let referenced_type = self
                        .schema
                        .types
                        .find_type(&t.type_name)
                        .ok_or(anyhow!("Referenced type {} not found", t.type_name))?;

                    self.generate_composite_field_from(&field_name, &referenced_type)?
                })
            })
        })
    }

    pub fn generate_composite_field_write(
        &self,
        field_name: &str,
        field: &Type,
    ) -> Result<impl FormatInto<Rust>> {
        let field_name = field_name.to_case(Case::Snake);
        if matches!(field.presence(&self.schema.types)?, Presence::Constant) {
            return Ok(quote!());
        }

        let encoded_field = |field_name: &str,
                             encoded_data_type: &EncodedDataType|
         -> Result<Tokens<Rust>> {
            let metadata = FieldMetadata::from(field_name, encoded_data_type, &self.schema.types)?;

            Ok(quote! {
                $(if metadata.field_length > 1 {
                    encoder.$(field_name)(&self.$(field_name))?
                } else {
                    encoder.$(field_name)(self.$(field_name).into())?
                })
            })
        };

        Ok(quote! {
            $(match field {
                Type::EncodedData(t) => {
                    $(encoded_field(&field_name, t)?);
                },
                Type::Set(_) | Type::Enum(_) => {
                    encoder.$(&field_name)(self.$(&field_name).into())?;
                }
                Type::Composite(_) => {
                    encoder.$(&field_name)_encoder(|encoder| { self.$(&field_name).write(encoder) })?;
                }
                Type::Reference(t) => $({
                        let referenced_type = self
                            .schema
                            .types
                            .find_type(&t.type_name)
                            .ok_or(anyhow!("Referenced type {} not found", t.type_name))?;

                        self.generate_composite_field_write(&field_name, &referenced_type)?
                })
            })
        })
    }

    fn generate_composite_fields(
        &self,
        composite_type: &CompositeType,
    ) -> Result<impl FormatInto<Rust>> {
        let mut composite_fields: Tokens<Rust> = quote!();
        for field in &composite_type.fields {
            composite_fields.push();
            composite_fields.append(self.generate_composite_field(field.name(), field)?);
        }

        Ok(composite_fields)
    }

    pub fn generate_composite_field(
        &self,
        field_name: &str,
        field_type: &Type,
    ) -> Result<impl FormatInto<Rust>> {
        if matches!(field_type.presence(&self.schema.types)?, Presence::Constant) {
            return Ok(quote!());
        }

        Ok(quote! {
            #[pyo3(get, set)]
            $(&field_name.to_case(Case::Snake)): $(get_type_name(field_type, &self.schema.types)?),
        })
    }

    pub fn generate_constant_fields(&self, fields: &Vec<Type>) -> Result<impl FormatInto<Rust>> {
        let mut constant_fields: Tokens<Rust> = quote!();

        for field in fields {
            if matches!(field.presence(&self.schema.types)?, Presence::Constant) {
                constant_fields.line();
                constant_fields.append(self.generate_constant_field(field.name(), field)?);
            }
        }

        Ok(constant_fields)
    }

    pub fn generate_constant_field(
        &self,
        field_name: &str,
        field_type: &Type,
    ) -> Result<impl FormatInto<Rust>> {
        let field_type = match field_type {
            Type::EncodedData(encoded_type) => encoded_type,
            _ => return Err(anyhow!("Constant field {} is not encoded data", field_name)),
        };

        let metadata = FieldMetadata::from(field_name, field_type, &self.schema.types)?;
        let value = field_type.default_value.as_ref().ok_or(anyhow!(
            "Constant field {} has no default value",
            metadata.field_name
        ))?;
        let value_type = match (&metadata.field_primitive_type, metadata.field_length) {
            (NativeType::Char, 2..) => "String".to_owned(),
            // TODO: Constant arrays?
            // (_, 2..) => format!("[{}; {}]", rust_type, field_length),
            (_, _) => metadata.lang_type.name.to_owned(),
        };

        let default_value = match metadata.field_primitive_type {
            NativeType::Char => format!("\"{}\".to_owned()", value),
            _ => value.to_owned(),
        };

        Ok(quote! {
            #[getter]
            #[inline]
            fn get_$(&metadata.field_name)(&self) -> $(&value_type) {
                $(&default_value)
            }
        })
    }
}

pub fn get_type_name(field_type: &Type, types: &TypeMap) -> Result<String> {
    Ok(match field_type {
        Type::EncodedData(t) => {
            let metadata = FieldMetadata::from(field_type.name(), t, types)?;
            let value_type = variable_value_type(
                &metadata.field_primitive_type,
                metadata.lang_type.name,
                metadata.field_length,
            );
            if t.presence == Presence::Optional {
                format!("Option<{}>", value_type)
            } else {
                value_type
            }
        }
        Type::Set(t) => t.name.clone().to_case(Case::UpperCamel),
        Type::Enum(t) => t.name.clone().to_case(Case::UpperCamel),
        Type::Composite(t) => t.name.to_case(Case::UpperCamel),
        Type::Reference(t) => {
            let referenced_type = types
                .find_type(&t.type_name)
                .ok_or(anyhow!("Referenced type {} not found", t.type_name))?;

            get_type_name(&referenced_type, types)?
        }
    })
}
