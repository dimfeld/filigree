pub mod ids;
pub(self) mod permissions;
pub mod population;
mod queries;
mod query_builder;

/// Common binding names used in a lot of places
pub mod bindings {
    pub const ACTOR_IDS: &str = "actor_ids";
    pub const ID: &str = "id";
    pub const JOIN_ID_0: &str = "join_id_0";
    pub const JOIN_ID_1: &str = "join_id_1";
    pub const IDS: &str = "ids";
    pub const PARENT_ID: &str = "parent_id";
    pub const ORGANIZATION: &str = "organization_id";
    pub const LIMIT: &str = "limit";
    pub const OFFSET: &str = "offset";
}

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// Context that helps use a SQL query
#[derive(Clone)]
pub struct SqlQueryContext {
    /// Names for each parameter to help with query binding generation
    pub bindings: Vec<String>,
    /// Clauses for each field that can be bound to a query.
    /// Replace the `$payload` string with the name of your actual payload variable.
    pub field_params: HashMap<String, String>,
    pub query: String,
    pub name: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SqlQueryTemplateContext {
    pub name: String,
    pub bindings: Vec<String>,
    pub num_bindings: usize,
    pub field_params: HashMap<String, String>,
}

impl From<SqlQueryContext> for SqlQueryTemplateContext {
    fn from(value: SqlQueryContext) -> Self {
        Self {
            name: value.name,
            num_bindings: value.bindings.len(),
            bindings: value.bindings,
            field_params: value.field_params,
        }
    }
}

/// Generate a list of strings that can be used as bindings for a SQL query
pub fn generate_query_bindings(args: &HashMap<String, tera::Value>) -> tera::Result<tera::Value> {
    let query = args
        .get("query")
        .map(|b| tera::from_value::<SqlQueryTemplateContext>(b.clone()))
        .transpose()?
        .ok_or_else(|| tera::Error::msg("Missing query argument"))?;

    let payload_var = args
        .get("_payload_var")
        .map(|b| b.to_string())
        .unwrap_or("payload".to_string());

    let call_bind = args
        .get("_call_bind")
        .map(|b| tera::from_value::<bool>(b.clone()))
        .transpose()?
        .unwrap_or(false);

    let output = query
        .bindings
        .iter()
        .map(|name| {
            let arg = args
                .get(name)
                .map(|b| tera::from_value::<String>(b.clone()))
                .transpose()?
                .or_else(|| query.field_params.get(name).map(|s| s.to_string()))
                .or_else(|| {
                    // Fixed cases for certain IDs
                    if name == bindings::JOIN_ID_0 {
                        Some("id.0.as_uuid()".to_string())
                    } else if name == bindings::JOIN_ID_1 {
                        Some("id.1.as_uuid()".to_string())
                    } else {
                        None
                    }
                })
                .ok_or_else(|| {
                    tera::Error::msg(format!("Missing {name} argument in query {}", query.name))
                })?;

            let output = arg.replace("$payload", &payload_var);

            let output = if call_bind {
                format!(".bind({output})")
            } else {
                output
            };

            Ok::<_, tera::Error>(output)
        })
        .collect::<tera::Result<Vec<String>>>()?;

    let joined = if call_bind {
        output.join("\n")
    } else {
        output.join(",\n")
    };

    Ok(tera::to_value(joined)?)
}

/// Build the various SQL queries that are needed by the model
pub struct SqlBuilder<'a> {
    pub context: &'a super::generator::TemplateContext,
}

impl<'a> SqlBuilder<'a> {
    pub fn create_model_queries(&self) -> Vec<SqlQueryContext> {
        [
            queries::delete::create_delete_query(self),
            queries::insert::insert(self),
            queries::update::update(self),
        ]
        .into_iter()
        .chain(
            // Optional queries
            [
                queries::list::list(self, false),
                queries::list::list(self, true),
                queries::select::select_one(self, false),
                queries::select::select_one(self, true),
            ]
            .into_iter()
            .flatten(),
        )
        .chain(
            // Vec-returning queries
            [
                queries::update::update_one_with_parent(self),
                queries::upsert::upsert_queries(self),
                queries::delete::delete_children_queries(self),
            ]
            .into_iter()
            .flatten(),
        )
        .collect()
    }
}
