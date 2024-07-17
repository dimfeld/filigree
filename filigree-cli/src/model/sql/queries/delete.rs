use std::fmt::Write;

use super::{bindings, QueryBuilder, SqlBuilder, SqlQueryContext};

pub fn create_delete_query(data: &SqlBuilder) -> SqlQueryContext {
    let mut q = QueryBuilder::new();
    write!(
        q,
        "DELETE FROM {schema}.{table} WHERE \nid = ",
        schema = data.context.schema,
        table = data.context.table
    )
    .unwrap();
    q.push_binding(bindings::ID);

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

pub fn create_delete_all_children_query(data: &SqlBuilder) -> Option<SqlQueryContext> {
    let Some(belongs_to_field) = data.context.belongs_to_field.as_ref() else {
        return None;
    };

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

    Some(q.finish("delete_all_children"))
}

pub fn delete_removed_children(data: &SqlBuilder) -> Option<SqlQueryContext> {
    let Some(belongs_to_field) = data.context.belongs_to_field.as_ref() else {
        return None;
    };

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

        where_sep.push("id <> ALL (");
        where_sep.push_binding_unseparated(bindings::IDS);
        where_sep.push_unseparated(")");
    }

    Some(q.finish("delete_removed_children"))
}

pub fn delete_with_parent(data: &SqlBuilder) -> Option<SqlQueryContext> {
    let Some(belongs_to_field) = data.context.belongs_to_field.as_ref() else {
        return None;
    };

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

        where_sep.push("id = ");
        where_sep.push_binding_unseparated(bindings::ID);
    }

    Some(q.finish("delete_with_parent"))
}
