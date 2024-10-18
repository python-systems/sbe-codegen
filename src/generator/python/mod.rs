mod codecs;
mod constants;
mod module;
mod typing;

use crate::generator::{write_file, CodeGenerator};
use crate::models::schema::ValidatedMessageSchema;
use anyhow::Result;
use convert_case::{Case, Casing};
use indoc::formatdoc;
use std::collections::HashMap;
use std::fs::{create_dir_all, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};

use crate::models::types::encoded_data_type::EncodedDataType;

use crate::generator::python::constants::{PYSRC_DIR, SRC_DIR};
use crate::generator::python::typing::TypingGenerator;
use crate::generator::rust::RustGenerator;
use crate::models::message::MessageType;
use crate::models::types::primitive_type::{
    LanguagePrimitive, NativeType, PrimitiveConvertible, ResolvableType,
};
use crate::models::types::{MessageField, Type};
use genco::prelude::*;

enum ExportedClass {
    Message(MessageType),
    MessageField(MessageField),
    Type(Type),
}

pub struct PythonGenerator {
    config: rust::Config,
    path: PathBuf,
    project_name: String,
    project_version: String,
    schemas: Vec<ValidatedMessageSchema>,
}

impl PythonGenerator {
    pub fn new(
        path: &Path,
        project_name: &str,
        project_version: &str,
        schemas: Vec<ValidatedMessageSchema>,
    ) -> Self {
        Self {
            config: rust::Config::default().with_default_import(rust::ImportMode::Direct),
            path: path.to_owned(),
            project_name: project_name.to_owned(),
            project_version: project_version.to_owned(),
            schemas,
        }
    }

    fn write_project_files(&self, with_test_dependencies: bool) -> Result<()> {
        create_dir_all(self.path.join(SRC_DIR))?;
        create_dir_all(self.path.join(PYSRC_DIR).join(&self.project_name))?;

        self.write_cargo_toml()?;
        self.write_pyproject_toml(with_test_dependencies)?;

        let mut modules = Vec::new();

        for schema in &self.schemas {
            create_dir_all(
                self.path
                    .join(PYSRC_DIR)
                    .join(&self.project_name)
                    .join(&schema.package),
            )?;

            let mut exported_classes = HashMap::new();

            exported_classes.extend(schema.types.iter_values().filter_map(|simple_type| {
                if !matches!(simple_type, Type::EncodedData(_)) {
                    (
                        simple_type.name().to_case(Case::UpperCamel),
                        ExportedClass::Type(simple_type.clone()),
                    )
                        .into()
                } else {
                    None
                }
            }));
            exported_classes.extend(schema.message_types.iter_values().filter_map(
                |message_type| {
                    if matches!(message_type, MessageField::Group(_)) {
                        (
                            message_type.name().to_case(Case::UpperCamel),
                            ExportedClass::MessageField(message_type.clone()),
                        )
                            .into()
                    } else {
                        None
                    }
                },
            ));
            exported_classes.extend(schema.message_types.message_types.values().map(
                |message_type| {
                    (
                        message_type.name.to_case(Case::UpperCamel),
                        ExportedClass::Message(message_type.clone()),
                    )
                },
            ));

            modules.push(self.generate_pymodule(schema, &exported_classes));

            self.write_schema_init_py(schema, &exported_classes)?;
            TypingGenerator::from_python(self, schema).write_typing_hints(&exported_classes)?;
        }

        self.write_init_py()?;
        self.write_lib_rs(modules)
    }

    fn write_cargo_toml(&self) -> Result<()> {
        let cargo_toml_content = formatdoc! {"
                [package]
                name = \"{name}\"
                version = \"{version}\"
                authors = [\"Second Foundation\"]
                edition = \"2021\"

                [lib]
                name = \"{name}\"
                crate-type = [\"cdylib\"]

                [workspace]

                [dependencies]
                pyo3 = {{ version = \"^0.22.0\", features = [\"anyhow\", \"extension-module\", \"generate-import-lib\"] }}
                anyhow = \"^1.0\"
                rust_codecs = {{ path = \"rust_codecs\" }}
            ",
            name = self.project_name,
            version = self.project_version,
        };

        let cargo_toml_path = self.path.join("Cargo.toml");
        let mut cargo_toml_file = File::create(cargo_toml_path)?;
        cargo_toml_file.write_all(cargo_toml_content.as_bytes())?;

        Ok(())
    }

    fn write_pyproject_toml(&self, with_test_dependencies: bool) -> Result<()> {
        let test_dependencies = formatdoc! {"
            [tool.poetry.group.dev.dependencies]
            pytest = \"^7\"
            pytest-benchmark = \"^4\"
            pydantic = \"^2.5.2\"
        "};

        let cargo_toml_content = formatdoc! {"
                [build-system]
                requires = [\"maturin>=1.3,<2.0\"]
                build-backend = \"maturin\"

                [project]
                name = \"{name}\"
                requires-python = \">=3.10\"
                classifiers = [
                    \"Programming Language :: Rust\",
                    \"Programming Language :: Python :: Implementation :: CPython\",
                    \"Programming Language :: Python :: Implementation :: PyPy\",
                ]
                dynamic = [\"version\"]

                [tool.maturin]
                python-source = \"pysrc\"
                features = [\"pyo3/extension-module\"]

                [tool.poetry]
                name = \"{name}\"
                version = \"{version}\"
                license = \"Proprietary\"
                authors = [\"Second Foundation\"]
                description = \"\"

                [tool.poetry.dependencies]
                python = \"^3.10\"
                maturin = \">=1.3,<2\"

                {test_deps}
            ",
            name = self.project_name,
            version = self.project_version,
            test_deps = if with_test_dependencies {
                test_dependencies
            } else {
                String::new()
            },
        };

        let pyproject_toml_path = self.path.join("pyproject.toml");
        let mut pyproject_toml_file = File::create(pyproject_toml_path)?;
        pyproject_toml_file.write_all(cargo_toml_content.as_bytes())?;

        Ok(())
    }

    fn generate_rust_codecs(&self, format_project: bool) -> Result<()> {
        let rust_lib_path = self.path.join("rust_codecs");

        let rust_generator = RustGenerator::new(
            &rust_lib_path,
            "rust_codecs",
            &self.project_version,
            self.schemas.clone(),
            false,
        );
        rust_generator.generate_project(false, format_project)?;

        Ok(())
    }

    fn generate_pymodule(
        &self,
        schema: &ValidatedMessageSchema,
        exported_classes: &HashMap<String, ExportedClass>,
    ) -> Tokens<Rust> {
        quote! {
            fn $(&schema.package)_submodule(py: Python) -> PyResult<Bound<PyModule>> {
                use $(&schema.package)::enums::*;
                use $(&schema.package)::sets::*;
                use $(&schema.package)::groups::*;
                use $(&schema.package)::messages::*;
                use $(&schema.package)::composites::*;

                let module = PyModule::new_bound(py, $[str]($[const](&schema.package)))?;

                $(for class_name in exported_classes.keys() {
                    $['\r']
                    module.add_class::<$class_name>()?;
                })

                Ok(module)
            }
        }
    }

    fn write_lib_rs(&self, modules: Vec<Tokens<Rust>>) -> Result<()> {
        let lib_rs_content: Tokens<Rust> = quote! {
            #![allow(clippy::upper_case_acronyms)]
            #![allow(clippy::too_many_arguments)]

            use pyo3::prelude::*;

            $(for schema in &self.schemas {
                $['\r']
                mod $(&schema.package);
            })

            #[pymodule]
            fn $(&self.project_name)(py: Python, m: &Bound<PyModule>) -> PyResult<()> {
                $(for schema in &self.schemas {
                    $['\r']
                    let $(&schema.package) = $(&schema.package)_submodule(py)?;

                    py.import_bound("sys")?
                        .getattr("modules")?
                        .set_item($[str]($[const](&self.project_name).$[const](&schema.package)), &$(&schema.package))?;
                    m.add_submodule(&$(&schema.package)_submodule(py)?)?;
                })

                Ok(())
            }

            $(for module in modules {
                $['\r']
                $module
            })
        };

        write_file(&self.path.join("src/lib.rs"), &self.config, lib_rs_content)
    }

    fn write_schema_init_py(
        &self,
        schema: &ValidatedMessageSchema,
        exported_classes: &HashMap<String, ExportedClass>,
    ) -> Result<()> {
        let init_py_content: Tokens<Python> = quote! {
            from . import *

            SCHEMA_ID: int = $(schema.id)
            SCHEMA_VERSION: int = $(schema.version)

            __all__ = [
                $(for class_name in exported_classes.keys() {
                    $['\r']
                    $(quoted(class_name)),
                })
            ]
        };

        write_file(
            &self
                .path
                .join(PYSRC_DIR)
                .join(&self.project_name)
                .join(&schema.package)
                .join("__init__.py"),
            &self.python_config(),
            init_py_content,
        )
    }
    fn write_init_py(&self) -> Result<()> {
        // Write py.typed
        write_file::<Python>(
            &self
                .path
                .join(PYSRC_DIR)
                .join(&self.project_name)
                .join("py.typed"),
            &self.python_config(),
            quote!(),
        )?;

        // Write __init__.py
        let init_py_content: Tokens<Python> = quote! {
            $(for schema in &self.schemas {
                from .$(&self.project_name) import $(&schema.package)
                $['\r']
            })

            __all__ = [
                $(for schema in &self.schemas {
                    $['\r']
                    $(quoted(&schema.package)),
                })
            ]
        };

        write_file(
            &self
                .path
                .join(PYSRC_DIR)
                .join(&self.project_name)
                .join("__init__.py"),
            &self.python_config(),
            init_py_content,
        )
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

    fn python_config(&self) -> python::Config {
        python::Config::default()
    }
}

impl CodeGenerator for PythonGenerator {
    fn generate_project(&self, with_test_dependencies: bool, format_project: bool) -> Result<()> {
        // Project files (Cargo.toml, ...)
        self.write_project_files(with_test_dependencies)?;

        self.generate_rust_codecs(format_project)?;

        for schema in &self.schemas {
            let module_generator = module::ModuleGenerator::new(
                &self.path.join(SRC_DIR).join(&schema.package),
                schema,
            );

            module_generator.generate_module()?;
        }

        if format_project {
            self.format_project();
        }

        Ok(())
    }
}

impl PrimitiveConvertible<Python> for NativeType {
    fn lang_primitive(
        &self,
        encoded_types: &HashMap<String, EncodedDataType>,
    ) -> Result<LanguagePrimitive<Python>> {
        let native_type = self.resolved(encoded_types)?;

        Ok(LanguagePrimitive::new(match native_type {
            NativeType::Char
            | NativeType::UInt8
            | NativeType::UInt16
            | NativeType::UInt32
            | NativeType::UInt64
            | NativeType::Int8
            | NativeType::Int16
            | NativeType::Int32
            | NativeType::Int64 => "int",
            NativeType::Float | NativeType::Double => "float",
            NativeType::Reference(type_name) => unreachable!("Resolved reference: {}", type_name),
        }))
    }
}
