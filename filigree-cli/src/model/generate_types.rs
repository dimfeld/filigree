use std::{borrow::Cow, collections::HashMap};

use itertools::Itertools;
use serde_json::json;

use super::{field::ModelField, generator::ModelGenerator};
use crate::Error;

impl<'a> ModelGenerator<'a> {
    pub(super) fn add_rust_structs_to_context(
        &self,
        context: &mut tera::Context,
    ) -> Result<(), Error> {
        let struct_base = self.model.struct_name();
        let user_can_write_anything = self.all_fields()?.any(|f| f.user_access.can_write());
        let struct_list = [
            (
                "AllFields",
                Self::struct_contents(
                    self.all_fields()?.filter(|f| !f.never_read),
                    |_| false,
                    true,
                ),
            ),
            (
                "PopulatedGet",
                Self::struct_contents(
                    self.all_fields()?.filter(|f| !f.never_read).chain(
                        self.virtual_fields(super::generator::ReadOperation::Get)?
                            .map(Cow::Owned),
                    ),
                    |_| false,
                    true,
                ),
            ),
            (
                "PopulatedList",
                Self::struct_contents(
                    self.all_fields()?.filter(|f| !f.never_read).chain(
                        self.virtual_fields(super::generator::ReadOperation::List)?
                            .map(Cow::Owned),
                    ),
                    |_| false,
                    true,
                ),
            ),
            (
                "CreatePayload",
                Self::struct_contents(self.write_payload_struct_fields(false)?, |_| false, false),
            ),
            (
                "UpdatePayload",
                Self::struct_contents(
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

        let owner_and_user_different_access =
            self.all_fields()?.any(|f| f.owner_read() && !f.user_read());
        context.insert(
            "owner_and_user_different_access",
            &owner_and_user_different_access,
        );

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
        Ok(())
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

                let mut serde_attrs = Vec::new();
                if rust_field_name != f.name {
                    serde_attrs.push(Cow::Owned(format!("rename = \"{name}\"", name = f.name)));
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

                format!("{serde_attr}pub {rust_field_name}: {rust_type},")
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
