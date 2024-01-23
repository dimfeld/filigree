use std::{borrow::Cow, fmt::Display};

pub mod file;
pub mod json_schema;
pub mod multipart;
pub mod urlencoded;

#[derive(Debug, Clone)]
pub struct ContentType<'a>(pub Cow<'a, str>);

impl<'a> Display for ContentType<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl<'a> ContentType<'a> {
    pub fn new(content_type: impl Into<Cow<'a, str>>) -> Self {
        Self(content_type.into())
    }

    pub fn is_json(&self) -> bool {
        self.0.starts_with("application/json")
    }

    pub fn is_form(&self) -> bool {
        self.0.starts_with("application/x-www-form-urlencoded")
    }
}
