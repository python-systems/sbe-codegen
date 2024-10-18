use crate::generator::python::constants::PYSRC_DIR;
use crate::generator::python::{ExportedClass, PythonGenerator};
use crate::generator::write_file;
use crate::models::message::MessageType;
use crate::models::schema::ValidatedMessageSchema;
use crate::models::types::composite_type::CompositeType;
use crate::models::types::enum_type::EnumType;
use crate::models::types::primitive_type::{
    LanguagePrimitive, NativeType, PrimitiveConvertible, ResolvableType,
};
use crate::models::types::set_type::SetType;
use crate::models::types::{MessageField, Presence, Type};
use crate::models::TypeMap;
use convert_case::{Case, Casing};
use genco::lang::{python, Python};
use genco::prelude::FormatInto;
use genco::{quote, Tokens};
use std::cmp::Ordering;
use std::collections::HashMap;
use std::path::Path;

pub struct TypingGenerator<'a> {
    pub path: &'a Path,
    pub project_name: &'a str,
    pub types: &'a TypeMap,
    pub python_config: python::Config,
    pub schema: &'a ValidatedMessageSchema,
}

impl<'a> TypingGenerator<'a> {
    pub fn from_python(
        python_generator: &'a PythonGenerator,
        schema: &'a ValidatedMessageSchema,
    ) -> Self {
        Self {
            path: &python_generator.path,
            project_name: &python_generator.project_name,
            types: &schema.types,
            python_config: python_generator.python_config(),
            schema,
        }
    }

    pub fn write_typing_hints(
        &self,
        exported_classes: &HashMap<String, ExportedClass>,
    ) -> anyhow::Result<()> {
        let mut pyi_tokens: Tokens<Python> = quote! {
            from enum import Enum
            from typing import ClassVar
        };

        for (class_name, class) in exported_classes {
            pyi_tokens.line();

            let class_tokens: Tokens<Python> = match class {
                ExportedClass::Message(message) => quote! {
                    class $class_name:
                        ID: ClassVar[int]

                        $(self.write_fields(&message.fields, false)?)

                        def __init__(
                            self,
                            $(self.write_fields(&message.fields, true)?)
                        ) -> None:
                            ...

                        $(self.write_message_functions(message)?)
                },
                ExportedClass::MessageField(MessageField::Group(group_type)) => quote! {
                    class $class_name:
                        $(self.write_fields(&group_type.fields, false)?)

                        def __init__(
                            self,
                            $(self.write_fields(&group_type.fields, true)?)
                        ) -> None:
                            ...
                },
                ExportedClass::Type(Type::Composite(composite)) => quote! {
                    class $class_name:
                        $(self.write_composite_fields(composite, false)?)

                        def __init__(
                            self,
                            $(self.write_composite_fields(composite, true)?)
                        ) -> None:
                            ...
                },
                ExportedClass::Type(Type::Set(set)) => quote! {
                    class $class_name:
                        $(self.write_set_fields(set)?)
                },
                ExportedClass::Type(Type::Enum(enum_type)) => quote! {
                    class $class_name(Enum):
                        $(self.write_enum_fields(enum_type)?)
                },
                _ => unreachable!("Only messages, groups, composites, sets and enums are exported"),
            };

            pyi_tokens.append(class_tokens);
        }

        write_file(
            &self
                .path
                .join(PYSRC_DIR)
                .join(self.project_name)
                .join(&self.schema.package)
                .join("__init__.pyi"),
            &self.python_config,
            pyi_tokens,
        )
    }

    fn write_fields(
        &self,
        fields: &[MessageField],
        init: bool,
    ) -> anyhow::Result<impl FormatInto<Python>> {
        if fields.is_empty() {
            return Ok(quote! {
                $(if !init {pass})
            });
        }

        let mut fields_tokens: Tokens<Python> = quote!();

        // Here, we need fields to be in order:
        // Fields, Groups, VariableData, Optionals.
        // However, optionals are mixed in. So we sort the fields to move Optional
        // fields to the end; everything else will be kept in its current (correct) position,
        // since the sort is stable.
        let mut fields = fields.to_vec();
        fields.sort_by(|a, b| match (a, b) {
            (MessageField::Field(a), _) => match a.presence {
                Presence::Optional => Ordering::Greater,
                _ => Ordering::Equal,
            },
            (_, MessageField::Field(b)) => match b.presence {
                Presence::Optional => Ordering::Less,
                _ => Ordering::Equal,
            },
            _ => Ordering::Equal,
        });

        for field in &fields {
            if let MessageField::Field(field_type) = field {
                if init && matches!(field_type.presence, Presence::Constant) {
                    continue;
                }
            }

            // Optional fields will have | None annotation. In the init, there will
            // be a comma at the end.
            let field_type = self.resolved_field_type_name(field)?;
            let field_type = match field {
                MessageField::Field(field) => match (field.presence, init) {
                    (Presence::Optional, true) => format!("{} | None = None,", field_type),
                    (Presence::Optional, false) => format!("{} | None", field_type),
                    (_, true) => format!("{},", field_type),
                    _ => field_type,
                },
                _ if init => format!("{},", field_type),
                _ => field_type,
            };

            let field_name = field.name().to_case(Case::Snake);
            fields_tokens.push();
            fields_tokens.append(quote! {
                $(field_name): $(field_type)
            });
        }

        Ok(fields_tokens)
    }

    fn write_message_functions(
        &self,
        message_type: &MessageType,
    ) -> anyhow::Result<impl FormatInto<Python>> {
        Ok(quote! {
            def to_bytes(self, buffer_size: int) -> bytes:
                ...

            def write_to_buffer(self, buffer: bytearray) -> int:
                ...

            @classmethod
            def from_bytes(cls, buffer: bytes) -> $(message_type.name.to_case(Case::UpperCamel)):
                ...
        })
    }

    fn write_composite_fields(
        &self,
        composite_type: &CompositeType,
        init: bool,
    ) -> anyhow::Result<impl FormatInto<Python>> {
        if composite_type.fields.is_empty() {
            return Ok(quote! {
                $(if !init {pass})
            });
        }

        let mut fields_tokens: Tokens<Python> = quote!();

        for simple_type in &composite_type.fields {
            if init && matches!(simple_type.presence(self.types)?, Presence::Constant) {
                continue;
            }

            let field_name = simple_type.name().to_case(Case::Snake);
            let field_type = self.resolved_type_name(simple_type)?;

            fields_tokens.push();
            fields_tokens.append(quote! {
                $(field_name): $(field_type)$(if init {,})
            });
        }

        Ok(fields_tokens)
    }

    fn write_set_fields(&self, set_type: &SetType) -> anyhow::Result<impl FormatInto<Python>> {
        Ok(quote! {
            $(for choice in &set_type.choices {
                $['\r']
                $(choice.name.to_case(Case::Snake)): bool
            })
        })
    }

    fn write_enum_fields(&self, enum_type: &EnumType) -> anyhow::Result<impl FormatInto<Python>> {
        let char_encoding = matches!(
            enum_type
                .encoding_type
                .resolved(&self.types.encoded_types)?,
            NativeType::Char
        );

        Ok(quote! {
            $(for value in &enum_type.values {
                $['\r']
                $(value.name.to_case(Case::UpperSnake)) = $(value.encoded_value(char_encoding)?),
            })
        })
    }

    fn resolved_field_type_name(&self, field: &MessageField) -> anyhow::Result<String> {
        match field {
            MessageField::Field(field_type) => {
                let field_type = self
                    .types
                    .find_type(&field_type.type_name)
                    .unwrap_or(field_type.try_into()?);
                self.resolved_type_name(&field_type)
            }
            MessageField::Group(group_type) => Ok(format!(
                "list[{}]",
                group_type.name.to_case(Case::UpperCamel)
            )),
            MessageField::VariableData(var_data_type) => {
                let repr_type = var_data_type.repr_type(&self.types.composite_types)?;
                let value_type = &repr_type.fields[1];

                Ok(if var_data_type.is_string(&self.types.composite_types)? {
                    "str".to_owned()
                } else if var_data_type.is_bytes(self.types)? {
                    "bytes".to_owned()
                } else {
                    format!("list[{}]", self.resolved_type_name(value_type)?)
                })
            }
        }
    }

    fn resolved_type_name(&self, simple_type: &Type) -> anyhow::Result<String> {
        Ok(match simple_type {
            Type::EncodedData(encoded_type) => {
                let field_type = encoded_type
                    .primitive_type
                    .resolved(&self.types.encoded_types)?;
                let lang_primitive: LanguagePrimitive<Python> =
                    field_type.lang_primitive(&self.types.encoded_types)?;
                let field_length = encoded_type.length.unwrap_or(1);
                match (field_type, field_length) {
                    (_, 0 | 1) => lang_primitive.name.to_owned(),
                    (NativeType::Char, 2..) => "str".to_owned(),
                    (NativeType::UInt8, 2..) => "bytes".to_owned(),
                    (_, _) => format!("list[{}]", lang_primitive.name),
                }
            }
            Type::Reference(ref_type) => self.resolved_type_name(
                &self
                    .types
                    .find_type(&ref_type.type_name)
                    .unwrap_or_else(|| panic!("Referenced type {} not found", ref_type.name)),
            )?,
            _ => simple_type.name().to_owned().to_case(Case::UpperCamel),
        })
    }
}
