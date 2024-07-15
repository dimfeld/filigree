use std::{borrow::Cow, collections::HashMap};

use itertools::Itertools;
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
    into_active_model: bool,
}

impl ImplFlags {
    fn or(&self, other: &Self) -> Self {
        Self {
            json_decode: self.json_decode || other.json_decode,
            serialize: self.serialize || other.serialize,
            into_active_model: self.into_active_model || other.into_active_model,
        }
    }
}

impl<'a> ModelGenerator<'a> {
    pub(super) fn add_rust_structs_to_context(
        &self,
        context: &mut tera::Context,
    ) -> Result<(), Error> {
        let struct_base = self.model.struct_name();
        let struct_list = [
            StructContents {
                suffix: "AllFields",
                similar_to: None,
                fields: Self::struct_contents(self.all_fields()?.filter(|f| !f.never_read), |_| {
                    false
                }),
                flags: ImplFlags {
                    serialize: true,
                    json_decode: true,
                    into_active_model: true,
                },
            },
            StructContents {
                suffix: "UserView",
                similar_to: None,
                fields: Self::struct_contents(self.all_fields()?.filter(|f| f.user_read()), |_| {
                    false
                }),
                flags: ImplFlags {
                    serialize: true,
                    json_decode: true,
                    into_active_model: true,
                },
            },
            StructContents {
                suffix: "OwnerView",
                similar_to: None,
                fields: Self::struct_contents(
                    self.all_fields()?.filter(|f| f.owner_read()),
                    |_| false,
                ),
                flags: ImplFlags {
                    serialize: true,
                    json_decode: true,
                    into_active_model: true,
                },
            },
            StructContents {
                suffix: "ListResult",
                similar_to: None,
                fields: Self::struct_contents(
                    self.all_fields()?
                        .filter(|f| !f.never_read && !f.omit_in_list),
                    |_| false,
                ),
                flags: ImplFlags {
                    serialize: true,
                    json_decode: true,
                    into_active_model: false,
                },
            },
            StructContents {
                suffix: "CreateResult",
                similar_to: None,
                fields: Self::struct_contents(
                    self.all_fields()?.filter(|f| !f.never_read).chain(
                        self.write_payload_child_fields(false)?.map(|f| {
                            let mut field = f.field;
                            field.nullable = !f.many;
                            field.rust_type = Some(Self::child_model_field_type(
                                &f.model,
                                super::ReferenceFetchType::Data,
                                f.many,
                                "",
                            ));
                            Cow::Owned(field)
                        }),
                    ),
                    |_| false,
                ),
                flags: ImplFlags {
                    serialize: true,
                    ..Default::default()
                },
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

        let owner_and_user_different_access =
            self.all_fields()?.any(|f| f.owner_read() && !f.user_read());
        context.insert(
            "owner_and_user_different_access",
            &owner_and_user_different_access,
        );

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

                    let is_primary = suffix.is_empty();

                    let name = format!("{struct_base}{suffix}");
                    let rust_name = if is_primary { "Model" } else { name.as_str() };

                    let mut aliases = (suffixes.len() > 1)
                        .then(|| {
                            suffixes
                                .iter()
                                .filter(|(s, _)| *s != "AllFields" && *s != suffix)
                                .map(|(s, _)| format!("{struct_base}{s}"))
                                .collect::<Vec<_>>()
                        })
                        .unwrap_or_default();

                    if is_primary {
                        aliases.push(struct_base.to_owned());
                    }

                    let field_info = fields
                        .into_iter()
                        .map(|field| field.template_context())
                        .collect::<Vec<_>>();

                    json!({
                        "name": name,
                        "rust_name": rust_name,
                        "is_primary_model": is_primary,
                        "rust_fields_content": rust_fields_content,
                        "zod_fields_content": zod_contents,
                        "fields": field_info,
                        "aliases": aliases,
                        "impl_json_decode": flags.json_decode,
                        "impl_serialize": flags.serialize,
                        "impl_into_active_model": flags.into_active_model,
                    })
                },
            )
            .sorted_by(|a, b| a["name"].as_str().unwrap().cmp(b["name"].as_str().unwrap()))
            .collect::<Vec<_>>();

        context.insert("struct_base", &struct_base);
        context.insert("structs", &structs);
        Ok(())
    }

    fn struct_contents<'b>(
        fields: impl Iterator<Item = Cow<'b, ModelField>>,
        force_optional: impl Fn(&ModelField) -> bool,
    ) -> GeneratedStruct {
        let fields = fields
            .map(|f| {
                let mut f = f.into_owned();
                if force_optional(&f) {
                    f.nullable = true;
                }

                f
            })
            .collect::<Vec<_>>();

        let rust_contents = fields
            .iter()
            .map(|f| {
                let rust_field_name = f.rust_field_name();
                let rust_type = f.rust_type();

                let mut sqlx_rename = String::new();
                let mut serde_attrs = Vec::new();
                if rust_field_name != f.name {
                    serde_attrs.push(Cow::Owned(format!("rename = \"{name}\"", name = f.name)));
                    sqlx_rename = format!(
                        "#[sqlx(rename = \"{name}\")]\n#[sea_orm(column_name = \"{name}\")]",
                        name = f.name
                    );
                };

                // Double option is used in some places to distinguish between `null` and missing
                // members coming from JSON.
                if rust_type.starts_with("Option<Option<") {
                    serde_attrs.push("default".into());
                    serde_attrs.push("skip_serializing_if = \"Option::is_none\"".into());
                    serde_attrs.push("with = \"::serde_with::rust::double_option\"".into());
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
            .filter(|f| f.owner_read())
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
