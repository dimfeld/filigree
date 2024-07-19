use std::fmt::Write;

use super::{bindings, QueryBuilder, SqlBuilder, SqlQueryContext};
use crate::model::generator::BelongsToFieldContext;

pub fn create_delete_query(data: &SqlBuilder) -> SqlQueryContext {
    let mut q = QueryBuilder::new();
    write!(
        q,
        "DELETE FROM {schema}.{table} WHERE \n",
        schema = data.context.schema,
        table = data.context.table
    )
    .unwrap();

    data.push_id_where_clause(&mut q);

    if !data.context.global {
        q.push(" AND organization_id = ");
        q.push_binding(bindings::ORGANIZATION);
    }

    if data.context.auth_check_in_query {
        q.push(" AND ");
        data.permissions_check_where_clause(&mut q, &[&data.context.owner_permission]);
    }

    q.finish("delete")
}

pub fn delete_children_queries(data: &SqlBuilder) -> Vec<SqlQueryContext> {
    data.context
        .belongs_to_fields
        .iter()
        .flat_map(|b| {
            [
                delete_all_children_query(data, b),
                delete_removed_children_query(data, b),
                delete_with_parent_query(data, b),
            ]
        })
        .collect()
}

fn delete_all_children_query(
    data: &SqlBuilder,
    belongs_to_field: &BelongsToFieldContext,
) -> SqlQueryContext {
    let mut q = QueryBuilder::new();
    write!(
        q,
        r##"DELETE FROM {schema}.{table} WHERE "##,
        schema = data.context.schema,
        table = data.context.table
    )
    .unwrap();

    {
        let mut where_sep = q.separated(" AND ");

        if !data.context.global {
            where_sep.push("organization_id = ");
            where_sep.push_binding_unseparated(bindings::ORGANIZATION);
        }

        where_sep.push(&belongs_to_field.sql_name);
        where_sep.push_unseparated(" = ");
        where_sep.push_binding_unseparated(bindings::PARENT_ID);
    }

    q.finish(format!(
        "delete_all_children_of_{}",
        belongs_to_field.model_snake_case_name
    ))
}

fn delete_removed_children_query(
    data: &SqlBuilder,
    belongs_to_field: &BelongsToFieldContext,
) -> SqlQueryContext {
    let mut q = QueryBuilder::new();
    write!(
        q,
        r##"DELETE FROM {schema}.{table} WHERE "##,
        schema = data.context.schema,
        table = data.context.table
    )
    .unwrap();

    {
        let mut where_sep = q.separated(" AND ");

        if !data.context.global {
            where_sep.push("organization_id = ");
            where_sep.push_binding_unseparated(bindings::ORGANIZATION);
        }

        where_sep.push(&belongs_to_field.sql_name);
        where_sep.push_unseparated(" = ");
        where_sep.push_binding_unseparated(bindings::PARENT_ID);

        where_sep.push(data.other_id_field(&belongs_to_field.sql_name));
        where_sep.push_unseparated(" <> ALL (");
        where_sep.push_binding_unseparated(bindings::IDS);
        where_sep.push_unseparated(")");
    }

    q.finish(format!(
        "delete_removed_children_of_{}",
        belongs_to_field.model_snake_case_name
    ))
}

fn delete_with_parent_query(
    data: &SqlBuilder,
    belongs_to_field: &BelongsToFieldContext,
) -> SqlQueryContext {
    let mut q = QueryBuilder::new();
    write!(
        q,
        r##"DELETE FROM {schema}.{table} WHERE "##,
        schema = data.context.schema,
        table = data.context.table
    )
    .unwrap();

    {
        let mut where_sep = q.separated(" AND ");

        if !data.context.global {
            where_sep.push("organization_id = ");
            where_sep.push_binding_unseparated(bindings::ORGANIZATION);
        }

        where_sep.push(&belongs_to_field.sql_name);
        where_sep.push_unseparated(" = ");
        where_sep.push_binding_unseparated(bindings::PARENT_ID);
    }

    q.push(" AND ");
    data.push_id_where_clause(&mut q);

    q.finish(format!(
        "delete_with_parent_{}",
        belongs_to_field.model_snake_case_name
    ))
}
