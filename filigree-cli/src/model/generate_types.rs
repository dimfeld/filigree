use std::{borrow::Cow, collections::HashMap};

use itertools::Itertools;
use serde::Serialize;
use serde_json::json;

use super::{field::ModelField, generator::ModelGenerator};
use crate::Error;

struct StructContents {
    suffix: &'static str,
    similar_to: Option<&'static str>,
    fields: GeneratedStruct,
    flags: ImplFlags,
}

struct GeneratedStruct {
    fields: Vec<ModelField>,
    rust_contents: String,
    zod_contents: String,
}

#[derive(Default, Copy, Clone)]
struct ImplFlags {
    json_decode: bool,
    serialize: bool,
}

impl ImplFlags {
    fn or(&self, other: &Self) -> Self {
        Self {
            json_decode: self.json_decode || other.json_decode,
            serialize: self.serialize || other.serialize,
        }
    }
}

#[derive(Serialize, Clone, Debug)]
pub struct StructsContext {
    pub struct_base: String,
    pub structs: Vec<serde_json::Value>,
}

impl<'a> ModelGenerator<'a> {
    pub(super) fn create_structs_context(&self) -> Result<StructsContext, Error> {
        let struct_base = self.model.struct_name();
        let struct_list = [
            StructContents {
                suffix: "AllFields",
                similar_to: None,
                fields: Self::struct_contents(self.all_fields()?.filter(|f| !f.never_read)),
                flags: ImplFlags {
                    serialize: true,
                    json_decode: true,
                },
            },
            StructContents {
                suffix: "ListResult",
                similar_to: None,
                fields: Self::struct_contents(
                    self.all_fields()?
                        .filter(|f| !f.never_read && !f.omit_in_list),
                ),
                flags: ImplFlags {
                    serialize: true,
                    json_decode: true,
                },
            },
            StructContents {
                suffix: "PopulatedGetResult",
                similar_to: Some("AllFields"),
                fields: Self::struct_contents(
                    self.all_fields()?.filter(|f| !f.never_read).chain(
                        self.virtual_fields(super::generator::ReadOperation::Get)?
                            .map(Cow::Owned),
                    ),
                ),
                flags: ImplFlags {
                    serialize: true,
                    json_decode: true,
                },
            },
            StructContents {
                suffix: "PopulatedListResult",
                similar_to: Some("ListResult"),
                fields: Self::struct_contents(
                    self.all_fields()?
                        .filter(|f| !f.never_read && !f.omit_in_list)
                        .chain(
                            self.virtual_fields(super::generator::ReadOperation::List)?
                                .map(Cow::Owned),
                        ),
                ),
                flags: ImplFlags {
                    serialize: true,
                    json_decode: true,
                },
            },
            StructContents {
                suffix: "CreatePayload",
                similar_to: None,
                fields: Self::struct_contents(self.write_payload_struct_fields(false)?),
                flags: ImplFlags::default(),
            },
            StructContents {
                suffix: "CreateResult",
                similar_to: None,
                fields: Self::struct_contents(self.all_fields()?.filter(|f| !f.never_read).chain(
                    self.write_payload_child_fields(false)?.map(|f| {
                        let mut field = f.field;
                        field.nullable = !f.many;
                        let fetch_type = if f.through.is_some() {
                            super::ReferenceFetchType::Id
                        } else {
                            super::ReferenceFetchType::Data
                        };
                        field.rust_type = Some(Self::child_model_field_type(
                            &f.model, fetch_type, f.many, "",
                        ));
                        Cow::Owned(field)
                    }),
                )),
                flags: ImplFlags {
                    serialize: true,
                    ..Default::default()
                },
            },
            StructContents {
                suffix: "UpdatePayload",
                similar_to: None,
                fields: Self::struct_contents(self.write_payload_struct_fields(true)?),
                flags: ImplFlags::default(),
            },
        ];

        struct GroupedStruct {
            fields: Vec<ModelField>,
            ts_contents: String,
            flags: ImplFlags,
            suffixes: Vec<(&'static str, Option<&'static str>)>,
        }

        let mut grouped_fields = HashMap::new();
        for StructContents {
            suffix,
            similar_to,
            fields,
            flags,
        } in struct_list
        {
            let entry = grouped_fields
                .entry(fields.rust_contents)
                .or_insert_with(|| GroupedStruct {
                    fields: fields.fields,
                    flags,
                    ts_contents: fields.zod_contents,
                    suffixes: Vec::new(),
                });
            entry.flags = entry.flags.or(&flags);
            entry.suffixes.push((suffix, similar_to));
        }

        let structs = grouped_fields
            .into_iter()
            .map(
                |(
                    rust_fields_content,
                    GroupedStruct {
                        fields,
                        ts_contents: zod_contents,
                        flags,
                        suffixes,
                    },
                )| {
                    let suffix = if suffixes.iter().find(|(s, _)| *s == "AllFields").is_some() {
                        // The AllFields struct should just have the base name
                        Cow::Borrowed("")
                    } else {
                        let suffix = suffixes
                            .iter()
                            .filter(|(_, similar_to)| {
                                let Some(similar_to) = similar_to else {
                                    return true;
                                };

                                suffixes.iter().find(|(s, _)| s == similar_to).is_none()
                            })
                            .map(|(s, _)| s)
                            .join("And");

                        Cow::Owned(suffix)
                    };

                    let name = format!("{struct_base}{suffix}");

                    let aliases = (suffixes.len() > 1)
                        .then(|| {
                            suffixes
                                .iter()
                                .filter(|(s, _)| *s != "AllFields" && *s != suffix)
                                .map(|(s, _)| format!("{struct_base}{s}"))
                                .collect::<Vec<_>>()
                        })
                        .unwrap_or_default();

                    let field_info = fields
                        .into_iter()
                        .map(|field| field.template_context())
                        .collect::<Vec<_>>();

                    json!({
                        "name": name,
                        "rust_fields_content": rust_fields_content,
                        "zod_fields_content": zod_contents,
                        "fields": field_info,
                        "aliases": aliases,
                        "impl_json_decode": flags.json_decode,
                        "impl_serialize": flags.serialize,
                    })
                },
            )
            .sorted_by(|a, b| a["name"].as_str().unwrap().cmp(b["name"].as_str().unwrap()))
            .collect::<Vec<_>>();

        Ok(StructsContext {
            struct_base,
            structs,
        })
    }

    fn struct_contents<'b>(fields: impl Iterator<Item = Cow<'b, ModelField>>) -> GeneratedStruct {
        let fields = fields.map(|f| f.into_owned()).collect::<Vec<_>>();
        let rust_contents = fields
            .iter()
            .map(|f| {
                let rust_field_name = f.rust_field_name();
                let rust_type = f.rust_type();

                let mut sqlx_rename = String::new();
                let mut serde_attrs = Vec::new();
                if rust_field_name != f.name {
                    serde_attrs.push(Cow::Owned(format!("rename = \"{name}\"", name = f.name)));
                    sqlx_rename = format!("#[sqlx(rename = \"{name}\")]\n", name = f.name);
                };

                // Double option is used in some places to distinguish between `null` and missing
                // members coming from JSON.
                if rust_type.starts_with("Option<Option<") {
                    serde_attrs.push("default".into());
                    serde_attrs.push("skip_serializing_if = \"Option::is_none\"".into());
                    serde_attrs.push(
                        "deserialize_with = \"::serde_with::rust::double_option::deserialize\""
                            .into(),
                    );
                }

                let serde_attr = if serde_attrs.is_empty() {
                    String::new()
                } else {
                    format!("#[serde({})]\n", serde_attrs.join(", "))
                };

                format!("{serde_attr}{sqlx_rename}pub {rust_field_name}: {rust_type},")
            })
            .join("\n");

        let zod_contents = fields
            .iter()
            .filter(|f| f.readable())
            .map(|f| {
                format!(
                    "  {ts_field_name}: {ts_type},",
                    ts_field_name = f.name,
                    ts_type = f.zod_type()
                )
            })
            .join("\n");

        GeneratedStruct {
            fields,
            zod_contents,
            rust_contents,
        }
    }
}
