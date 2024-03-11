use std::fmt::Display;

use url::Url;

/// Write an Option<String> out in a way that will compile to the same thing.
/// None remains "None", Some("abc") becomes "Some(r##"abc"##.to_string())
pub struct OptionAsString<'a>(pub &'a Option<String>);

impl Display for OptionAsString<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.0 {
            None => write!(f, "None"),
            Some(s) => write!(f, r###"Some(r##"{}"##.to_string())"###, s),
        }
    }
}

/// Write an Option<Url> out in a way that will compile to the same thing.
/// None remains "None", Some("abc") becomes "Some(r##"abc"##.parse()?)
pub struct OptionAsStorageUrl<'a>(pub &'a Option<Url>);

impl Display for OptionAsStorageUrl<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.0 {
            None => write!(f, "None"),
            Some(s) => {
                write!(
                    f,
                    r###"Some(r##"{}"##.parse().map(StorageError::Configuration("Invalid endpoint URL)))"###,
                    s.to_string(),
                )
            }
        }
    }
}
