use std::fmt::Write;

use super::{bindings, QueryBuilder, SqlBuilder, SqlQueryContext};
use crate::model::field::ModelFieldTemplateContext;

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

pub fn update_one_with_parent(data: &SqlBuilder) -> Option<SqlQueryContext> {
    let Some(belongs_to) = data.context.belongs_to_field.as_ref() else {
        return None;
    };

    let fields = data
        .context
        .fields
        .iter()
        .filter(|f| f.writable)
        .collect::<Vec<_>>();
    let q = update_query(data, &fields, Some(&belongs_to.sql_name));

    Some(q.finish_with_field_bindings("update_one_with_parent", &fields))
}

fn update_query<'a>(
    data: &'a SqlBuilder,
    fields: &[&ModelFieldTemplateContext],
    parent_field: Option<&str>,
) -> QueryBuilder {
    let mut query = QueryBuilder::new();
    let id = query.create_binding(bindings::ID);
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

    write!(
        query,
        "updated_at = NOW()
        WHERE id = {id}"
    )
    .unwrap();

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
