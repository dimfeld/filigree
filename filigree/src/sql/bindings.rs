use std::fmt::{Display, Write};

#[derive(Debug, Copy, Clone)]
pub enum BindingOperator {
    Eq,
    Array,
    Gte,
    Lte,
}

impl BindingOperator {
    fn write(&self, f: &mut std::fmt::Formatter<'_>, param: usize) -> std::fmt::Result {
        match self {
            BindingOperator::Eq => write!(f, "= ${param}"),
            BindingOperator::Array => write!(f, "= ANY(${param})"),
            BindingOperator::Gte => write!(f, ">= ${param}"),
            BindingOperator::Lte => write!(f, "<= ${param}"),
        }
    }
}

pub struct QueryBindings<'a> {
    clauses: Vec<(&'a str, BindingOperator)>,
    first_parameter: usize,
}

impl<'a> QueryBindings<'a> {
    /// Create a QueryBindings, starting at the given parameter number
    pub fn new(first_parameter: usize) -> QueryBindings<'a> {
        QueryBindings {
            clauses: Vec::new(),
            first_parameter,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.clauses.is_empty()
    }

    pub fn add_vec<T>(&mut self, field: &'a str, values: &[T]) {
        if values.is_empty() {
            return;
        }

        self.clauses.push((field, BindingOperator::Array));
    }

    pub fn add_option<T>(&mut self, field: &'a str, value: &Option<T>, operator: BindingOperator) {
        if value.is_none() {
            return;
        }

        self.clauses.push((field, operator));
    }
}

impl<'a> Display for QueryBindings<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.is_empty() {
            // We need to write something, but don't have any conditions, so just "true" will
            // suffice. The query planner is smart enough to just ignore this.
            return f.write_str("true");
        }

        f.write_char('(')?;
        for (i, (field, operator)) in self.clauses.iter().enumerate() {
            if i > 0 {
                f.write_str(" AND ")?;
            }

            let param = self.first_parameter + i;
            f.write_str(field)?;
            f.write_char(' ')?;
            operator.write(f, param)?;
        }
        f.write_char(')')?;

        Ok(())
    }
}
