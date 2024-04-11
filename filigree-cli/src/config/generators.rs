use std::{borrow::Cow, collections::BTreeMap, ops::Deref};

use itertools::Itertools;
use serde::{Deserialize, Serialize};

use crate::Error;

/// A reference to an existing object, or a definition of a new one.
#[derive(Serialize, Deserialize, Debug, Default, Clone)]
#[serde(untagged)]
pub enum ObjectRefOrDef {
    #[default]
    Empty,
    String(String),
    Map(BTreeMap<String, String>),
}

impl ObjectRefOrDef {
    pub fn struct_name(&self) -> Option<&str> {
        match self {
            ObjectRefOrDef::Empty => None,
            ObjectRefOrDef::String(s) => Some(s),
            ObjectRefOrDef::Map(_) => None,
        }
    }

    pub fn is_empty(&self) -> bool {
        match self {
            ObjectRefOrDef::Empty => true,
            _ => false,
        }
    }

    pub fn is_definition(&self) -> bool {
        matches!(self, ObjectRefOrDef::Map(_))
    }

    fn define_rust_type(&self, prefix: &str, suffix: &str, contents: &str) -> String {
        format!("#[derive(serde::Deserialize, serde::Serialize, Debug, JsonSchema)]\npub struct {prefix}{suffix} {{\n{contents}\n}}")
    }

    fn define_ts_type(&self, prefix: &str, suffix: &str, contents: &str) -> String {
        format!("export interface {prefix}{suffix} {{\n{contents}\n}}")
    }

    pub fn type_def(&self, prefix: &str, suffix: &str) -> (String, String) {
        match self {
            ObjectRefOrDef::Empty => (
                self.define_rust_type(prefix, suffix, ""),
                self.define_ts_type(prefix, suffix, ""),
            ),
            // The structure is defined elsewhere, so we don't define it
            ObjectRefOrDef::String(_) => (String::new(), String::new()),
            ObjectRefOrDef::Map(m) => {
                let rust_contents = m
                    .iter()
                    .map(|(k, v)| format!("pub {k}: {v},\n", v = rust_field_type(v)))
                    .join("");

                let ts_contents = m
                    .iter()
                    .map(|(k, v)| format!("{k}: {v},\n", v = ts_field_type(v)))
                    .join("");

                (
                    self.define_rust_type(prefix, suffix, &rust_contents),
                    self.define_ts_type(prefix, suffix, &ts_contents),
                )
            }
        }
    }
}

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
pub struct EndpointPath(pub String);

impl Deref for EndpointPath {
    type Target = str;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl EndpointPath {
    /// Make sure paths start with a slash and do not end with a slash
    pub fn normalize(&mut self) {
        if self.0.len() > 1 && self.0.ends_with("/") {
            self.0.pop();
        }

        if !self.0.starts_with('/') {
            self.0 = format!("/{p}", p = self.0);
        }
    }

    pub fn segments(&self) -> impl Iterator<Item = &str> {
        self.0.split('/').filter(|s| !s.is_empty())
    }

    pub fn args(&self) -> impl Iterator<Item = &str> {
        self.segments()
            .filter(|s| s.starts_with(':'))
            .map(|s| &s[1..])
    }

    pub fn parse_to_rust_args(
        &self,
        id_type: &str,
        params: &BTreeMap<String, String>,
    ) -> Option<String> {
        let segments = self.args().collect::<Vec<_>>();

        if segments.is_empty() {
            return None;
        }

        let segment_types = segments
            .iter()
            .map(|&s| {
                params
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

    pub fn ts_path(&self) -> String {
        let segments = self.segments().map(|s| {
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
}

pub fn ts_field_type(t: &str) -> &str {
    match t {
        "bool" => "boolean",
        "i32" | "u32" | "i64" | "f64" | "isize" | "usize" => "number",
        "text" | "String" | "uuid" | "Uuid" => "string",
        "date" | "date-time" | "datetime" => "string",
        "json" => "object",
        t => t,
    }
}

pub fn rust_field_type(t: &str) -> &str {
    match t {
        "boolean" => "bool",
        "number" => "usize",
        "text" | "string" => "String",
        "uuid" | "Uuid" => "uuid::Uuid",
        "date" | "date-time" | "datetime" => "chrono::DateTime<chrono::Utc>",
        "json" => "serde_json::Value",
        t => t,
    }
}

pub fn rust_permission(permission: &str) -> Cow<'static, str> {
    match permission {
        "owner" => "OWNER_PERMISSION".into(),
        "write" => "WRITE_PERMISSION, OWNER_PERMISSION".into(),
        "read" => "READ_PERMISSION, WRITE_PERMISSION, OWNER_PERMISSION".into(),
        "create" => "CREATE_PERMISSION".into(),
        t => format!("\"{}\"", t).into(),
    }
}
