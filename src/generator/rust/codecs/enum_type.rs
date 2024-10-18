use anyhow::Result;
use convert_case::{Case, Casing};
use std::fs::create_dir_all;
use std::path::Path;

use crate::models::types::enum_type::EnumType;

use crate::generator::rust::constants::ENUM_MODULE_NAME;
use crate::generator::rust::module::ModuleGenerator;
use crate::generator::write_file;
use crate::models::types::primitive_type::{NativeType, PrimitiveConvertible, ResolvableType};
use genco::prelude::*;

impl ModuleGenerator<'_> {
    fn write_enum_codec(&self, module_path: &Path, enum_type: &EnumType) -> Result<()> {
        let name = enum_type.name.to_case(Case::UpperCamel);
        let values = enum_type.values.as_slice();
        let encoding_type = enum_type
            .encoding_type
            .resolved(&self.schema.types.encoded_types)?;
        let language_primitive_type =
            encoding_type.lang_primitive(&self.schema.types.encoded_types)?;
        let char_encoding = encoding_type == NativeType::Char;

        let null_value = encoding_type.null()?;

        let enum_tokens: Tokens<Rust> = quote! {
            use crate::error::*;
            use std::any::type_name;
            use std::convert::TryFrom;

            #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
            #[repr($language_primitive_type)]
            pub enum $(&name)  {
                $(for value in values {
                    $['\r']
                    $(value.name.to_case(Case::UpperCamel)) = $(value.encoded_value(char_encoding)?)_$language_primitive_type,
                })
                NullVal = $(&null_value),
            }

            impl Default for $(&name)  {
                #[inline]
                fn default() -> Self {
                    Self::NullVal
                }
            }

            impl TryFrom<$language_primitive_type> for $(&name) {
                type Error = SbeError;

                #[inline]
                fn try_from(v: $language_primitive_type) -> Result<Self> {
                    Ok(match v {
                        $(for value in values {
                            $['\r']
                            $(value.encoded_value(char_encoding)?)_$language_primitive_type => Self::$(value.name.to_case(Case::UpperCamel)),
                        })
                        $(&null_value) => Self::NullVal,
                        _ => return Err(SbeError::InvalidEnumValue {
                            type_name: type_name::<Self>(),
                            value: format!("{v}"),
                        })
                    })
                }
            }
        };

        let file_path = module_path.join(format!("{}.rs", name.to_case(Case::Snake)));
        write_file(&file_path, &self.config, enum_tokens)?;

        Ok(())
    }

    pub fn write_enum_codecs(&self) -> Result<()> {
        let module_path = self.path.join(ENUM_MODULE_NAME);
        create_dir_all(&module_path)?;

        let mut module_tokens: Tokens<Rust> = quote!();

        for (name, enum_type) in &self.schema.types.enum_types {
            self.write_enum_codec(&module_path, enum_type)?;

            module_tokens.append(quote! {
                pub mod $(name.to_case(Case::Snake));
                pub use self::$(name.to_case(Case::Snake))::$(name.to_case(Case::UpperCamel));
            });
            module_tokens.push();
        }

        write_file(&module_path.join("mod.rs"), &self.config, module_tokens)?;

        Ok(())
    }
}
