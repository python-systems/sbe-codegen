use crate::generator::python::PythonGenerator;
use crate::generator::rust::RustGenerator;
use crate::generator::CodeGenerator;
use crate::models::schema::{MessageSchema, ValidatedMessageSchema};
use anyhow::{Context, Result};
use clap::{Parser, ValueEnum};
use std::path::{Path, PathBuf};
use xml_include::resolve_xml_includes;

mod generator;
#[allow(clippy::needless_late_init)]
mod models;

#[derive(ValueEnum, Copy, Clone)]
enum Language {
    Rust,
    Python,
}

impl Language {
    fn generator(
        &self,
        schemas: Vec<ValidatedMessageSchema>,
        project_name: &str,
        project_path: &Path,
        version: &str,
    ) -> Box<dyn CodeGenerator> {
        match self {
            Language::Rust => Box::new(RustGenerator::new(
                project_path,
                project_name,
                version,
                schemas,
                true,
            )),
            Language::Python => Box::new(PythonGenerator::new(
                project_path,
                project_name,
                version,
                schemas,
            )),
        }
    }
}

#[derive(Parser)]
#[command(name = "sbe-codegen", about = "SBE multi-language codec generator")]
struct Opt {
    /// Path to schema file
    #[arg(long = "schema", help = "Path to XML SBE schema")]
    schema_paths: Vec<PathBuf>,

    /// Language
    #[arg(long = "language", help = "Codec language")]
    language: Language,

    /// Project name
    #[arg(long = "project-name", help = "Project name")]
    project_name: String,

    /// Project path
    #[arg(long = "project-path", help = "Project path")]
    project_path: PathBuf,

    /// Version
    #[arg(
        long = "project-version",
        help = "Project version (optional, taken from schema if not specified)"
    )]
    version: Option<String>,

    /// Test dependencies
    #[arg(long = "with-test-deps", help = "Include test dependencies")]
    test_dependencies: bool,

    /// Format project
    #[arg(long = "format", help = "Format project")]
    format: bool,
}

fn validate_schemas(schema_paths: &Vec<PathBuf>) -> Vec<ValidatedMessageSchema> {
    let mut schemas = Vec::new();

    for schema_path in schema_paths {
        let merged_content = resolve_xml_includes(schema_path)
            .context("failed to resolve XML includes")
            .unwrap();

        let schema = MessageSchema::load_from_string(&merged_content)
            .context("failed to load XML schema")
            .unwrap();

        let validated_schema = schema
            .validate()
            .context("failed to validate schema")
            .unwrap();
        schemas.push(validated_schema);
    }

    schemas
}

fn main() -> Result<()> {
    let opt: Opt = Opt::parse();

    let schemas = validate_schemas(&opt.schema_paths);

    if schemas.is_empty() {
        return Err(anyhow::anyhow!("No schemas found"));
    }

    // TODO: A better way to determine project version
    // It does not matter now, because the release pipeline will set the version
    // regardless, but this is not pretty.
    let version = opt.version.unwrap_or(schemas[0].semantic_version.clone());

    let generator = opt
        .language
        .generator(schemas, &opt.project_name, &opt.project_path, &version);

    generator.generate_project(opt.test_dependencies, opt.format)
}
