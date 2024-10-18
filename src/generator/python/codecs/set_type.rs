use crate::generator::python::constants::SET_MODULE_NAME;
use crate::generator::python::module::ModuleGenerator;
use crate::generator::write_file;
use crate::models::types::set_type::{Choice, SetType};
use anyhow::Result;
use convert_case::{Case, Casing};
use genco::prelude::*;
use std::fs::create_dir_all;
use std::path::Path;

impl ModuleGenerator<'_> {
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

    fn write_set_codec(&self, module_path: &Path, set_type: &SetType) -> Result<()> {
        let name = set_type.name.to_case(Case::UpperCamel);
        let rust_name = format!("Rust{}", name);
        let choices = set_type.choices.as_slice();

        let set_tokens: Tokens<Rust> = quote! {
            use std::hash::{DefaultHasher, Hash, Hasher};
            use pyo3::{pyclass, pymethods};
            use rust_codecs::$(&self.schema.package)::sets::$(&name) as $(&rust_name);
            use pyo3::pyclass::CompareOp;

            #[pyclass(subclass, eq, ord)]
            #[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
            pub struct $(&name)($(&rust_name));

            #[pymethods]
            impl $(&name) {
                #[new]
                pub fn new() -> Self {
                    Self($(&rust_name)::default())
                }

                fn __hash__(&self) -> u64 {
                    let mut hasher = DefaultHasher::new();
                    self.hash(&mut hasher);
                    hasher.finish()
                }

                $(for choice in choices => $(Self::generate_choice_token(choice)))
            }

            impl From<$(&rust_name)> for $(&name) {
                #[inline]
                fn from(value: $(&rust_name)) -> Self {
                    Self(value)
                }
            }

            impl From<$(&name)> for $(&rust_name) {
                #[inline]
                fn from(value: $(&name)) -> Self {
                    value.0
                }
            }
        };

        let file_path = module_path.join(format!("{}.rs", name.to_case(Case::Snake)));
        write_file(&file_path, &self.config, set_tokens)?;

        Ok(())
    }

    fn generate_choice_token(choice: &Choice) -> impl FormatInto<Rust> {
        quote! {
            $['\n']
            #[inline]
            #[getter]
            pub fn get_$(choice.name.to_case(Case::Snake))(&self) -> bool {
                self.0.get_$(choice.name.to_case(Case::Snake))()
            }

            #[inline]
            #[setter]
            pub fn set_$(choice.name.to_case(Case::Snake))(&mut self, value: bool) {
                self.0.set_$(choice.name.to_case(Case::Snake))(value);
            }
        }
    }
}
