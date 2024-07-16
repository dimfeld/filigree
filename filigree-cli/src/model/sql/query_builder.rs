//! Query building helpers. This is modeled after [sqlx::QueryBuilder] but the binding tracking
//! works a bit differently to better support the building queries in the CLI.
use std::{borrow::Cow, fmt::Write};

use super::SqlQueryContext;

/// A simple query builder that keeps track of named bindings
pub struct QueryBuilder {
    pub bindings: Vec<String>,
    pub query: String,
}

impl QueryBuilder {
    pub fn new() -> Self {
        Self {
            bindings: Vec::new(),
            query: String::new(),
        }
    }

    /// Initialize the [QueryBuilder] with an initial query fragment and bindings.
    pub fn with_initial(query: String, bindings: Vec<String>) -> Self {
        Self { bindings, query }
    }

    /// Add a string to the query
    pub fn push(&mut self, sql: &str) {
        self.query.push_str(sql);
    }

    /// Create or reuse a binding, but don't add anything to the query. This returns the
    /// number of the binding, which can be prefixed with a `$` to use in a query.
    pub fn create_binding_index(&mut self, name: &str) -> usize {
        self.bindings
            .iter()
            .position(|b| b == name)
            .unwrap_or_else(|| {
                self.bindings.push(name.to_string());
                self.bindings.len() - 1
            })
            + 1
    }

    /// Create a binding and return a string with a prefixed `$` that can be pasted directly into a query.
    pub fn create_binding(&mut self, name: &str) -> String {
        let index = self.create_binding_index(name);
        format!("${}", index)
    }

    /// Create or reuse a binding, and add a the corresponding parameter syntax to the query.
    pub fn push_binding(&mut self, name: &str) {
        let index = self.create_binding_index(name);
        write!(&mut self.query, "${}", index).unwrap();
    }

    /// Return the query and bindings from the builder.
    pub fn finish(self, name: &str) -> SqlQueryContext {
        SqlQueryContext {
            bindings: self.bindings,
            query: self.query,
            name: name.to_string(),
        }
    }

    pub fn separated<'a, 'b>(&'a mut self, sep: impl Into<Cow<'b, str>>) -> Separated<'a, 'b> {
        Separated::new(self, sep)
    }
}

impl std::fmt::Write for QueryBuilder {
    fn write_str(&mut self, s: &str) -> std::fmt::Result {
        self.query.write_str(s)
    }

    fn write_char(&mut self, c: char) -> std::fmt::Result {
        self.query.write_char(c)
    }

    fn write_fmt(&mut self, args: std::fmt::Arguments<'_>) -> std::fmt::Result {
        self.query.write_fmt(args)
    }
}

/// Build part of a query that is separated by a string
pub struct Separated<'a, 'b> {
    builder: &'a mut QueryBuilder,
    sep: Cow<'b, str>,
    first: bool,
    on_first: &'b str,
}

impl<'a, 'b> Separated<'a, 'b> {
    fn new(builder: &'a mut QueryBuilder, sep: impl Into<Cow<'b, str>>) -> Self {
        Self {
            builder,
            sep: sep.into(),
            first: true,
            on_first: "",
        }
    }

    /// Define a string to be written just before the first push call.
    /// For example you could use `sep.on_first(" WHERE ")` which would then only
    /// output the WHERE if something else was written.
    pub fn on_first(&mut self, on_first: &'b str) {
        self.on_first = on_first;
    }

    /// Add a string to the query, prefixed with the separator if appropriate
    pub fn push(&mut self, sql: &str) {
        if self.first {
            self.first = false;
            if !self.on_first.is_empty() {
                self.builder.push(self.on_first);
            }
        } else {
            self.builder.push(&self.sep);
        }
        self.builder.push(sql);
    }

    /// Add a string to the query, without the separator.
    pub fn push_unseparated(&mut self, sql: &str) {
        self.builder.push(sql);
    }

    /// Add a binding to the query, prefixed with the separator if appropriate
    pub fn push_binding(&mut self, name: &str) {
        if self.first {
            self.first = false;
            if !self.on_first.is_empty() {
                self.builder.push(self.on_first);
            }
        } else {
            self.builder.push(&self.sep);
        }
        self.builder.push_binding(name);
    }

    /// Add a binding to the query, without a separator
    pub fn push_binding_unseparated(&mut self, name: &str) {
        self.builder.push_binding(name);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn query_builder_new() {
        let builder = QueryBuilder::new();
        assert_eq!(builder.query, "");
        assert!(builder.bindings.is_empty());
    }

    #[test]
    fn query_builder_with_initial() {
        let builder =
            QueryBuilder::with_initial("SELECT * FROM users".to_string(), vec!["id".to_string()]);
        assert_eq!(builder.query, "SELECT * FROM users");
        assert_eq!(builder.bindings, vec!["id"]);
    }

    #[test]
    fn query_builder_push() {
        let mut builder = QueryBuilder::new();
        builder.push("SELECT * FROM users");
        assert_eq!(builder.query, "SELECT * FROM users");
    }

    #[test]
    fn query_builder_create_binding_index() {
        let mut builder = QueryBuilder::new();
        let index1 = builder.create_binding_index("name");
        let index2 = builder.create_binding_index("age");
        let index3 = builder.create_binding_index("name");
        assert_eq!(index1, 1);
        assert_eq!(index2, 2);
        assert_eq!(index3, 1);
        assert_eq!(builder.bindings, vec!["name", "age"]);
    }

    #[test]
    fn query_builder_create_binding() {
        let mut builder = QueryBuilder::new();
        let index1 = builder.create_binding("name");
        let index2 = builder.create_binding("age");
        let index3 = builder.create_binding("name");
        assert_eq!(index1, "$1");
        assert_eq!(index2, "$2");
        assert_eq!(index3, "$1");
        assert_eq!(builder.bindings, vec!["name", "age"]);
    }

    #[test]
    fn query_builder_push_binding() {
        let mut builder = QueryBuilder::new();
        builder.push_binding("name");
        builder.push_binding("age");
        builder.push_binding("name");
        assert_eq!(builder.query, "$1$2$1");
        assert_eq!(builder.bindings, vec!["name", "age"]);
    }

    #[test]
    fn query_builder_finish() {
        let mut builder = QueryBuilder::new();
        builder.push("SELECT * FROM users WHERE name = ");
        builder.push_binding("name");
        let result = builder.finish("query.sql");
        assert_eq!(result.query, "SELECT * FROM users WHERE name = $1");
        assert_eq!(result.bindings, vec!["name"]);
        assert_eq!(result.name, "query.sql");
    }

    #[test]
    fn separated() {
        let mut builder = QueryBuilder::new();
        {
            let mut sep = builder.separated(", ");
            sep.on_first("WHERE ");
            sep.push("column1");
            sep.push("column2");
            sep.push_binding("param");
        }
        assert_eq!(builder.query, "WHERE column1, column2, $1");
        assert_eq!(builder.bindings, vec!["param"]);
    }

    #[test]
    fn separated_push_unseparated() {
        let mut builder = QueryBuilder::new();
        {
            let mut sep = builder.separated(", ");
            sep.push_unseparated("(");
            sep.push("column1");
            sep.push("column2");
            sep.push_unseparated(")");
        }
        assert_eq!(builder.query, "(column1, column2)");
    }

    #[test]
    fn complex_query_building() {
        let mut builder = QueryBuilder::new();
        builder.push("SELECT ");
        {
            let mut columns = builder.separated(", ");
            columns.push("id");
            columns.push("name");
            columns.push("age");
        }
        builder.push(" FROM users WHERE ");
        {
            let mut conditions = builder.separated(" AND ");
            conditions.push("active = true");
            conditions.push("age > ");
            conditions.push_binding("min_age");
        }
        builder.push(" ORDER BY name");

        let result = builder.finish("a-query.sql");
        assert_eq!(
            result.query,
            "SELECT id, name, age FROM users WHERE active = true AND age > $1 ORDER BY name"
        );
        assert_eq!(result.bindings, vec!["min_age"]);
        assert_eq!(result.name, "a-query.sql");
    }
}
