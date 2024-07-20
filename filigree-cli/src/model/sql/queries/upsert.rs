use std::fmt::Write;

use itertools::Itertools;

use super::{bindings, QueryBuilder, SqlBuilder, SqlQueryContext};
use crate::model::{field::ModelFieldTemplateContext, generator::BelongsToFieldContext};

pub fn upsert_queries(data: &SqlBuilder) -> Vec<SqlQueryContext> {
    data.context
        .belongs_to_fields
        .iter()
        .flat_map(|b| [upsert_single_child(data, b), upsert_children(data, b)])
        .collect()
}

fn upsert_single_child(data: &SqlBuilder, belongs_to: &BelongsToFieldContext) -> SqlQueryContext {
    let fields = data
        .context
        .fields
        .iter()
        .filter(|f| {
            if f.writable {
                return true;
            }

            let join_match = data
                .context
                .join
                .as_ref()
                .map(|j| j.model_ids.0 == f.name || j.model_ids.1 == f.name)
                .unwrap_or(false);
            if belongs_to.globally_unique && join_match && belongs_to.name != f.name {
                // If this is a unique join, allow updating the other side when doing
                // an upsert.
                return true;
            }

            false
        })
        .collect::<Vec<_>>();

    let q = upsert(data, &fields, belongs_to, true);
    q.finish_with_field_bindings(
        format!(
            "upsert_single_child_of_{}",
            belongs_to.model_snake_case_name
        ),
        &fields,
    )
}

fn upsert_children(data: &SqlBuilder, belongs_to: &BelongsToFieldContext) -> SqlQueryContext {
    let fields = data
        .context
        .fields
        .iter()
        .filter(|f| f.writable)
        .collect::<Vec<_>>();
    let q = upsert(data, &fields, belongs_to, false);
    q.finish_with_field_bindings(
        format!("upsert_children_of_{}", belongs_to.model_snake_case_name),
        &fields,
    )
}

fn upsert(
    data: &SqlBuilder,
    fields: &[&ModelFieldTemplateContext],
    belongs_to_field: &BelongsToFieldContext,
    single: bool,
) -> QueryBuilder {
    let mut q = QueryBuilder::new();

    // TODO add permissions check when doing project or object level permissions

    write!(
        q,
        "INSERT INTO {schema}.{table} (",
        schema = data.context.schema,
        table = data.context.table
    )
    .unwrap();

    let id_fields = data.id_fields();
    let data_fields = fields
        .iter()
        .filter(|f| !id_fields.iter().any(|id_field| id_field.0 == f.name))
        .collect::<Vec<_>>();
    {
        let mut sep = q.separated(", ");
        for field in &id_fields {
            sep.push(&field.0);
        }

        if !data.context.global {
            sep.push("organization_id");
        }

        for field in &data_fields {
            sep.push(&field.sql_name);
        }
    }

    q.push(") VALUES ");

    if single {
        q.push("(");

        let mut sep = q.separated(", ");
        for (_, binding) in &id_fields {
            sep.push_binding(binding);
        }

        if !data.context.global {
            sep.push_binding(bindings::ORGANIZATION);
        }

        for field in &data_fields {
            sep.push_binding(&field.name);
        }

        q.push(")");
    } else {
        q.push("__insertion_point_insert_values")
    }

    let conflict_field = if belongs_to_field.globally_unique {
        belongs_to_field.sql_name.clone()
    } else {
        id_fields.iter().map(|f| f.0).join(", ")
    };

    if fields.is_empty() {
        write!(q, " ON CONFLICT ({conflict_field}) DO NOTHING").unwrap();
    } else {
        write!(q, " ON CONFLICT ({conflict_field}) DO UPDATE SET ").unwrap();

        for field in fields {
            q.push(&field.sql_name);
            q.push(" = EXCLUDED.");
            q.push(&field.sql_name);
            q.push(",\n");
        }

        q.push("updated_at = now()");

        q.push("\nWHERE ");
        if !data.context.global {
            write!(q, "{table}.organization_id = ", table = data.context.table).unwrap();
            q.push_binding(bindings::ORGANIZATION);
            q.push(" AND ");
        }

        write!(
            q,
            "{table}.{belongs_to} = ",
            table = data.context.table,
            belongs_to = belongs_to_field.sql_name
        )
        .unwrap();
        q.push_binding(bindings::PARENT_ID);
    }

    {
        q.push("\nRETURNING ");
        let mut sep = q.separated(", ");
        for field in &data.context.fields {
            if !field.never_read {
                // Hack for now since upsert_single_child uses the macro
                // but upsert_children does not, and the macro requires type annotations
                // on the RETURNING fields but that breaks when not using the macro.
                if single {
                    sep.push(&field.sql_full_name);
                } else {
                    sep.push(&field.sql_name);
                }
            }
        }
    }

    q
}
