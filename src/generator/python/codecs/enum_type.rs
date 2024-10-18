use anyhow::Result;
use convert_case::{Case, Casing};
use std::fs::create_dir_all;
use std::path::Path;

use crate::models::types::enum_type::EnumType;

use crate::generator::python::constants::ENUM_MODULE_NAME;
use crate::generator::python::module::ModuleGenerator;
use crate::generator::write_file;
use crate::models::types::primitive_type::{NativeType, PrimitiveConvertible, ResolvableType};
use genco::prelude::*;

impl ModuleGenerator<'_> {
    pub fn write_enum_codecs(&self) -> Result<()> {
        let module_path = self.path.join(ENUM_MODULE_NAME);
        create_dir_all(&module_path)?;

        let mut module_tokens: Tokens<Rust> = quote! {
            #![allow(non_camel_case_types)]
        };

        for (name, enum_type) in &self.schema.types.enum_types {
            self.write_enum_codec(&module_path, enum_type)?;

            module_tokens.push();
            module_tokens.append(quote! {
                pub mod $(name.to_case(Case::Snake));
                pub use self::$(name.to_case(Case::Snake))::$(name.to_case(Case::UpperCamel));
            });
        }

        write_file(&module_path.join("mod.rs"), &self.config, module_tokens)?;

        Ok(())
    }

    fn write_enum_codec(&self, module_path: &Path, enum_type: &EnumType) -> Result<()> {
        let name = enum_type.name.to_case(Case::UpperCamel);
        let rust_name = format!("Rust{}", name);
        let values = enum_type.values.as_slice();
        let rust_type = enum_type
            .encoding_type
            .lang_primitive(&self.schema.types.encoded_types)?;
        let char_encoding = matches!(
            enum_type
                .encoding_type
                .resolved(&self.schema.types.encoded_types)?,
            NativeType::Char
        );

        let enum_tokens: Tokens<Rust> = quote! {
            use anyhow::{anyhow, Result};
            use pyo3::{pyclass, pymethods};
            use rust_codecs::$(&self.schema.package)::enums::$(&name) as $(&rust_name);
            use rust_codecs::error::SbeError;

            #[pyclass(hash, eq, ord, frozen)]
            #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
            pub enum $(&name) {
                $(for value in values {
                    $['\r']
                    $(value.name.to_case(Case::UpperSnake)),
                })
            }

            #[pymethods]
            impl $(&name) {
                #[new]
                pub fn py_new(value: $(&rust_type)) -> Result<Self> {
                    Ok(match value {
                        $(for value in values {
                            $['\r']
                            $(value.encoded_value(char_encoding)?) => Self::$(value.name.to_case(Case::UpperSnake)),
                        })
                        _ => return Err(anyhow!("Invalid value for enum {}: {}", $[str]($[const](&name)), value))
                    })
                }

                #[getter]
                pub fn value(&self) -> $(&rust_type) {
                    match self {
                        $(for value in values {
                            $['\r']
                            Self::$(value.name.to_case(Case::UpperSnake)) => $(value.encoded_value(char_encoding)?),
                        })
                    }
                }
            }

            impl TryFrom<$(&rust_name)> for $(&name) {
                type Error = SbeError;

                fn try_from(rust_enum: $(&rust_name)) -> Result<Self, SbeError> {
                    match rust_enum {
                        $(for value in values {
                            $['\r']
                            $(&rust_name)::$(value.name.to_case(Case::UpperCamel)) => Ok(Self::$(value.name.to_case(Case::UpperSnake))),
                        })
                        $(&rust_name)::NullVal => Err(SbeError::InvalidEnumValue {
                            type_name: $[str]($[const](&name)),
                            value: format!("{:?}", rust_enum),
                        })
                    }
                }
            }

            impl From<$(&name)> for $(&rust_name) {
                #[inline]
                fn from(value: $(&name)) -> $(&rust_name) {
                    match value {
                        $(for value in values {
                            $['\r']
                            $(&name)::$(value.name.to_case(Case::UpperSnake)) => Self::$(value.name.to_case(Case::UpperCamel)),
                        })
                    }
                }
            }
        };

        let file_path = module_path.join(format!("{}.rs", name.to_case(Case::Snake)));
        write_file(&file_path, &self.config, enum_tokens)?;

        Ok(())
    }
}
