//! Helpers for parsing configuration

use std::env::VarError;

/// Get an environment variable with an optional prefix
pub fn prefixed_env_var(prefix: &str, key: &str) -> Result<String, VarError> {
    if prefix.is_empty() {
        std::env::var(key)
    } else {
        std::env::var(format!("{prefix}{key}"))
    }
}

/// Parse an `Option<String>`, returning an error if the value is present and fails to parse to the expected type.
pub fn parse_option<T: std::str::FromStr>(value: Option<String>) -> Result<Option<T>, T::Err> {
    match value {
        Some(v) => Ok(Some(v.parse()?)),
        None => Ok(None),
    }
}

/// Try two options for an environment variable, or fallback to a default value. This is used
/// when setting up storage providers.
pub fn double_env_var_fallback<T: std::str::FromStr>(
    first: &str,
    second: &str,
    fallback_value: T,
) -> Result<T, T::Err> {
    if let Ok(val) = std::env::var(first) {
        val.parse()
    } else if let Ok(val) = std::env::var(second) {
        val.parse()
    } else {
        Ok(fallback_value)
    }
}

/// Set `dest` to the value of `src`, if src is Some
pub fn merge_option_if_set<T>(dest: &mut Option<T>, src: Option<T>) {
    if src.is_some() {
        *dest = src;
    }
}
