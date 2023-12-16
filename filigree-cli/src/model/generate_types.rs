use std::{borrow::Cow, collections::HashMap, path::PathBuf};

use error_stack::Report;
use itertools::Itertools;
use serde_json::json;

use super::{
    field::{Access, ModelField},
    generator::ModelGenerator,
};
use crate::{Error, RenderedFile};

impl<'a> ModelGenerator<'a> {
    pub fn render_types_file(&self) -> Result<RenderedFile, Report<Error>> {
        let struct_base = self.model.struct_name();
        let struct_list = [
            ("AllFields", self.construct_fields(|_| true)),
            (
                "UserView",
                self.construct_fields(|f| f.user_access.can_read()),
            ),
            (
                "OwnerView",
                self.construct_fields(|f| f.owner_access.can_read()),
            ),
            (
                "WritePayload",
                self.construct_fields(|f| f.owner_access.can_write()),
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

        let mut context = tera::Context::new();
        context.insert("struct_base", &struct_base);
        context.insert("structs", &structs);

        self.render_with_context("types.rs.tera", &context)
    }

    fn construct_fields(&self, filter: impl Fn(&ModelField) -> bool) -> String {
        self.all_fields()
            .filter(|(_, f)| filter(f))
            .map(|(_, f)| format!("pub {}: {},", f.rust_field_name(), f.rust_type()))
            .join("\n")
    }
}
