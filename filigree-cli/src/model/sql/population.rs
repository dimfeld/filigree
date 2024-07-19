use std::fmt::Write;

use itertools::Itertools;

use super::SqlBuilder;
use crate::{
    model::{
        field::ModelFieldTemplateContext,
        generator::{ChildContext, ReferenceFieldContext},
        ReferenceFetchType,
    },
    templates::sql_string,
};

impl<'a> SqlBuilder<'a> {
    pub fn child_population(
        &self,
        child: &ChildContext,
        fetch_type: &ReferenceFetchType,
        org_binding: &str,
        parent_field_match: &str,
    ) -> Option<String> {
        match fetch_type {
            ReferenceFetchType::Id => {
                let mut output = String::new();
                output.push_str("(SELECT ");
                if child.relationship.many {
                    output.push_str("COALESCE(ARRAY_AGG(");
                }

                let (schema, table, id_field) = if let Some(through) = &child.through {
                    (
                        through.schema.as_str(),
                        through.table.as_str(),
                        through.to_id_field.as_str(),
                    )
                } else {
                    (child.schema.as_str(), child.table.as_str(), "id")
                };
                write!(output, "ct.{id_field}").unwrap();

                if child.relationship.many {
                    output.push_str("), ARRAY[]::uuid[])");
                }

                write!(
                    output,
                    " FROM {schema}.{table} ct WHERE ct.{parent_field} = {parent_field_match}",
                    parent_field = child.parent_field,
                    parent_field_match = parent_field_match
                )
                .unwrap();

                if !self.context.global {
                    output.push_str(" AND organization_id = ");
                    output.push_str(org_binding);
                }
                if !child.relationship.many {
                    output.push_str(" LIMIT 1");
                }

                output.push(')');

                Some(output)
            }
            ReferenceFetchType::Data => {
                let mut output = String::new();
                output.push_str("(SELECT ");
                if child.relationship.many {
                    output.push_str("COALESCE(ARRAY_AGG(");
                }

                let fields = Self::jsonb_build_object_contents(&child.fields, "t");
                write!(output, "JSONB_BUILD_OBJECT({fields})").unwrap();

                if child.relationship.many {
                    output.push_str("), ARRAY[]::jsonb[])");
                }

                if let Some(through) = &child.through {
                    write!(
                        output,
                        r##"FROM {through_schema}.{through_table} tt
                        JOIN {schema}.{table} t ON tt.{to_id_field} = t.id
                        WHERE tt.{from_id_field} = {parent_field_match}"##,
                        schema = child.schema,
                        table = child.table,
                        through_schema = through.schema,
                        through_table = through.table,
                        from_id_field = child.parent_field,
                        to_id_field = through.to_id_field,
                        parent_field_match = parent_field_match,
                    )
                    .unwrap();
                } else {
                    write!(
                        output,
                        " FROM {schema}.{table} t WHERE {parent_field} = {parent_field_match}",
                        schema = child.schema,
                        table = child.table,
                        parent_field = child.parent_field,
                        parent_field_match = parent_field_match
                    )
                    .unwrap();
                }

                if !self.context.global {
                    output.push_str(" AND organization_id = ");
                    output.push_str(org_binding);
                }
                if !child.relationship.many {
                    output.push_str(" LIMIT 1");
                }
                output.push(')');

                Some(output)
            }
            ReferenceFetchType::None => None,
        }
    }

    pub fn jsonb_build_object_contents(
        fields: &[ModelFieldTemplateContext],
        field_table: &str,
    ) -> String {
        fields
            .iter()
            .flat_map(|field| {
                let sql_name = if field_table.is_empty() {
                    field.sql_name.to_string()
                } else {
                    format!("{field_table}.{}", field.sql_name)
                };
                [sql_string(&field.rust_name), sql_name]
            })
            .join(", ")
    }
}
