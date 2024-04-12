use std::{borrow::Cow, collections::BTreeMap};

use convert_case::{Case, Casing};
use itertools::Itertools;
use serde::Deserialize;
use serde_json::json;

use super::generators::{ts_field_type, EndpointPath, ObjectRefOrDef};
use crate::config::generators::rust_permission;

#[derive(Deserialize, Clone, Debug)]
pub struct CustomEndpoint {
    name: String,
    /// The URL for this endpoint. A parameter named `:id` will be given the ID type of the model, and all other
    /// parameters will default to `String` if not otherwise specified in `params`.
    path: EndpointPath,
    /// Customize the types of certain path parameters.
    #[serde(default)]
    params: BTreeMap<String, String>,
    /// The HTTP method for this endpoint.
    method: String,
    /// The name of the payload type for this model. If omitted, an empty structure will be generated for you to fill in.
    #[serde(default)]
    input: ObjectRefOrDef,
    /// The name of the response type for this model. If omitted, an empty structure will be generated for you to fill
    /// in.
    #[serde(default)]
    output: ObjectRefOrDef,
    /// The query parameters that this endpoint accepts.
    #[serde(default)]
    query: ObjectRefOrDef,

    /// What kind of permission is needed to call the endpoint
    /// This can be one of "read", "write", "owner", or "create" to use the corresponding permission for the model,
    /// or any other string.
    permission: String,
}

impl CustomEndpoint {
    fn rust_args(&self, id_type: &str, input_type_name: &str, query_type_name: &str) -> String {
        let mut args = vec![
            "State(state): State<ServerState>".to_string(),
            "auth: Authed".to_string(),
        ];

        if let Some(path) = self.path.parse_to_rust_args(id_type, &self.params) {
            args.push(path);
        }

        if !query_type_name.is_empty() {
            args.push(format!("Query(qs): Query<{}>", query_type_name));
        }

        if self.has_payload() {
            args.push(format!(
                "FormOrJson(payload): FormOrJson<{}>",
                input_type_name
            ));
        }

        args.join(",\n")
    }

    fn has_payload(&self) -> bool {
        match self.method.to_lowercase().as_str() {
            "post" | "patch" | "put" => true,
            _ => false,
        }
    }

    fn ts_args_struct(&self, input_type: &str, query_type: &str) -> String {
        let path_args = self.path.args().map(|s| {
            self.params
                .get(s)
                .map(|s| ts_field_type(s).to_string())
                .unwrap_or_else(|| format!("{s}: string"))
        });
        let payload = self
            .has_payload()
            .then_some(format!("payload: {}", input_type));
        let query = (!query_type.is_empty()).then_some(format!("query: {}", query_type));

        path_args.chain(payload).chain(query).join(",\n")
    }

    fn ts_args(&self) -> String {
        let path_args = self.path.args();
        let payload = self.has_payload().then_some("payload");
        let query = (!self.query.is_empty()).then_some("query");

        path_args.chain(payload).chain(query).join(", ")
    }

    fn type_def(&self, def: &ObjectRefOrDef, suffix: &str) -> (String, String) {
        let prefix = self.name.to_case(Case::Pascal);
        def.type_def(&prefix, suffix)
    }

    fn input_type_def(&self) -> (String, String) {
        self.type_def(&self.input, "Payload")
    }

    fn output_type_def(&self) -> (String, String) {
        self.type_def(&self.output, "Response")
    }

    fn query_type_def(&self) -> (String, String) {
        match &self.query {
            // Empty query indicates no query
            ObjectRefOrDef::Empty => (String::new(), String::new()),
            _ => self.type_def(&self.query, "Query"),
        }
    }

    pub fn template_context(&self, id_type: &str) -> serde_json::Value {
        let input_type_name = self
            .input
            .struct_name()
            .map(Cow::Borrowed)
            .unwrap_or_else(|| Cow::Owned(format!("{}Payload", self.name.to_case(Case::Pascal))));

        let output_type_name = self
            .output
            .struct_name()
            .map(Cow::Borrowed)
            .unwrap_or_else(|| Cow::Owned(format!("{}Response", self.name.to_case(Case::Pascal))));

        let query_type_name = match self.query {
            ObjectRefOrDef::Empty => Cow::Borrowed(""),
            _ => self
                .query
                .struct_name()
                .map(Cow::Borrowed)
                .unwrap_or_else(|| {
                    Cow::Owned(format!("{}Response", self.name.to_case(Case::Pascal)))
                }),
        };

        let input_type_def = self.input_type_def();
        let output_type_def = self.output_type_def();
        let query_type_def = self.query_type_def();

        let rust_path = if self.path.starts_with('/') {
            Cow::Borrowed(&self.path.0)
        } else {
            Cow::Owned(format!("/{}", self.path.0))
        };

        json!({
            "rust": {
                "path": rust_path,
                "args": self.rust_args(id_type, &input_type_name, &query_type_name),
                "input_type_def": input_type_def.0,
                "output_type_def": output_type_def.0,
                "query_type_def": query_type_def.0,
                "permission": rust_permission(&self.permission),
                "method": self.method.to_lowercase(),
            },
            "ts": {
                "path": self.path.ts_path(),
                "method": self.method.to_uppercase(),
                "args": self.ts_args(),
                "args_struct": self.ts_args_struct(&input_type_name, &query_type_name),
                "input_type_def": input_type_def.1,
                "output_type_def": output_type_def.1,
                "query_type_def": query_type_def.1,
            },
            "name": &self.name,
            "snake_name": self.name.to_case(Case::Snake),
            "pascal_name": self.name.to_case(Case::Pascal),
            "upper_name": self.name.to_case(Case::ScreamingSnake),
            "has_payload": self.has_payload(),
            "input_type": input_type_name,
            "output_type": output_type_name,
            "query_type": query_type_name,
        })
    }
}
