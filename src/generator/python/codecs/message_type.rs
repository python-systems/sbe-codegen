use crate::generator::common::field_groups;
use crate::generator::python::constants::MESSAGE_MODULE_NAME;
use crate::generator::python::module::ModuleGenerator;
use crate::generator::write_file;
use crate::models::message::MessageType;
use crate::models::types::field_type::FieldType;
use crate::models::types::group_type::GroupType;
use crate::models::types::variable_data_type::VariableDataType;
use crate::models::types::{Presence, Type};
use convert_case::{Case, Casing};
use genco::lang::Rust;
use genco::prelude::FormatInto;
use genco::{quote, Tokens};
use std::fs::create_dir_all;
use std::path::Path;

impl ModuleGenerator<'_> {
    pub fn write_message_codecs(&self) -> anyhow::Result<()> {
        let module_path = self.path.join(MESSAGE_MODULE_NAME);
        create_dir_all(&module_path)?;

        let mut module_tokens: Tokens<Rust> = quote!();

        for message_type in self.schema.message_types.message_types.values() {
            let name = &message_type.name;
            self.write_message_codec(&module_path, message_type)?;

            module_tokens.append(quote! {
                pub mod $(name.to_case(Case::Snake));
                pub use self::$(name.to_case(Case::Snake))::$(name.to_case(Case::UpperCamel));
            });
            module_tokens.push();
        }

        write_file(&module_path.join("mod.rs"), &self.config, module_tokens)?;

        Ok(())
    }

    fn write_message_codec(&self, module_path: &Path, message: &MessageType) -> anyhow::Result<()> {
        let name = message.name.to_case(Case::UpperCamel);
        let rust_decoder = format!("Rust{}Decoder", name);
        let rust_encoder = format!("Rust{}Encoder", name);
        let (fields, groups, var_data_fields) = field_groups(&message.fields);

        let optional_fields = fields
            .iter()
            .filter(|field| matches!(field.presence, Presence::Optional))
            .copied()
            .collect();
        let mandatory_fields = fields
            .iter()
            .filter(|field| matches!(field.presence, Presence::Required))
            .copied()
            .collect();
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

        let message_tokens: Tokens<Rust> = quote! {
            use anyhow::{anyhow, Result};
            use pyo3::{pyclass, pymethods, Bound};
            use pyo3::types::{PyType, PyByteArray, PyTuple, PyDict};
            use pyo3::prelude::PyByteArrayMethods;
            use rust_codecs::$(&self.schema.package)::messages::$(&name)Decoder as $(&rust_decoder);
            use rust_codecs::$(&self.schema.package)::messages::$(&name)Encoder as $(&rust_encoder);
            use rust_codecs::$(&self.schema.package)::messages::$(name.to_case(Case::ScreamingSnake))_ID;
            use rust_codecs::$(&self.schema.package)::encoder::WriteBuf;
            use rust_codecs::$(&self.schema.package)::decoder::ReadBuf;
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
                #[classattr]
                const ID: u16 = $(name.to_case(Case::ScreamingSnake))_ID;

                $(generate_ctor_signature(&mandatory_fields, &groups, &var_data_fields, &optional_fields))
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
                    _py_args: &Bound<PyTuple>,
                    _py_kwargs: Option<&Bound<PyDict>>
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

                $(self.message_encode())

                $(self.message_decode())
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
        write_file(&file_path, &self.config, message_tokens)?;

        Ok(())
    }

    fn message_encode(&self) -> impl FormatInto<Rust> {
        quote! {
            #[inline]
            pub fn write_to_buffer(&self, buffer: &Bound<'_, PyByteArray>) -> Result<usize> {
                let slice = unsafe {
                    // SAFETY: Neither the interpreter, neither any other Rust code will mutate the slice.
                    buffer.as_bytes_mut()
                };

                let write_buf = WriteBuf::new(slice);
                let mut encoder = write_buf.try_into()?;

                self.write(&mut encoder)?;
                let size = encoder.size().ok_or(anyhow!("Missing encoder size"))?;
                Ok(size)
            }

            #[inline]
            pub fn to_bytes(&self, buffer_size: usize) -> Result<Cow<[u8]>> {
                let mut buffer = vec![0; buffer_size];
                let write_buf = WriteBuf::new(&mut buffer);
                let mut encoder = write_buf.try_into()?;

                self.write(&mut encoder)?;
                let size = encoder.size().ok_or(anyhow!("Missing encoder size"))?;
                Ok(buffer[0..size].to_vec().into())
            }
        }
    }

    fn message_decode(&self) -> impl FormatInto<Rust> {
        quote! {
            #[inline]
            #[classmethod]
            pub fn from_bytes(_cls: &Bound<PyType>, buffer: &[u8]) -> Result<Self> {
                let read_buf = ReadBuf::new(buffer);
                let mut decoder = read_buf.try_into()?;

                Ok(Self::try_from(&mut decoder)?)
            }
        }
    }
}

pub fn generate_ctor_signature(
    mandatory_fields: &Vec<&FieldType>,
    groups: &Vec<&GroupType>,
    var_data: &Vec<&VariableDataType>,
    optional_fields: &Vec<&FieldType>,
) -> Tokens<Rust> {
    quote! {
        #[pyo3(signature = (
            $(for field in mandatory_fields {
                $(&field.name.to_case(Case::Snake)),
            })
            $(for group in groups {
                $(&group.name.to_case(Case::Snake)),
            })
            $(for var_data in var_data {
                $(&var_data.name.to_case(Case::Snake)),
            })
            $(for field in optional_fields {
                $(&field.name.to_case(Case::Snake))=None,
            })
            *_py_args, **_py_kwargs
        ))]
    }
}
