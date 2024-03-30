use std::{borrow::Cow, collections::BTreeMap};

use convert_case::{Case, Casing};
use itertools::Itertools;
use serde::Deserialize;
use serde_json::json;

#[derive(Deserialize, Clone, Debug)]
pub struct CustomEndpoint {
    name: String,
    /// The URL for this endpoint. A parameter named `:id` will be given the ID type of the model, and all other
    /// parameters will default to `String` if not otherwise specified in `params`.
    path: String,
    /// Customize the types of certain path parameters.
    #[serde(default)]
    params: BTreeMap<String, String>,
    /// The HTTP method for this endpoint.
    method: String,
    /// The name of the payload type for this model. If omitted, an empty structure will be generated for you to fill in.
    #[serde(default)]
    input: EmptyOrStringOrMap,
    /// The name of the response type for this model. If omitted, an empty structure will be generated for you to fill
    /// in.
    #[serde(default)]
    output: EmptyOrStringOrMap,
    /// The query parameters that this endpoint accepts.
    #[serde(default)]
    query: EmptyOrStringOrMap,

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

        if let Some(path) = self.parse_path_to_rust_arg(id_type) {
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

    fn path_segments(&self) -> impl Iterator<Item = &str> {
        self.path.split('/').filter(|s| !s.is_empty())
    }

    fn path_args(&self) -> impl Iterator<Item = &str> {
        self.path_segments()
            .filter(|s| s.starts_with(':'))
            .map(|s| &s[1..])
    }

    fn parse_path_to_rust_arg(&self, id_type: &str) -> Option<String> {
        let segments = self.path_args().collect_vec();

        if segments.is_empty() {
            return None;
        }

        let segment_types = segments
            .iter()
            .map(|&s| {
                self.params
                    .get(s)
                    .map(|t| rust_field_type(t))
                    .unwrap_or_else(|| if s == "id" { id_type } else { "String" })
            })
            .collect::<Vec<_>>();

        if segments.len() == 1 {
            Some(format!("Path({}): Path<{}>", segments[0], segment_types[0],))
        } else {
            Some(format!(
                "Path(({})): Path<({})>",
                segments.join(", "),
                segment_types.join(", ")
            ))
        }
    }

    fn ts_path(&self) -> String {
        let segments = self.path_segments().map(|s| {
            if s.starts_with(':') {
                Cow::Owned(format!("${{{s}}}", s = &s[1..]))
            } else {
                Cow::Borrowed(s)
            }
        });

        std::iter::once(Cow::Borrowed("/api"))
            .chain(segments)
            .join("/")
    }

    fn has_payload(&self) -> bool {
        match self.method.to_lowercase().as_str() {
            "post" | "patch" | "put" => true,
            _ => false,
        }
    }

    fn ts_args_struct(&self, input_type: &str, query_type: &str) -> String {
        let path_args = self.path_args().map(|s| {
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
        let path_args = self.path_args();
        let payload = self.has_payload().then_some("payload");
        let query = (!self.query.is_empty()).then_some("query");

        path_args.chain(payload).chain(query).join(", ")
    }

    fn define_rust_type(&self, suffix: &str, contents: &str) -> String {
        let prefix = self.name.to_case(Case::Pascal);
        format!("#[derive(serde::Deserialize, serde::Serialize, Debug, JsonSchema)]\npub struct {prefix}{suffix} {{\n{contents}\n}}")
    }

    fn define_ts_type(&self, suffix: &str, contents: &str) -> String {
        let prefix = self.name.to_case(Case::Pascal);
        format!("export interface {prefix}{suffix} {{\n{contents}\n}}")
    }

    fn type_def(&self, def: &EmptyOrStringOrMap, suffix: &str) -> (String, String) {
        match def {
            EmptyOrStringOrMap::Empty => (
                self.define_rust_type(suffix, ""),
                self.define_ts_type(suffix, ""),
            ),
            // The structure is defined elsewhere, so we don't define it
            EmptyOrStringOrMap::String(_) => (String::new(), String::new()),
            EmptyOrStringOrMap::Map(m) => {
                let rust_contents = m
                    .iter()
                    .map(|(k, v)| format!("pub {k}: {v},\n", v = rust_field_type(v)))
                    .join("");

                let ts_contents = m
                    .iter()
                    .map(|(k, v)| format!("{k}: {v},\n", v = ts_field_type(v)))
                    .join("");

                (
                    self.define_rust_type(suffix, &rust_contents),
                    self.define_ts_type(suffix, &ts_contents),
                )
            }
        }
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
            EmptyOrStringOrMap::Empty => (String::new(), String::new()),
            _ => self.type_def(&self.query, "Query"),
        }
    }

    fn rust_permission(&self) -> Cow<'static, str> {
        match self.permission.as_str() {
            "owner" => "OWNER_PERMISSION".into(),
            "write" => "WRITE_PERMISSION, OWNER_PERMISSION".into(),
            "read" => "READ_PERMISSION, WRITE_PERMISSION, OWNER_PERMISSION".into(),
            "create" => "CREATE_PERMISSION".into(),
            t => format!("\"{}\"", t).into(),
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
            EmptyOrStringOrMap::Empty => Cow::Borrowed(""),
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
            Cow::Borrowed(&self.path)
        } else {
            Cow::Owned(format!("/{}", self.path))
        };

        json!({
            "rust": {
                "path": rust_path,
                "args": self.rust_args(id_type, &input_type_name, &query_type_name),
                "input_type_def": input_type_def.0,
                "output_type_def": output_type_def.0,
                "query_type_def": query_type_def.0,
                "permission": self.rust_permission(),
                "method": self.method.to_lowercase(),
            },
            "ts": {
                "path": self.ts_path(),
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
            "has_payload": self.has_payload(),
            "input_type": input_type_name,
            "output_type": output_type_name,
            "query_type": query_type_name,
        })
    }
}

#[derive(Deserialize, Debug, Default, Clone)]
#[serde(untagged)]
enum EmptyOrStringOrMap {
    #[default]
    Empty,
    String(String),
    Map(BTreeMap<String, String>),
}

impl EmptyOrStringOrMap {
    fn struct_name(&self) -> Option<&str> {
        match self {
            EmptyOrStringOrMap::Empty => None,
            EmptyOrStringOrMap::String(s) => Some(s),
            EmptyOrStringOrMap::Map(_) => None,
        }
    }

    fn is_empty(&self) -> bool {
        match self {
            EmptyOrStringOrMap::Empty => true,
            _ => false,
        }
    }
}

fn ts_field_type(t: &str) -> &str {
    match t {
        "bool" => "boolean",
        "i32" | "u32" | "i64" | "f64" | "isize" | "usize" => "number",
        "String" | "uuid" | "Uuid" => "string",
        "date" | "date-time" | "datetime" => "string",
        "json" => "object",
        t => t,
    }
}

fn rust_field_type(t: &str) -> &str {
    match t {
        "boolean" => "bool",
        "number" => "usize",
        "string" => "String",
        "uuid" | "Uuid" => "uuid::Uuid",
        "date" | "date-time" | "datetime" => "chrono::DateTime<chrono::Utc>",
        "json" => "serde_json::Value",
        t => t,
    }
}
