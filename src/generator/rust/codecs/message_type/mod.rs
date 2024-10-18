use crate::generator::rust::codecs::message_type::decoder::RustMessageDecoderGenerator;
use crate::generator::rust::codecs::message_type::encoder::RustMessageEncoderGenerator;
use crate::generator::rust::constants::MESSAGE_MODULE_NAME;
use crate::generator::rust::module::ModuleGenerator;
use crate::generator::write_file;
use crate::models::message::MessageType;
use anyhow::Result;
use convert_case::{Case, Casing};
use genco::lang::Rust;
use genco::{quote, Tokens};
use std::fs::create_dir_all;
use std::path::Path;

pub mod decoder;
pub mod encoder;

impl ModuleGenerator<'_> {
    fn message_encoder_generator<'a>(
        &'a self,
        module_path: &'a Path,
    ) -> RustMessageEncoderGenerator<'a> {
        RustMessageEncoderGenerator {
            config: &self.config,
            path: module_path,
            types: &self.schema.types,
            package: &self.schema.package,
        }
    }

    fn message_decoder_generator<'a>(
        &'a self,
        module_path: &'a Path,
    ) -> RustMessageDecoderGenerator<'a> {
        RustMessageDecoderGenerator {
            config: &self.config,
            path: module_path,
            types: &self.schema.types,
            package: &self.schema.package,
        }
    }

    fn write_message_codec(&self, module_path: &Path, message_type: &MessageType) -> Result<()> {
        let name = message_type.name.as_str();
        let module_path = module_path.join(name.to_case(Case::Snake));
        create_dir_all(&module_path)?;

        let encoder_generator = self.message_encoder_generator(&module_path);
        let decoder_generator = self.message_decoder_generator(&module_path);

        let composite_tokens: Tokens<Rust> = quote! {
            mod decoder;
            mod encoder;

            pub use self::decoder::$(name.to_case(Case::UpperCamel))Decoder;
            pub use self::encoder::$(name.to_case(Case::UpperCamel))Encoder;

            pub const $(name.to_case(Case::ScreamingSnake))_ID: u16 = $(message_type.id);
        };

        encoder_generator.write_encoder(message_type)?;
        decoder_generator.write_decoder(message_type)?;

        write_file(&module_path.join("mod.rs"), &self.config, composite_tokens)?;

        Ok(())
    }

    pub fn write_message_codecs(&self) -> Result<()> {
        let module_path = self.path.join(MESSAGE_MODULE_NAME);
        create_dir_all(&module_path)?;

        let mut module_tokens: Tokens<Rust> = quote!();

        for message_type in self.schema.message_types.message_types.values() {
            self.write_message_codec(&module_path, message_type)?;

            module_tokens.append(quote! {
                pub mod $(message_type.name.to_case(Case::Snake));
                pub use self::$(message_type.name.to_case(Case::Snake))::*;
            });
            module_tokens.push();
        }

        write_file(&module_path.join("mod.rs"), &self.config, module_tokens)?;

        Ok(())
    }
}
