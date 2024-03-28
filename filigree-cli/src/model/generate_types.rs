use std::{borrow::Cow, collections::HashMap};

use itertools::Itertools;
use serde_json::json;

use super::{field::ModelField, generator::ModelGenerator};
use crate::Error;

struct StructContents {
    suffix: &'static str,
    fields: GeneratedStruct,
    flags: ImplFlags,
}

struct GeneratedStruct {
    fields: Vec<ModelField>,
    add_permissions_field: bool,
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

impl<'a> ModelGenerator<'a> {
    pub(super) fn add_rust_structs_to_context(
        &self,
        context: &mut tera::Context,
    ) -> Result<(), Error> {
        let struct_base = self.model.struct_name();
        let user_can_write_anything = self.all_fields()?.any(|f| f.user_access.can_write());
        let struct_list = [
            StructContents {
                suffix: "AllFields",
                fields: Self::struct_contents(
                    self.all_fields()?.filter(|f| !f.never_read),
                    |_| false,
                    true,
                ),
                flags: ImplFlags {
                    serialize: true,
                    json_decode: true,
                },
            },
            StructContents {
                suffix: "ListResult",
                fields: Self::struct_contents(
                    self.all_fields()?
                        .filter(|f| !f.never_read && !f.omit_in_list),
                    |_| false,
                    true,
                ),
                flags: ImplFlags {
                    serialize: true,
                    json_decode: true,
                },
            },
            StructContents {
                suffix: "PopulatedGetResult",
                fields: Self::struct_contents(
                    self.all_fields()?.filter(|f| !f.never_read).chain(
                        self.virtual_fields(super::generator::ReadOperation::Get)?
                            .map(Cow::Owned),
                    ),
                    |_| false,
                    true,
                ),
                flags: ImplFlags {
                    serialize: true,
                    json_decode: true,
                },
            },
            StructContents {
                suffix: "PopulatedListResult",
                fields: Self::struct_contents(
                    self.all_fields()?
                        .filter(|f| !f.never_read && !f.omit_in_list)
                        .chain(
                            self.virtual_fields(super::generator::ReadOperation::List)?
                                .map(Cow::Owned),
                        ),
                    |_| false,
                    true,
                ),
                flags: ImplFlags {
                    serialize: true,
                    json_decode: true,
                },
            },
            StructContents {
                suffix: "CreatePayload",
                fields: Self::struct_contents(
                    self.write_payload_struct_fields(false)?,
                    |_| false,
                    false,
                ),
                flags: ImplFlags::default(),
            },
            StructContents {
                suffix: "CreateResult",
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
                    true,
                ),
                flags: ImplFlags {
                    serialize: true,
                    ..Default::default()
                },
            },
            StructContents {
                suffix: "UpdatePayload",
                fields: Self::struct_contents(
                    self.write_payload_struct_fields(true)?,
                    |f| {
                        // Allow optional fields for those that the owner can write,
                        // but the user can not, so that we can accept either form of
                        // the field.
                        user_can_write_anything
                            && !f.user_access.can_write()
                            && f.owner_access.can_write()
                    },
                    false,
                ),
                flags: ImplFlags::default(),
            },
        ];

        struct GroupedStruct {
            fields: Vec<ModelField>,
            ts_contents: String,
            has_permissions_field: bool,
            flags: ImplFlags,
            suffixes: Vec<&'static str>,
        }

        let mut grouped_fields = HashMap::new();
        for StructContents {
            suffix,
            fields,
            flags,
        } in struct_list
        {
            let entry = grouped_fields
                .entry(fields.rust_contents)
                .or_insert_with(|| GroupedStruct {
                    fields: fields.fields,
                    has_permissions_field: fields.add_permissions_field,
                    flags,
                    ts_contents: fields.zod_contents,
                    suffixes: Vec::new(),
                });
            entry.flags = entry.flags.or(&flags);
            entry.suffixes.push(suffix);
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
                        has_permissions_field,
                        flags,
                        suffixes,
                    },
                )| {
                    let name = if suffixes.contains(&"AllFields") {
                        // The AllFields struct should just have the base name
                        Cow::Borrowed(&struct_base)
                    } else {
                        Cow::Owned(format!(
                            "{struct_base}{suffix}",
                            suffix = suffixes.join("And")
                        ))
                    };

                    let aliases = (suffixes.len() > 1)
                        .then(|| {
                            suffixes
                                .iter()
                                .filter(|suffix| **suffix != "AllFields")
                                .map(|suffix| format!("{struct_base}{suffix}"))
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
                        "has_permission_field": has_permissions_field,
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
        add_permissions_field: bool,
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
                    sqlx_rename = format!("#[sqlx(rename = \"{name}\")]\n", name = f.name);
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

        let (zod_contents, rust_contents) = if add_permissions_field {
            (
                format!("{zod_contents}\n_permission: ObjectPermission,"),
                format!("{rust_contents}\npub _permission: ObjectPermission,"),
            )
        } else {
            (zod_contents, rust_contents)
        };

        GeneratedStruct {
            fields,
            add_permissions_field,
            zod_contents,
            rust_contents,
        }
    }
}
