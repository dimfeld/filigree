use std::fmt::Write;

use super::{bindings, QueryBuilder, SqlBuilder, SqlQueryContext};
use crate::model::field::ModelFieldTemplateContext;

pub fn upsert_single_child(data: &SqlBuilder) -> Option<SqlQueryContext> {
    let Some(belongs_to) = data.context.belongs_to_field.as_ref() else {
        return None;
    };

    let fields = data
        .context
        .fields
        .iter()
        .filter(|f| f.writable)
        .collect::<Vec<_>>();
    let q = upsert(data, &fields, belongs_to, true);
    Some(q.finish_with_field_bindings("upsert_single_child", &fields))
}

pub fn upsert_children(data: &SqlBuilder) -> Option<SqlQueryContext> {
    let Some(belongs_to) = data.context.belongs_to_field.as_ref() else {
        return None;
    };

    let fields = data
        .context
        .fields
        .iter()
        .filter(|f| f.writable)
        .collect::<Vec<_>>();
    let q = upsert(data, &fields, belongs_to, false);
    Some(q.finish_with_field_bindings("upsert_children", &fields))
}

fn upsert(
    data: &SqlBuilder,
    fields: &[&ModelFieldTemplateContext],
    belongs_to_field: &ModelFieldTemplateContext,
    single: bool,
) -> QueryBuilder {
    let mut q = QueryBuilder::new();

    // TODO add permissions check when doing project or object level permissions

    write!(
        q,
        "INSERT INTO {schema}.{table} (id ",
        schema = data.context.schema,
        table = data.context.table
    )
    .unwrap();
    if !data.context.global {
        q.push(", organization_id");
    }

    for field in fields {
        q.push(", ");
        q.push(&field.sql_name);
    }

    q.push(") VALUES ");

    if single {
        q.push("(");

        let mut sep = q.separated(", ");
        sep.push_binding(bindings::ID);

        if !data.context.global {
            sep.push_binding(bindings::ORGANIZATION);
        }

        for field in fields {
            sep.push_binding(&field.name);
        }

        q.push(")");
    } else {
        q.push("__insertion_point_insert_values")
    }

    let conflict_field = if belongs_to_field.globally_unique {
        &belongs_to_field.sql_name
    } else {
        "id"
    };

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
