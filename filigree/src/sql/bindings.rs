use std::fmt::{Display, Write};

/// The operator that should be used when comparing a field to a value
#[derive(Debug, Copy, Clone)]
pub enum BindingOperator {
    /// Simple equals
    Eq,
    /// Use = ANY()
    Array,
    /// Greater than or equal
    Gte,
    /// Less than or equal
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

/// Generate a WHERE clause that uses query bindings when some or all of the filters may not be
/// present.
pub struct FilterBuilder<'a> {
    clauses: Vec<(&'a str, BindingOperator)>,
    first_parameter: usize,
}

impl<'a> FilterBuilder<'a> {
    /// Create a QueryBindings, starting at the given parameter number
    pub fn new(first_parameter: usize) -> FilterBuilder<'a> {
        FilterBuilder {
            clauses: Vec::new(),
            first_parameter,
        }
    }

    /// Return true if no clauses were added
    pub fn is_empty(&self) -> bool {
        self.clauses.is_empty()
    }

    /// Compare against a Vec, if it is not empty
    pub fn add_vec<T>(&mut self, field: &'a str, values: &[T]) {
        if values.is_empty() {
            return;
        }

        self.clauses.push((field, BindingOperator::Array));
    }

    /// Compare against an Option if it is `Some`
    pub fn add_option<T>(&mut self, field: &'a str, value: &Option<T>, operator: BindingOperator) {
        if value.is_none() {
            return;
        }

        self.clauses.push((field, operator));
    }
}

impl<'a> Display for FilterBuilder<'a> {
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

/// Build a series of VALUES entries for a SQL INSERT query
pub struct ValuesBuilder {
    /// The index of the first binding to generate
    pub first_parameter: usize,
    /// The number of rows to generate
    pub num_values: usize,
    /// The number of columns in each row
    pub num_columns: usize,
}

impl Display for ValuesBuilder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut binding = self.first_parameter;
        for i in 0..self.num_values {
            if i > 0 {
                f.write_str(",\n")?;
            }

            f.write_char('(')?;
            for j in 0..self.num_columns {
                if j > 0 {
                    f.write_char(',')?;
                }

                f.write_char('$')?;
                write!(f, "{}", binding)?;
                binding += 1;
            }
            f.write_char(')')?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    mod values_builder {
        #[test]
        #[ignore = "todo"]
        fn single_row() {
            todo!()
        }

        #[test]
        #[ignore = "todo"]
        fn single_column() {
            todo!()
        }

        #[test]
        #[ignore = "todo"]
        fn multiple_rows() {
            todo!()
        }
    }
}
