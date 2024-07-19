use super::{bindings, QueryBuilder, SqlBuilder, SqlQueryContext};

pub fn insert(data: &SqlBuilder) -> SqlQueryContext {
    let mut q = QueryBuilder::new();
    q.push("INSERT INTO ");
    q.push(&data.context.schema);
    q.push(".");
    q.push(&data.context.table);
    q.push(" ( ");

    let id_fields = data.id_fields();

    let data_fields = data
        .context
        .fields
        .iter()
        .filter(|f| f.writable)
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

    q.push(") VALUES (");

    {
        let mut sep = q.separated(", ");

        for (_, binding) in data.id_fields() {
            sep.push_binding(binding);
        }

        if !data.context.global {
            sep.push_binding(bindings::ORGANIZATION);
        }

        for f in &data_fields {
            sep.push_binding(&f.sql_name);
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

    q.finish_with_field_bindings("insert", &data_fields)
}
