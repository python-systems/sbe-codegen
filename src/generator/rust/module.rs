use crate::generator::write_file;
use crate::models::schema::ValidatedMessageSchema;
use anyhow::Result;
use genco::lang::{rust, Rust};
use genco::{quote, Tokens};
use std::fs::create_dir_all;
use std::path::{Path, PathBuf};

// TODO: do not make everything pub
pub struct ModuleGenerator<'a> {
    pub(crate) config: rust::Config,
    pub(crate) path: PathBuf,
    pub(crate) schema: &'a ValidatedMessageSchema,
}

impl<'a> ModuleGenerator<'a> {
    pub fn new(path: &Path, schema: &'a ValidatedMessageSchema) -> Self {
        Self {
            config: rust::Config::default().with_default_import(rust::ImportMode::Direct),
            path: path.to_owned(),
            schema,
        }
    }

    pub fn generate_module(&self) -> Result<()> {
        create_dir_all(&self.path)?;

        self.write_mod_rs()?;

        // Universal traits needed for encoding/decoding
        self.generate_encoder_traits()?;
        self.generate_decoder_traits()?;

        // Type specific encoders and decoders
        self.write_enum_codecs()?;
        self.write_set_codecs()?;
        self.write_composite_codecs()?;

        self.write_group_codecs()?;
        self.write_var_data_codecs()?;
        self.write_message_codecs()
    }

    fn write_mod_rs(&self) -> Result<()> {
        let mod_rs_content: Tokens<Rust> = quote! {
            pub mod encoder;
            pub mod decoder;
            pub mod sets;
            pub mod enums;
            pub mod composites;
            pub mod groups;
            pub mod var_data;
            pub mod messages;

            pub const SCHEMA_ID: u16 = $(self.schema.id);
            pub const SCHEMA_VERSION: u16 = $(self.schema.version);
        };

        write_file(&self.path.join("mod.rs"), &self.config, mod_rs_content)
    }
}
