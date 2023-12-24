use std::{borrow::Cow, collections::HashMap};

use itertools::Itertools;
use serde_json::json;

use super::{field::ModelField, generator::ModelGenerator, Model};

impl<'a> ModelGenerator<'a> {
    pub(super) fn add_structs_to_rust_context(model: &Model, context: &mut tera::Context) {
        let struct_base = model.struct_name();
        let struct_list = [
            (
                "AllFields",
                Self::struct_contents(model.all_fields().map(|f| f.1), |_| false, true),
            ),
            (
                "UserView",
                Self::struct_contents(model.user_view_struct_fields(), |_| false, true),
            ),
            (
                "OwnerView",
                Self::struct_contents(model.owner_view_struct_fields(), |_| false, true),
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
                        f.owner_access.can_write() && !f.user_access.can_write()
                    },
                    false,
                ),
            ),
        ];

        let mut grouped_fields = HashMap::new();
        for (suffix, fields) in struct_list {
            grouped_fields
                .entry(fields)
                .or_insert_with(Vec::new)
                .push(suffix);
        }

        let structs = grouped_fields
            .into_iter()
            .map(|(fields, suffixes)| {
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

                json!({
                    "name": name,
                    "fields": fields,
                    "aliases": aliases,
                    "is_user_view": suffixes.contains(&"UserView"),
                    "is_owner_view": suffixes.contains(&"OwnerView"),
                })
            })
            .collect::<Vec<_>>();

        let user_view_struct = structs
            .iter()
            .find(|v| v["is_user_view"].as_bool().unwrap_or_default())
            .unwrap();
        let owner_view_struct = structs
            .iter()
            .find(|v| v["is_owner_view"].as_bool().unwrap_or_default())
            .unwrap();
        context.insert("user_view_struct", &user_view_struct["name"]);
        context.insert("owner_view_struct", &owner_view_struct["name"]);

        context.insert("struct_base", &struct_base);
        context.insert("structs", &structs);
    }

    fn struct_contents<'b>(
        fields: impl Iterator<Item = Cow<'b, ModelField>>,
        force_optional: impl Fn(&ModelField) -> bool,
        add_permissions_field: bool,
    ) -> String {
        let content = fields
            .map(|f| {
                let typ = if force_optional(&f) {
                    format!("Option<{}>", f.base_rust_type()).into()
                } else {
                    f.rust_type()
                };

                format!("pub {}: {},", f.rust_field_name(), typ)
            })
            .join("\n");
        if add_permissions_field {
            format!("{content}\npub _permission: ObjectPermission,")
        } else {
            content
        }
    }
}
