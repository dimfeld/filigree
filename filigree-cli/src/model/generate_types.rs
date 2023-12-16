use std::{borrow::Cow, collections::HashMap};

use itertools::Itertools;
use serde_json::json;

use super::{field::ModelField, generator::ModelGenerator, Model};

impl<'a> ModelGenerator<'a> {
    pub(super) fn add_structs_to_rust_context(model: &Model, context: &mut tera::Context) {
        let struct_base = model.struct_name();
        let struct_list = [
            ("AllFields", Self::construct_fields(model, |_| true)),
            (
                "UserView",
                Self::construct_fields(model, |f| f.user_access.can_read()),
            ),
            (
                "OwnerView",
                Self::construct_fields(model, |f| f.owner_access.can_read()),
            ),
            (
                "WritePayload",
                Self::construct_fields(model, |f| f.owner_access.can_write()),
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
                })
            })
            .collect::<Vec<_>>();

        context.insert("struct_base", &struct_base);
        context.insert("structs", &structs);
    }

    fn construct_fields(model: &Model, filter: impl Fn(&ModelField) -> bool) -> String {
        model
            .all_fields()
            .filter(|(_, f)| filter(f))
            .map(|(_, f)| format!("pub {}: {},", f.rust_field_name(), f.rust_type()))
            .join("\n")
    }
}
