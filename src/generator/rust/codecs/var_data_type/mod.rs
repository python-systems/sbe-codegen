use crate::generator::common::FieldMetadata;
use crate::generator::rust::codecs::var_data_type::decoder::RustVariableDataDecoderGenerator;
use crate::generator::rust::codecs::var_data_type::encoder::RustVariableDataEncoderGenerator;
use crate::generator::rust::constants::VAR_DATA_MODULE_NAME;
use crate::generator::rust::module::ModuleGenerator;
use crate::generator::write_file;
use crate::models::types::composite_type::CompositeType;
use crate::models::types::variable_data_type::VariableDataType;
use crate::models::types::Type;
use crate::models::TypeMap;
use anyhow::{anyhow, Result};
use convert_case::{Case, Casing};
use genco::prelude::*;
use std::fs::create_dir_all;
use std::path::Path;

pub mod decoder;
mod encoder;

impl ModuleGenerator<'_> {
    fn var_data_encoder_generator<'a>(
        &'a self,
        module_path: &'a Path,
    ) -> RustVariableDataEncoderGenerator<'a> {
        RustVariableDataEncoderGenerator {
            config: &self.config,
            path: module_path,
            types: &self.schema.types,
            package: &self.schema.package,
        }
    }

    fn var_data_decoder_generator<'a>(
        &'a self,
        module_path: &'a Path,
    ) -> RustVariableDataDecoderGenerator<'a> {
        RustVariableDataDecoderGenerator {
            config: &self.config,
            path: module_path,
            types: &self.schema.types,
            package: &self.schema.package,
        }
    }

    fn write_var_data_codec(
        &self,
        module_path: &Path,
        var_data_type: &VariableDataType,
    ) -> Result<()> {
        let name = var_data_type.name.as_str();
        let module_path = module_path.join(name.to_case(Case::Snake));
        create_dir_all(&module_path)?;

        let encoder_generator = self.var_data_encoder_generator(&module_path);
        let decoder_generator = self.var_data_decoder_generator(&module_path);

        let composite_tokens: Tokens<Rust> = quote! {
            mod decoder;
            mod encoder;

            pub use self::decoder::$(name.to_case(Case::UpperCamel))Decoder;
            pub use self::encoder::$(name.to_case(Case::UpperCamel))Encoder;
        };

        encoder_generator.write_encoder(var_data_type)?;
        decoder_generator.write_decoder(var_data_type)?;

        write_file(&module_path.join("mod.rs"), &self.config, composite_tokens)?;

        Ok(())
    }

    pub fn write_var_data_codecs(&self) -> Result<()> {
        let module_path = self.path.join(VAR_DATA_MODULE_NAME);
        create_dir_all(&module_path)?;

        let mut module_tokens: Tokens<Rust> = quote!();

        for (name, var_data_type) in &self.schema.message_types.variable_data_types {
            self.write_var_data_codec(&module_path, var_data_type)?;

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

pub fn repr_type_metadata(
    var_data_name: &str,
    repr_type: &CompositeType,
    types: &TypeMap,
) -> Result<(FieldMetadata<Rust>, FieldMetadata<Rust>)> {
    let length_type = match &repr_type.fields[0] {
        Type::EncodedData(length_type) => length_type,
        _ => {
            return Err(anyhow!(
            "Only encoded data type expected for the length type in variable data encoding '{}'",
            var_data_name
        ))
        }
    };
    let length_type_metadata = FieldMetadata::from("", length_type, types)?;

    let value_type = match &repr_type.fields[1] {
        Type::EncodedData(value_type) => value_type,
        _ => {
            return Err(anyhow!(
                "Only encoded data type expected for the value type in variable data encoding '{}'",
                var_data_name
            ))
        }
    };
    let value_type_metadata = FieldMetadata::from("", value_type, types)?;

    Ok((length_type_metadata, value_type_metadata))
}
