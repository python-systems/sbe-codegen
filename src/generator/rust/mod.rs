mod codecs;
mod constants;
mod decoder;
mod encoder;
mod error;
mod module;

use crate::generator::{write_file, CodeGenerator};
use crate::models::schema::ValidatedMessageSchema;
use anyhow::Result;
use indoc::formatdoc;
use std::collections::HashMap;
use std::fs::{create_dir_all, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};

use crate::models::types::encoded_data_type::EncodedDataType;

use crate::models::types::primitive_type::{
    LanguagePrimitive, NativeType, PrimitiveConvertible, ResolvableType,
};
use genco::prelude::*;

pub struct RustGenerator {
    config: rust::Config,
    path: PathBuf,
    project_name: String,
    project_version: String,
    workspace_root: bool,
    schemas: Vec<ValidatedMessageSchema>,
}

impl RustGenerator {
    pub fn new(
        path: &Path,
        project_name: &str,
        project_version: &str,
        schemas: Vec<ValidatedMessageSchema>,
        workspace_root: bool,
    ) -> Self {
        Self {
            config: rust::Config::default().with_default_import(rust::ImportMode::Direct),
            path: path.to_owned(),
            project_name: project_name.to_owned(),
            project_version: project_version.to_owned(),
            workspace_root,
            schemas,
        }
    }

    fn write_project_files(&self, with_test_dependencies: bool) -> Result<()> {
        create_dir_all(self.path.join("src"))?;

        let test_dependencies = formatdoc! {"
            [dev-dependencies]
            serde_json = \"^1.0\"
            rstest = \"^0.23.0\"
            proptest = \"^1.4.0\"
            criterion = \"^0.5.1\"
            time = {{ version = \"^0.3\", features = [\"parsing\"] }}
        "};

        let cargo_toml_content = formatdoc! {"
                [package]
                name = \"{name}\"
                version = \"{version}\"
                authors = [\"Second Foundation\"]
                edition = \"2021\"

                {workspace}

                [dependencies]
                thiserror = \"^1.0\"

                {test_deps}
            ",
            name = self.project_name,
            version = self.project_version,
            workspace = if self.workspace_root {
                "[workspace]".to_owned()
            } else {
                String::new()
            },
            test_deps = if with_test_dependencies {
                test_dependencies
            } else {
                String::new()
            },
        };

        let cargo_toml_path = self.path.join("Cargo.toml");
        let mut cargo_toml_file = File::create(cargo_toml_path)?;
        cargo_toml_file.write_all(cargo_toml_content.as_bytes())?;

        self.write_lib_rs()?;
        self.write_error_module()
    }

    fn write_lib_rs(&self) -> Result<()> {
        let lib_rs_content: Tokens<Rust> = quote! {
            pub mod error;

            $(for schema in &self.schemas {
                $['\r']
                pub mod $(&schema.package);
            })
        };

        write_file(&self.path.join("src/lib.rs"), &self.config, lib_rs_content)
    }

    fn format_project(&self) {
        // It was decided to ignore the possible failure of formatting.
        // If user did not install Rust, this will fail, but it is not critical.
        let clippy_result = self.run_cargo_clippy();
        let fmt_result = self.run_cargo_fmt();

        if fmt_result.is_err() || clippy_result.is_err() {
            eprintln!("Failed to format project. Do you have rustfmt/clippy installed?");
        }
    }

    fn run_cargo_fmt(&self) -> Result<()> {
        let fmt_status = Command::new("cargo")
            .arg("fmt")
            .current_dir(&self.path)
            .status()?;

        if !fmt_status.success() {
            return Err(anyhow::anyhow!("cargo fmt failed"));
        }

        Ok(())
    }

    fn run_cargo_clippy(&self) -> Result<()> {
        // multiple passes of clippy are needed, first fix uncovers more errors to fix.
        let mut status = ExitStatus::default();
        loop {
            let clippy_status = Command::new("cargo")
                .arg("clippy")
                .arg("--fix")
                .arg("--allow-dirty")
                .arg("--allow-staged")
                .arg("--allow-no-vcs")
                .arg("--")
                .arg("-Dwarnings")
                .current_dir(&self.path)
                .status()?;

            if clippy_status == status {
                break;
            }

            status = clippy_status;
        }

        if !status.success() {
            return Err(anyhow::anyhow!("cargo clippy failed"));
        }

        Ok(())
    }
}

impl CodeGenerator for RustGenerator {
    fn generate_project(&self, with_test_dependencies: bool, format_project: bool) -> Result<()> {
        // Project files (Cargo.toml, ...)
        self.write_project_files(with_test_dependencies)?;

        // Generate modules
        for schema in &self.schemas {
            let module_generator =
                module::ModuleGenerator::new(&self.path.join("src").join(&schema.package), schema);

            module_generator.generate_module()?;
        }

        if format_project {
            self.format_project();
        }

        Ok(())
    }
}

impl PrimitiveConvertible<Rust> for NativeType {
    fn lang_primitive(
        &self,
        encoded_types: &HashMap<String, EncodedDataType>,
    ) -> Result<LanguagePrimitive<Rust>> {
        let native_type = self.resolved(encoded_types)?;

        Ok(LanguagePrimitive::new(match native_type {
            NativeType::Char | NativeType::UInt8 => "u8",
            NativeType::UInt16 => "u16",
            NativeType::UInt32 => "u32",
            NativeType::UInt64 => "u64",
            NativeType::Int8 => "i8",
            NativeType::Int16 => "i16",
            NativeType::Int32 => "i32",
            NativeType::Int64 => "i64",
            NativeType::Float => "f32",
            NativeType::Double => "f64",
            NativeType::Reference(type_name) => unreachable!("Resolved reference: {}", type_name),
        }))
    }
}
