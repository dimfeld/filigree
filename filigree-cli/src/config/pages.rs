use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::json;

use super::generators::ObjectRefOrDef;

pub struct PagesConfigFile {
    pages: Vec<PageConfig>,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct PageConfig {
    /// The URL for this endpoint. A parameter named `:id` will be given the ID type of the model, and all other
    /// parameters will default to `String` if not otherwise specified in `params`.
    path: String,
    /// Customize the types of certain path parameters.
    #[serde(default)]
    params: BTreeMap<String, String>,

    /// If set, this page is also a form with a POST handler. The value here can be a string to reference an
    /// already-existing type, or an object to define a new object.
    #[serde(default)]
    form: Option<ObjectRefOrDef>,

    /// The query parameters that this endpoint accepts.
    #[serde(default)]
    query: Option<ObjectRefOrDef>,

    /// Action definitions for this page. Each action is a POST endpoint that should return an
    /// HTML fragment for htmx.
    actions: Vec<PageAction>,
}

impl PageConfig {
    pub fn template_context(&self) -> serde_json::Value {
        json!({
            "path": self.path,
            "args": todo!(),
            "form": todo!(),
            "actions": self.actions.iter().map(|a| a.template_context()).collect::<Vec<_>>(),
        })
    }
}

/// A custom action for a page
#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct PageAction {
    /// The subpath of this action, which will be placed under `{page path}/_action/{path}`
    path: String,

    method: String,

    /// Customize the types of certain path parameters.
    #[serde(default)]
    params: BTreeMap<String, String>,

    /// The query parameters that this endpoint accepts.
    #[serde(default)]
    query: Option<ObjectRefOrDef>,

    /// The payload of this action.
    #[serde(default)]
    payload: Option<ObjectRefOrDef>,
}

impl PageAction {
    pub fn template_context(&self) -> serde_json::Value {
        json!({
            "path": self.path,
            "method": self.method,
            "args": todo!(),
        })
    }
}
