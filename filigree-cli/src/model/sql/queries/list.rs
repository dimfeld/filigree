use std::fmt::Write;

use super::{bindings, QueryBuilder, SqlBuilder, SqlQueryContext};
use crate::model::field::FilterableType;

pub fn list(data: &SqlBuilder, populate_children: bool) -> Option<SqlQueryContext> {
    if populate_children && data.context.children.is_empty() {
        return None;
    }

    let mut q = QueryBuilder::new();

    q.push("SELECT ");

    let org_binding = q.create_binding(bindings::ORGANIZATION);
    let mut select_sep = q.separated(", ");

    data.context
        .fields
        .iter()
        .filter(|f| !f.never_read && !f.omit_in_list)
        .for_each(|f| {
            select_sep.push(&f.sql_name);
        });

    if populate_children {
        data.context
            .children
            .iter()
            .filter_map(|c| {
                let clause = data.child_population(
                    c,
                    &c.relationship.populate_on_list,
                    &org_binding,
                    "tb.id",
                );
                clause.map(|clause| (c, clause))
            })
            .for_each(|(c, clause)| {
                select_sep.push(&clause);
                select_sep.push_unseparated(" AS \"");
                select_sep.push_unseparated(&c.list_sql_field_name);
                select_sep.push_unseparated("\"");
            });

        let references = data
            .context
            .reference_populations
            .iter()
            .filter(|r| r.on_list);
        for r in references {
            let name = format!("ref_{}", r.name);
            let object = SqlBuilder::jsonb_build_object_contents(&r.fields, &name);
            let clause = format!(
                r##"(SELECT JSONB_BUILD_OBJECT({object})
                FROM {table} {name}
                WHERE tb.{r_id_field} IS NOT NULL
                    AND {name}.id = tb.{r_id_field}
                    AND {name}.organization_id = tb.organization_id
                ) AS "{full_name}"
                "##,
                table = r.table,
                r_id_field = r.id_field,
                full_name = r.full_name
            );

            select_sep.push(&clause);
        }
    }

    write!(
        q,
        " FROM {schema}.{table} tb",
        schema = data.context.schema,
        table = data.context.table
    )
    .unwrap();

    {
        let mut where_sep = q.separated(" AND ");
        where_sep.on_first(" WHERE ");

        if !data.context.global {
            where_sep.push("organization_id = ");
            where_sep.push_binding_unseparated(bindings::ORGANIZATION);
        }

        let has_filterable = data
            .context
            .fields
            .iter()
            .any(|f| !matches!(f.filterable, FilterableType::None));
        if has_filterable {
            where_sep.push("__insertion_point_filters");
        }
    }

    // TODO add auth query once project/object-level permissions are implemented

    if !data.context.pagination.disable {
        q.push(" LIMIT ");
        q.push_binding(bindings::LIMIT);
        q.push(" OFFSET ");
        q.push_binding(bindings::OFFSET);
    }

    let filename = if populate_children {
        "list_populated"
    } else {
        "list"
    };
    Some(q.finish(filename))
}
