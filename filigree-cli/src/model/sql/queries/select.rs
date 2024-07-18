use std::fmt::Write;

use super::{bindings, QueryBuilder, SqlBuilder, SqlQueryContext};

pub fn select_one(data: &SqlBuilder, populate_children: bool) -> Option<SqlQueryContext> {
    if populate_children && data.context.children.is_empty() {
        return None;
    }

    let mut q = QueryBuilder::new();
    let id = q.create_binding(bindings::ID);
    let organization = if data.context.global {
        String::new()
    } else {
        q.create_binding(bindings::ORGANIZATION)
    };

    q.push("SELECT ");

    let mut select_sep = q.separated(", ");

    data.context
        .fields
        .iter()
        .filter(|f| !f.never_read)
        .for_each(|f| select_sep.push(f.sql_full_name.as_str()));

    if populate_children {
        data.context
            .children
            .iter()
            .filter_map(|c| {
                let clause =
                    data.child_population(c, &c.relationship.populate_on_get, &organization, &id);

                clause.map(|clause| (c, clause))
            })
            .for_each(|(c, clause)| {
                select_sep.push(&clause);
                select_sep.push_unseparated(" AS \"");
                select_sep.push_unseparated(&c.full_get_sql_field_name);
                select_sep.push_unseparated("\"");
            });

        let references = data
            .context
            .reference_populations
            .iter()
            .filter(|r| r.on_get);

        for r in references {
            let r_name = format!("ref_{}", r.name);
            let clause = format!(
                r##"CASE WHEN {r_name}.id IS NOT NULL THEN
                    JSONB_BUILD_OBJECT({object})
                ELSE NULL END AS "{full_name}""##,
                full_name = r.full_name,
                object = SqlBuilder::jsonb_build_object_contents(&r.fields, &r_name)
            );
            select_sep.push(&clause);
        }
    }

    // TODO add auth query once project/object-level permissions are implemented

    write!(
        q,
        " FROM {schema}.{table} tb ",
        schema = data.context.schema,
        table = data.context.table
    )
    .unwrap();

    if populate_children {
        let references = data
            .context
            .reference_populations
            .iter()
            .filter(|r| r.on_get);
        for r in references {
            let r_name = format!("ref_{}", r.name);
            write!(
                q,
                "LEFT JOIN {table} {r_name}
                ON tb.{id_field} = {r_name}.id AND tb.organization_id = {r_name}.organization_id",
                table = r.table,
                id_field = r.id_field
            )
            .unwrap();
        }
    }

    write!(q, " WHERE tb.id = {id}").unwrap();
    if !data.context.global {
        write!(q, " AND tb.organization_id = {organization}").unwrap();
    }

    let filename = if populate_children {
        "select_one_populated"
    } else {
        "select_one"
    };
    Some(q.finish(filename))
}
