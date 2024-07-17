use std::fmt::Write;

use super::{bindings, QueryBuilder, SqlBuilder, SqlQueryContext};
use crate::model::field::ModelFieldTemplateContext;

pub fn update_user(data: &SqlBuilder) -> SqlQueryContext {
    let fields = data
        .context
        .fields
        .iter()
        .filter(|f| f.user_write)
        .collect::<Vec<_>>();
    let q = update(data, &fields, None);
    q.finish_with_field_bindings("update_with_user_permissions", &fields)
}

pub fn update_owner(data: &SqlBuilder) -> Option<SqlQueryContext> {
    if !data.context.structs.owner_and_user_different_write_access {
        // This is the same as the user query above so don't generate it twice.
        return None;
    }

    let fields = data
        .context
        .fields
        .iter()
        .filter(|f| f.owner_write)
        .collect::<Vec<_>>();
    let q = update(data, &fields, None);
    Some(q.finish_with_field_bindings("update_with_owner_permissions", &fields))
}

pub fn update_one_with_parent_user(data: &SqlBuilder) -> Option<SqlQueryContext> {
    let Some(belongs_to) = data.context.belongs_to_field.as_ref() else {
        return None;
    };

    let fields = data
        .context
        .fields
        .iter()
        .filter(|f| f.user_write)
        .collect::<Vec<_>>();
    let q = update(data, &fields, Some(&belongs_to.sql_name));

    Some(q.finish_with_field_bindings("update_one_with_parent_user_permissions", &fields))
}

pub fn update_one_with_parent_owner(data: &SqlBuilder) -> Option<SqlQueryContext> {
    if !data.context.structs.owner_and_user_different_write_access {
        // This is the same as the user query above so don't generate it twice.
        return None;
    }

    let Some(belongs_to) = data.context.belongs_to_field.as_ref() else {
        return None;
    };

    let fields = data
        .context
        .fields
        .iter()
        .filter(|f| f.owner_write)
        .collect::<Vec<_>>();
    let q = update(data, &fields, Some(&belongs_to.sql_name));

    Some(q.finish_with_field_bindings("update_one_with_parent_owner_permissions", &fields))
}

fn update<'a>(
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
