use crate::models::types::group_type::GroupType;
use anyhow::{anyhow, Result};
use convert_case::{Case, Casing};
use std::collections::HashMap;
use std::fs::create_dir_all;
use std::path::Path;

use crate::generator::rust::codecs::group_type::decoder::RustGroupDecoderGenerator;
use crate::generator::rust::codecs::group_type::encoder::RustGroupEncoderGenerator;
use crate::generator::rust::constants::GROUP_MODULE_NAME;
use crate::generator::rust::module::ModuleGenerator;
use crate::generator::write_file;
use crate::models::types::composite_type::CompositeType;
use genco::prelude::*;

pub mod decoder;
pub mod encoder;

impl ModuleGenerator<'_> {
    fn group_encoder_generator<'a>(
        &'a self,
        module_path: &'a Path,
    ) -> RustGroupEncoderGenerator<'a> {
        RustGroupEncoderGenerator {
            config: &self.config,
            path: module_path,
            types: &self.schema.types,
            package: &self.schema.package,
        }
    }

    fn group_decoder_generator<'a>(
        &'a self,
        module_path: &'a Path,
    ) -> RustGroupDecoderGenerator<'a> {
        RustGroupDecoderGenerator {
            config: &self.config,
            path: module_path,
            types: &self.schema.types,
            package: &self.schema.package,
        }
    }

    fn write_group_codece(&self, module_path: &Path, group_type: &GroupType) -> Result<()> {
        let name = group_type.name.as_str();
        let module_path = module_path.join(name.to_case(Case::Snake));
        create_dir_all(&module_path)?;

        let encoder_generator = self.group_encoder_generator(&module_path);
        let decoder_generator = self.group_decoder_generator(&module_path);

        let composite_tokens: Tokens<Rust> = quote! {
            mod decoder;
            mod encoder;

            pub use self::decoder::$(name.to_case(Case::UpperCamel))Decoder;
            pub use self::encoder::$(name.to_case(Case::UpperCamel))Encoder;
        };

        encoder_generator.write_encoder(group_type)?;
        decoder_generator.write_decoder(group_type)?;

        write_file(&module_path.join("mod.rs"), &self.config, composite_tokens)?;

        Ok(())
    }

    pub fn write_group_codecs(&self) -> Result<()> {
        let module_path = self.path.join(GROUP_MODULE_NAME);
        create_dir_all(&module_path)?;

        let mut module_tokens: Tokens<Rust> = quote!();

        for (name, group_type) in &self.schema.message_types.group_types {
            self.write_group_codece(&module_path, group_type)?;

            module_tokens.append(quote! {
                pub mod $(name.to_case(Case::Snake));
                pub use self::$(name.to_case(Case::Snake))::*;
            });
            module_tokens.push();
        }

        write_file(&module_path.join("mod.rs"), &self.config, module_tokens)?;

        Ok(())
    }
}

fn dimension_type<'a>(
    group: &GroupType,
    composite_types: &'a HashMap<String, CompositeType>,
) -> Result<&'a CompositeType> {
    let dimension_type_name = group
        .dimension_type
        .clone()
        .unwrap_or("groupSizeEncoding".to_owned());

    composite_types.get(&dimension_type_name).ok_or(anyhow!(
        "Missing dimension type '{}' for group '{}'",
        dimension_type_name,
        group.name
    ))
}
