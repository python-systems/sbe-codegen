mod common;
pub mod python;
pub mod rust;

use anyhow::Result;
use genco::fmt;
use genco::fmt::IoWriter;
use genco::prelude::*;
use std::fs::File;
use std::path::Path;

pub trait CodeGenerator {
    fn generate_project(&self, with_test_dependencies: bool, format_project: bool) -> Result<()>;
}

fn write_file<L: Lang>(path: &Path, config: &L::Config, content: Tokens<L>) -> Result<()> {
    let file = File::create(path)?;
    let mut writer = IoWriter::new(file);
    let fmt_config = fmt::Config::from_lang::<L>().with_indentation(fmt::Indentation::Space(4));

    content.format_file(&mut writer.as_formatter(&fmt_config), config)?;

    Ok(())
}
