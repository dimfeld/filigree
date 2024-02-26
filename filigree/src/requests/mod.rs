use std::{borrow::Cow, fmt::Display};

pub mod file;
pub mod json_schema;
pub mod multipart;
pub mod urlencoded;

/// A wrapper for a content type with convenience methods for matching relevant types
#[derive(Debug, Clone)]
pub struct ContentType<'a>(pub Cow<'a, str>);

impl<'a> Display for ContentType<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl<'a> ContentType<'a> {
    /// Create a ContentType
    pub fn new(content_type: impl Into<Cow<'a, str>>) -> Self {
        Self(content_type.into())
    }

    /// Check if the content type is JSON
    pub fn is_json(&self) -> bool {
        self.0.starts_with("application/json")
    }

    /// Check if the content type is form
    pub fn is_form(&self) -> bool {
        self.0.starts_with("application/x-www-form-urlencoded")
    }

    /// Check if the content type is multipart
    pub fn is_multipart(&self) -> bool {
        self.0.starts_with("multipart/form-data")
    }
}
