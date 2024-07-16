use super::{bindings, QueryBuilder, SqlBuilder, SqlQueryContext};

pub fn insert(data: &SqlBuilder) -> SqlQueryContext {
    let mut q = QueryBuilder::new();
    q.push("INSERT INTO ");
    q.push(&data.context.schema);
    q.push(".");
    q.push(&data.context.table);
    q.push(" (id ");

    if !data.context.global {
        q.push(", organization_id");
    }

    let fields = data
        .context
        .fields
        .iter()
        .filter(|f| f.owner_write)
        .map(|f| f.sql_name.as_str())
        .collect::<Vec<_>>();

    for f in &fields {
        q.push(", ");
        q.push(f);
    }

    q.push(") VALUES (");

    {
        let mut sep = q.separated(", ");
        sep.push_binding(bindings::ID);
        if !data.context.global {
            sep.push_binding(bindings::ORGANIZATION);
        }

        for f in &fields {
            sep.push_binding(f);
        }
    }

    q.push(") RETURNING ");

    let returning = data
        .context
        .fields
        .iter()
        .filter(|f| !f.never_read)
        .map(|f| f.sql_full_name.as_str())
        .collect::<Vec<_>>()
        .join(",\n");
    q.push(&returning);

    q.finish("insert")
}
