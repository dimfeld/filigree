use std::{borrow::Cow, collections::HashMap};

use itertools::Itertools;
use serde_json::json;

use super::{field::ModelField, generator::ModelGenerator, Model};

impl<'a> ModelGenerator<'a> {
    pub(super) fn add_structs_to_rust_context(model: &Model, context: &mut tera::Context) {
        let struct_base = model.struct_name();
        let user_can_write_anything = model.all_fields().any(|f| f.1.user_access.can_write());
        let struct_list = [
            (
                "AllFields",
                Self::struct_contents(model.all_fields().map(|f| f.1), |_| false, true),
            ),
            (
                "CreatePayload",
                Self::struct_contents(model.write_payload_struct_fields(), |_| false, false),
            ),
            (
                "UpdatePayload",
                Self::struct_contents(
                    model.write_payload_struct_fields(),
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
            ),
        ];

        let mut grouped_fields = HashMap::new();
        for (suffix, fields) in struct_list {
            grouped_fields
                .entry(fields.2)
                .or_insert_with(|| (fields.0, fields.1, Vec::new()))
                .2
                .push(suffix);
        }

        let structs = grouped_fields
            .into_iter()
            .map(
                |(fields_content, (fields, has_permission_field, suffixes))| {
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
                                .map(|suffix| format!("{struct_base}{suffix}"))
                                .collect::<Vec<_>>()
                        })
                        .unwrap_or_default();

                    let field_info = fields
                        .into_iter()
                        .map(|field| {
                            json!({
                                "name": field.name,
                                "rust_name": field.rust_field_name(),
                                "base_rust_type": field.base_rust_type(),
                                "rust_type": field.rust_type(),
                                "is_custom_rust_type": field.rust_type.is_some(),
                                "default_rust": field.default_rust,
                                "nullable": field.nullable,
                            })
                        })
                        .collect::<Vec<_>>();

                    json!({
                        "name": name,
                        "fields_content": fields_content,
                        "fields": field_info,
                        "aliases": aliases,
                        "has_permission_field": has_permission_field,
                    })
                },
            )
            .sorted_by(|a, b| a["name"].as_str().unwrap().cmp(b["name"].as_str().unwrap()))
            .collect::<Vec<_>>();

        context.insert("struct_base", &struct_base);
        context.insert("structs", &structs);
    }

    fn struct_contents<'b>(
        fields: impl Iterator<Item = Cow<'b, ModelField>>,
        force_optional: impl Fn(&ModelField) -> bool,
        add_permissions_field: bool,
    ) -> (Vec<ModelField>, bool, String) {
        let fields = fields
            .map(|f| {
                let mut f = f.into_owned();
                if force_optional(&f) {
                    f.nullable = true;
                }

                f
            })
            .collect::<Vec<_>>();

        let content = fields
            .iter()
            .map(|f| {
                let rust_field_name = f.rust_field_name();
                let rust_type = f.rust_type();
                let serde_rename = if rust_field_name != f.name {
                    format!("#[serde(rename = \"{name}\")]\n", name = f.name)
                } else {
                    String::new()
                };
                format!("{serde_rename}pub {rust_field_name}: {rust_type},")
            })
            .join("\n");

        let content = if add_permissions_field {
            format!("{content}\npub _permission: ObjectPermission,")
        } else {
            content
        };

        (fields, add_permissions_field, content)
    }
}
