use crate::generator::python::codecs::composite_type::get_type_name;
use crate::generator::python::constants::GROUP_MODULE_NAME;
use crate::generator::write_file;
use crate::models::types::field_type::FieldType;
use crate::models::types::group_type::GroupType;
use crate::models::types::primitive_type::PrimitiveConvertible;
use crate::models::types::variable_data_type::VariableDataType;
use crate::models::types::{Presence, Type};

use crate::generator::common::field_groups;
use crate::generator::python::module::ModuleGenerator;
use anyhow::{anyhow, Result};
use convert_case::{Case, Casing};
use genco::prelude::*;
use genco::{quote, Tokens};
use std::fs::create_dir_all;
use std::path::Path;

impl ModuleGenerator<'_> {
    pub fn write_group_codecs(&self) -> Result<()> {
        let module_path = self.path.join(GROUP_MODULE_NAME);
        create_dir_all(&module_path)?;

        let mut module_tokens: Tokens<Rust> = quote!();

        for (name, group_type) in &self.schema.message_types.group_types {
            self.write_group_codec(&module_path, group_type)?;

            module_tokens.append(quote! {
                pub mod $(name.to_case(Case::Snake));
                pub use self::$(name.to_case(Case::Snake))::$(name.to_case(Case::UpperCamel));
            });
            module_tokens.push();
        }

        write_file(&module_path.join("mod.rs"), &self.config, module_tokens)?;

        Ok(())
    }

    fn write_group_codec(&self, module_path: &Path, group: &GroupType) -> Result<()> {
        let name = group.name.to_case(Case::UpperCamel);
        let rust_decoder = format!("Rust{}Decoder", name);
        let rust_encoder = format!("Rust{}Encoder", name);
        let (fields, groups, var_data_fields) = field_groups(&group.fields);

        let optional_fields = fields
            .iter()
            .filter(|field| matches!(field.presence, Presence::Optional));
        let mandatory_fields = fields
            .iter()
            .filter(|field| matches!(field.presence, Presence::Required));
        // Floats are not hashable
        let hashable_fields = fields
            .iter()
            .filter(|field| !matches!(field.presence, Presence::Constant))
            .filter(|field| {
                let encoded = field.to_type(&self.schema.types).unwrap();
                match encoded {
                    Type::EncodedData(encoded) => encoded
                        .is_hashable(&self.schema.types.encoded_types)
                        .unwrap_or(false),
                    _ => true,
                }
            });

        let group_tokens: Tokens<Rust> = quote! {
            use pyo3::{pyclass, pymethods};
            use rust_codecs::$(&self.schema.package)::groups::$(&name)Decoder as $(&rust_decoder);
            use rust_codecs::$(&self.schema.package)::groups::$(&name)Encoder as $(&rust_encoder);
            use rust_codecs::error::{Result as SbeResult, SbeError};
            use crate::$(&self.schema.package)::composites::*;
            use crate::$(&self.schema.package)::enums::*;
            use crate::$(&self.schema.package)::sets::*;
            use crate::$(&self.schema.package)::groups::*;
            use std::convert::TryFrom;
            use std::borrow::Cow;
            use std::hash::{DefaultHasher, Hash, Hasher};

            #[pyclass(subclass, eq)]
            #[derive(Debug, Clone, PartialEq)]
            pub struct $(&name) {
                $(for field in &fields {
                    $['\r']
                    $(if !matches!(field.presence, Presence::Constant) {
                        #[pyo3(get, set)]
                        $(self.field_struct(field)?)
                    })
                })
                $(for group in &groups {
                    $['\r']
                    #[pyo3(get, set)]
                    $(self.group_struct(group)?)
                })
                $(for var_data in &var_data_fields {
                    $['\r']
                    #[pyo3(get, set)]
                    $(self.var_data_field_struct(var_data)?)
                })
            }

            #[pymethods]
            impl $(&name) {
                #[new]
                pub fn py_new(
                    $(for field in mandatory_fields {
                        $['\r']
                        $(self.field_struct(field)?)
                    })
                    $(for group in &groups {
                        $['\r']
                        $(self.group_struct(group)?)
                    })
                    $(for var_data in &var_data_fields {
                        $['\r']
                        $(self.var_data_field_struct(var_data)?)
                    })
                    $(for field in optional_fields {
                        $['\r']
                        $(self.field_struct(field)?)
                    })
                ) -> Self {
                    Self {
                        $(for field in &fields {
                            $(if !matches!(field.presence, Presence::Constant) {
                                $['\r']
                                $(field.name.to_case(Case::Snake)),
                            })
                        })
                        $(for group in &groups {
                            $['\r']
                            $(group.name.to_case(Case::Snake)),
                        })
                        $(for var_data in &var_data_fields {
                            $['\r']
                            $(var_data.name.to_case(Case::Snake)),
                        })
                    }
                }

                fn __hash__(&self) -> u64 {
                    let mut hasher = DefaultHasher::new();
                    self.hash(&mut hasher);
                    hasher.finish()
                }

                $(self.field_constant_enums(&fields)?)

                $(self.var_data_fields_getter_bytes(&var_data_fields)?)
            }

            impl $(&name) {
                #[inline]
                pub fn write(&self, encoder: &mut $(&rust_encoder)) -> SbeResult<()> {
                    $(for field in &fields {
                        $['\r']
                        $(self.field_write(field)?)
                    })
                    $(for group in &groups {
                        $['\r']
                        $(self.group_write(group)?)
                    })
                    $(for var_data in &var_data_fields {
                        $['\r']
                        $(self.var_data_field_write(var_data)?)
                    })

                    Ok(())
                }
            }

            impl Hash for $(&name) {
                fn hash<H: Hasher>(&self, state: &mut H) {
                    $(for field in hashable_fields {
                        $['\r']
                        self.$(field.name.to_case(Case::Snake)).hash(state);
                    })
                }
            }

            impl TryFrom<&mut $(&rust_decoder)<'_>> for $(&name) {
                type Error = SbeError;

                #[inline]
                fn try_from(value: &mut $(&rust_decoder)) -> SbeResult<Self> {
                    Ok(Self {
                        $(for field in &fields {
                            $['\r']
                            $(self.field_read(field)?)
                        })
                        $(for group in &groups {
                            $['\r']
                            $(self.group_read(group)?)
                        })
                        $(for var_data in &var_data_fields {
                            $['\r']
                            $(self.var_data_field_read(var_data)?)
                        })
                    })
                }
            }
        };

        let file_path = module_path.join(format!("{}.rs", name.to_case(Case::Snake)));
        write_file(&file_path, &self.config, group_tokens)?;

        Ok(())
    }

    pub fn field_struct(&self, field: &FieldType) -> Result<impl FormatInto<Rust>> {
        if matches!(field.presence, Presence::Constant) {
            return Ok(quote!());
        }

        let field_name = field.name.as_str();
        let var_name = field_name.to_case(Case::Snake);
        let field_type = field.to_type(&self.schema.types)?;

        let field_type = get_type_name(&field_type, &self.schema.types)?;

        Ok(quote! {
            $(&var_name): $field_type,
        })
    }

    pub fn field_write(&self, field: &FieldType) -> Result<impl FormatInto<Rust>> {
        if matches!(field.presence, Presence::Constant) {
            return Ok(quote!());
        }

        let field_type = field.to_type(&self.schema.types)?;

        Ok(quote!($(self.generate_composite_field_write(&field.name, &field_type)?)))
    }

    pub fn field_read(&self, field: &FieldType) -> Result<impl FormatInto<Rust>> {
        if matches!(field.presence, Presence::Constant) {
            return Ok(quote!());
        }

        let field_type = field.to_type(&self.schema.types)?;

        Ok(quote!($(self.generate_composite_field_from(&field.name, &field_type)?)))
    }

    pub fn field_constant_enums(&self, fields: &[&FieldType]) -> Result<impl FormatInto<Rust>> {
        let mut tokens: Tokens<Rust> = quote!();

        for field in fields {
            if !matches!(field.presence, Presence::Constant) {
                continue;
            }

            tokens.push();
            tokens.append(self.field_constant_enum(field)?);
        }

        Ok(tokens)
    }

    pub fn field_constant_enum(&self, field: &FieldType) -> Result<impl FormatInto<Rust>> {
        let field_name = field.name.to_case(Case::Snake);
        let repr_type = field.to_type(&self.schema.types)?;
        let enum_type = match repr_type {
            Type::Enum(enum_type) => enum_type,
            _ => return Err(anyhow!("Constant field {} is not an enum", field.name)),
        };
        let enum_type_name = enum_type.name.to_case(Case::UpperCamel);

        let default_value = field
            .value_ref
            .as_ref()
            .ok_or(anyhow!("Constant field {} has no ref value", field.name))?;

        let default_value = default_value
            .split('.')
            .last()
            .ok_or(anyhow!("Constant field {} has no ref value", field.name))?;
        let default_value = enum_type
            .values
            .iter()
            .find(|value| value.name == default_value)
            .ok_or(anyhow!(
                "Constant field {} has no value {}",
                field.name,
                default_value
            ))?;

        Ok(quote! {
            #[inline]
            #[getter]
            pub fn get_$(&field_name)(&self) -> $(&enum_type_name) {
                $(&enum_type_name)::$(&default_value.name)
            }
        })
    }

    pub fn group_struct(&self, group: &GroupType) -> Result<impl FormatInto<Rust>> {
        let group_name = group.name.as_str();
        let var_name = group_name.to_case(Case::Snake);
        let type_name = group_name.to_case(Case::UpperCamel);

        Ok(quote! {
            $(&var_name): Vec<$(&type_name)>,
        })
    }

    pub fn group_write(&self, group: &GroupType) -> Result<impl FormatInto<Rust>> {
        let group_name = group.name.as_str();
        let var_name = group_name.to_case(Case::Snake);

        Ok(quote! {
            encoder.$(&var_name)_encoder(|encoder| {
                for $(&var_name) in &self.$(&var_name) {
                    $(&var_name).write(encoder)?;
                    encoder.advance()?;
                }
                Ok(())
            })?;
        })
    }

    pub fn group_read(&self, group: &GroupType) -> Result<impl FormatInto<Rust>> {
        let group_name = group.name.as_str();
        let var_name = group_name.to_case(Case::Snake);

        Ok(quote! {
            $(&var_name): value.$(&var_name)_decoder(|decoder| {
                let mut $(&var_name) = Vec::with_capacity(decoder.num_in_group());
                for _ in 0..decoder.num_in_group() {
                    $(&var_name).push(decoder.try_into()?);
                    decoder.advance()?;
                }
                Ok($(&var_name))
            })?,
        })
    }

    pub fn var_data_field_struct(
        &self,
        var_data_field: &VariableDataType,
    ) -> Result<impl FormatInto<Rust>> {
        let field_name = var_data_field.name.as_str();
        let var_name = field_name.to_case(Case::Snake);
        let repr_type = var_data_field.repr_type(&self.schema.types.composite_types)?;
        let value_type = match repr_type.fields[1] {
            Type::EncodedData(ref var_data_type) => var_data_type,
            _ => {
                return Err(anyhow!(
                "Only encoded data type expected for the value type in variable data encoding '{}'",
                &var_data_field.name
            ))
            }
        };
        let value_rust_type = value_type
            .primitive_type
            .lang_primitive(&self.schema.types.encoded_types)?;

        Ok(quote! {
            $(if value_type.is_string() {
                $(&var_name): String
            } else {
                $(&var_name): Vec<$value_rust_type>
            }),
        })
    }

    pub fn var_data_field_write(
        &self,
        var_data_field: &VariableDataType,
    ) -> Result<impl FormatInto<Rust>> {
        let field_name = var_data_field.name.as_str();
        let var_name = field_name.to_case(Case::Snake);

        let encode = if var_data_field.is_string(&self.schema.types.composite_types)? {
            quote!(encoder.put_slice_at(0, self.$(&var_name).as_bytes()))
        } else if var_data_field.is_bytes(&self.schema.types)? {
            quote!(encoder.put_slice_at(0, &self.$(&var_name)))
        } else {
            quote! {
                for (idx, value) in self.$(&var_name).iter().enumerate() {
                    encoder.put_at(idx.try_into().unwrap(), *value)?;
                }

                Ok(())
            }
        };

        Ok(quote! {
            encoder.$(&var_name)_encoder(|encoder| {
                $encode
            })?;
        })
    }

    pub fn var_data_field_read(
        &self,
        var_data_field: &VariableDataType,
    ) -> Result<impl FormatInto<Rust>> {
        let field_name = var_data_field.name.as_str();
        let var_name = field_name.to_case(Case::Snake);

        let decode = if var_data_field.is_string(&self.schema.types.composite_types)? {
            quote! {
                Ok(String::from_utf8(decoder.get_slice_at(0, decoder.length())?.to_vec())?)
            }
        } else if var_data_field.is_bytes(&self.schema.types)? {
            quote!(Ok(decoder.get_slice_at(0, decoder.length())?.to_vec()))
        } else {
            quote! {
                let mut res = Vec::new();
                for idx in 0..decoder.length() {
                    res.push(decoder.get_at(idx)?);
                }
                Ok(res)
            }
        };

        Ok(quote! {
            $(&var_name): value.$(&var_name)_decoder(|decoder| {
                $decode
            })?,
        })
    }

    /// Generates more efficient getters for all var-data fields that are bytes
    pub fn var_data_fields_getter_bytes(
        &self,
        var_data_fields: &[&VariableDataType],
    ) -> Result<impl FormatInto<Rust>> {
        let mut tokens: Tokens<Rust> = quote!();

        for var_data_field in var_data_fields {
            if !var_data_field.is_bytes(&self.schema.types)?
                || var_data_field.is_string(&self.schema.types.composite_types)?
            {
                continue;
            }

            let field_name = var_data_field.name.as_str();
            let var_name = field_name.to_case(Case::Snake);

            tokens.push();
            tokens.append(quote! {
                #[inline]
                #[getter]
                pub fn get_$(&var_name)(&self) -> Cow<[u8]> {
                    (&self.$(&var_name)).into()
                }
            });
        }

        Ok(tokens)
    }
}
