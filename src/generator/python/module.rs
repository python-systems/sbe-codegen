use crate::generator::write_file;
use crate::models::schema::ValidatedMessageSchema;
use anyhow::Result;
use genco::lang::{rust, Rust};
use genco::{quote, Tokens};
use std::fs::create_dir_all;
use std::path::{Path, PathBuf};

pub struct ModuleGenerator<'a> {
    pub config: rust::Config,
    pub path: PathBuf,
    pub schema: &'a ValidatedMessageSchema,
}

impl<'a> ModuleGenerator<'a> {
    pub fn new(path: &Path, schema: &'a ValidatedMessageSchema) -> Self {
        Self {
            config: rust::Config::default().with_default_import(rust::ImportMode::Direct),
            path: path.to_owned(),
            schema,
        }
    }

    fn generate_mod_rs(&self) -> Result<()> {
        let mod_rs_content: Tokens<Rust> = quote! {
            pub mod sets;
            pub mod enums;
            pub mod composites;
            pub mod groups;
            pub mod messages;
        };

        write_file(&self.path.join("mod.rs"), &self.config, mod_rs_content)
    }

    pub fn generate_module(&self) -> Result<()> {
        create_dir_all(&self.path)?;

        self.generate_mod_rs()?;

        // Type specific encoders and decoders
        self.write_enum_codecs()?;
        self.write_set_codecs()?;
        self.write_composite_codecs()?;

        self.write_group_codecs()?;
        self.write_message_codecs()
    }
}
