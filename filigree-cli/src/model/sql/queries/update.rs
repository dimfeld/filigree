use std::fmt::Write;

use super::{bindings, QueryBuilder, SqlBuilder, SqlQueryContext};
use crate::model::{field::ModelFieldTemplateContext, generator::BelongsToFieldContext};

pub fn update(data: &SqlBuilder) -> SqlQueryContext {
    let fields = data
        .context
        .fields
        .iter()
        .filter(|f| f.writable)
        .collect::<Vec<_>>();
    let q = update_query(data, &fields, None);
    q.finish_with_field_bindings("update", &fields)
}

pub fn update_one_with_parent(data: &SqlBuilder) -> Vec<SqlQueryContext> {
    if data.context.join.is_some() {
        // For joining models, the parent is in the ID itself.
        return Vec::new();
    }

    data.context
        .belongs_to_fields
        .iter()
        .map(|b| update_one_with_parent_query(data, b))
        .collect()
}

pub fn update_one_with_parent_query(
    data: &SqlBuilder,
    belongs_to: &BelongsToFieldContext,
) -> SqlQueryContext {
    let fields = data
        .context
        .fields
        .iter()
        .filter(|f| f.writable)
        .collect::<Vec<_>>();
    let q = update_query(data, &fields, Some(&belongs_to.sql_name));

    q.finish_with_field_bindings(
        format!(
            "update_one_with_parent_{}",
            belongs_to.model_snake_case_name
        ),
        &fields,
    )
}

fn update_query<'a>(
    data: &'a SqlBuilder,
    fields: &[&ModelFieldTemplateContext],
    parent_field: Option<&str>,
) -> QueryBuilder {
    let mut query = QueryBuilder::new();
    write!(
        query,
        "UPDATE {schema}.{table} SET ",
        schema = data.context.schema,
        table = data.context.table
    )
    .unwrap();

    for field in fields {
        query.push(&field.sql_name);
        query.push(" = ");
        query.push_binding(&field.name);
        query.push(",\n");
    }

    query.push(
        "updated_at = NOW()
        WHERE ",
    );

    data.push_id_where_clause(&mut query);

    if let Some(parent_field) = parent_field {
        query.push(" AND ");
        query.push(parent_field);
        query.push(" = ");
        query.push_binding(bindings::PARENT_ID);
    }

    if !data.context.global {
        query.push(" AND organization_id = ");
        query.push_binding(bindings::ORGANIZATION);
    }

    query
}
