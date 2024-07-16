pub(self) mod permissions;
pub mod population;
mod queries;
mod query_builder;

/// Common binding names used in a lot of places
pub mod bindings {
    pub const ACTOR_IDS: &str = "actor_ids";
    pub const ID: &str = "id";
    pub const IDS: &str = "ids";
    pub const PARENT_ID: &str = "parent_id";
    pub const ORGANIZATION: &str = "organization_id";
    pub const LIMIT: &str = "limit";
    pub const OFFSET: &str = "offset";
}

use std::collections::HashMap;

use serde::Serialize;

/// Context that helps use a SQL query
#[derive(Serialize)]
pub struct SqlQueryContext {
    /// Names for each parameter to help with query binding generation
    pub bindings: Vec<String>,
    pub query: String,
    pub name: String,
}

/// Generate a list of strings that can be used as bindings for a SQL query
pub fn generate_query_bindings(args: &HashMap<String, tera::Value>) -> tera::Result<tera::Value> {
    let bindings = args
        .get("bindings")
        .map(|b| tera::from_value::<Vec<String>>(b.clone()))
        .transpose()?
        .ok_or_else(|| tera::Error::msg("Missing bindings argument"))?;

    let output = bindings
        .iter()
        .map(|name| {
            args.get(name)
                .map(|b| tera::from_value::<String>(b.clone()))
                .transpose()?
                .ok_or_else(|| tera::Error::msg(format!("Missing {name} argument")))
        })
        .collect::<tera::Result<Vec<String>>>()?;

    Ok(tera::to_value(output)?)
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
            queries::update::update_user(self),
        ]
        .into_iter()
        .chain(
            // Optional queries
            [
                queries::list::list(self, false),
                queries::list::list(self, true),
                queries::select::select_one(self, false),
                queries::select::select_one(self, true),
                queries::delete::create_delete_all_children_query(self),
                queries::delete::delete_removed_children(self),
                queries::delete::delete_with_parent(self),
                queries::update::update_owner(self),
                queries::update::update_one_with_parent_owner(self),
                queries::update::update_one_with_parent_user(self),
                queries::upsert::upsert_children_user(self),
                queries::upsert::upsert_children_owner(self),
                queries::upsert::upsert_single_child_user(self),
                queries::upsert::upsert_single_child_owner(self),
            ]
            .into_iter()
            .flatten(),
        )
        .collect()
    }
}
