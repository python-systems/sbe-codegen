use anyhow::Result;
use convert_case::{Case, Casing};
use std::fs::create_dir_all;
use std::path::Path;

use crate::models::types::set_type::{Choice, SetType};

use crate::generator::rust::constants::SET_MODULE_NAME;
use crate::generator::rust::module::ModuleGenerator;
use crate::generator::write_file;
use crate::models::types::primitive_type::PrimitiveConvertible;
use genco::prelude::*;

impl ModuleGenerator<'_> {
    fn generate_choice_token(choice: &Choice) -> impl FormatInto<Rust> {
        quote! {
            $['\n']
            #[inline]
            pub fn get_$(choice.name.to_case(Case::Snake))(&self) -> bool {
                0 != self.0 & (1 << $(choice.value))
            }

            #[inline]
            pub fn set_$(choice.name.to_case(Case::Snake))(&mut self, value: bool) -> &mut Self {
                self.0 = if value {
                    self.0 | (1 << $(choice.value))
                } else {
                    self.0 & !(1 << $(choice.value))
                };
                self
            }
        }
    }

    fn write_set_codec(&self, module_path: &Path, set_type: &SetType) -> Result<()> {
        let name = set_type.name.to_case(Case::UpperCamel);
        let choices = set_type.choices.as_slice();
        let rust_type = set_type
            .encoding_type
            .lang_primitive(&self.schema.types.encoded_types)?;

        let set_tokens: Tokens<Rust> = quote! {
            #[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
            pub struct $(&name)(pub $(rust_type));

            impl $(&name) {
                #[inline]
                pub fn clear(&mut self) -> &mut Self {
                    self.0 = 0;
                    self
                }

                $(for choice in choices => $(Self::generate_choice_token(choice)))
            }
        };

        let file_path = module_path.join(format!("{}.rs", name.to_case(Case::Snake)));
        write_file(&file_path, &self.config, set_tokens)?;

        Ok(())
    }

    pub fn write_set_codecs(&self) -> Result<()> {
        let module_path = self.path.join(SET_MODULE_NAME);
        create_dir_all(&module_path)?;

        let mut module_tokens: Tokens<Rust> = quote!();

        for (name, set_type) in &self.schema.types.set_types {
            self.write_set_codec(&module_path, set_type)?;

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
