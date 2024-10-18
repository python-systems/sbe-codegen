pub mod decoder;
pub mod encoder;

use crate::generator::rust::constants::COMPOSITE_MODULE_NAME;
use anyhow::Result;
use convert_case::{Case, Casing};
use std::fs::create_dir_all;
use std::path::Path;

use crate::generator::rust::codecs::composite_type::decoder::RustCompositeDecoderGenerator;
use crate::generator::rust::codecs::composite_type::encoder::RustCompositeEncoderGenerator;
use crate::generator::write_file;
use crate::models::types::composite_type::CompositeType;
use crate::models::types::primitive_type::NativeType;
use crate::models::types::SizedEncoded;

use crate::generator::rust::module::ModuleGenerator;
use genco::prelude::*;

impl ModuleGenerator<'_> {
    fn composite_encoder_generator<'a>(
        &'a self,
        module_path: &'a Path,
    ) -> RustCompositeEncoderGenerator<'a> {
        RustCompositeEncoderGenerator {
            config: &self.config,
            path: module_path,
            types: &self.schema.types,
            package: &self.schema.package,
        }
    }

    fn composite_decoder_generator<'a>(
        &'a self,
        module_path: &'a Path,
    ) -> RustCompositeDecoderGenerator<'a> {
        RustCompositeDecoderGenerator {
            config: &self.config,
            path: module_path,
            types: &self.schema.types,
            package: &self.schema.package,
        }
    }

    fn write_composite_codec(
        &self,
        module_path: &Path,
        composite_type: &CompositeType,
    ) -> Result<()> {
        let name = composite_type.name.as_str();
        let module_path = module_path.join(name.to_case(Case::Snake));
        create_dir_all(&module_path)?;

        let encoder_generator = self.composite_encoder_generator(&module_path);
        let decoder_generator = self.composite_decoder_generator(&module_path);

        let composite_tokens: Tokens<Rust> = quote! {
            mod decoder;
            mod encoder;

            pub use self::decoder::$(name.to_case(Case::UpperCamel))Decoder;
            pub use self::encoder::$(name.to_case(Case::UpperCamel))Encoder;

            pub const $(name.to_case(Case::ScreamingSnake))_ENCODED_LENGTH: usize = $(composite_type.size(&self.schema.types)?);
        };

        encoder_generator.write_encoder(composite_type)?;
        decoder_generator.write_decoder(composite_type)?;

        write_file(&module_path.join("mod.rs"), &self.config, composite_tokens)?;

        Ok(())
    }

    pub fn write_composite_codecs(&self) -> Result<()> {
        let module_path = self.path.join(COMPOSITE_MODULE_NAME);
        create_dir_all(&module_path)?;

        let mut module_tokens: Tokens<Rust> = quote!();

        for (name, composite_type) in &self.schema.types.composite_types {
            self.write_composite_codec(&module_path, composite_type)?;

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

fn default_value(default_value: &str, field_type: &NativeType) -> String {
    match field_type {
        NativeType::Char => format!("\"{}\"", default_value),
        _ => default_value.to_owned(),
    }
}
