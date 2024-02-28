use serde::Deserialize;

/// A query string with a boolean `populate` member
#[derive(Deserialize, Debug, Clone)]
pub struct Populate {
    /// Whether or not to populate the children of the model
    pub populate: Option<bool>,
}
