use std::{borrow::Cow, collections::BTreeMap, sync::Arc};

use convert_case::{Case, Casing};
use serde::{Deserialize, Serialize};
use serde_json::json;

use super::generators::{EndpointPath, ObjectRefOrDef};

#[derive(Deserialize)]
pub struct PagesConfigFile {
    #[serde(flatten)]
    pub global_config: GlobalPageConfig,
    pub pages: Vec<PageConfig>,
}

impl PagesConfigFile {
    pub fn into_pages(self) -> Vec<Page> {
        let global = Arc::new(self.global_config);

        self.pages
            .into_iter()
            .map(|mut page| {
                let global = global.clone();
                page.normalize_path();
                Page {
                    config: page,
                    global,
                }
            })
            .collect::<Vec<_>>()
    }
}

#[derive(Deserialize, Debug, Default)]
pub struct GlobalPageConfig {
    /// Require auth for all pages in this file
    pub require_auth: Option<bool>,

    /// Require this permission for all pages in this file
    pub permission: Option<String>,
}

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
pub struct PageConfig {
    /// The URL for this endpoint. A parameter named `:id` will be given the ID type of the model, and all other
    /// parameters will default to `String` if not otherwise specified in `params`.
    pub path: EndpointPath,

    #[serde(default)]
    pub require_auth: Option<bool>,

    /// A permission needed to view this page.
    pub permission: Option<String>,

    /// Customize the types of certain path parameters.
    #[serde(default)]
    pub params: BTreeMap<String, String>,

    /// If set, this page is also a form with a POST handler. The value here can be a string to reference an
    /// already-existing type, or an object to define a new object.
    pub form: Option<PageForm>,

    /// The query parameters that this endpoint accepts.
    pub query: Option<ObjectRefOrDef>,

    /// Action definitions for this page. Each action is a POST endpoint that should return an
    /// HTML fragment for htmx.
    #[serde(default)]
    pub actions: Vec<PageAction>,
}

impl PageConfig {
    fn normalize_path(&mut self) {
        self.path.normalize();
        for action in &mut self.actions {
            if let Some(path) = &mut action.path {
                let new_path = path.trim_matches('/');
                if new_path.len() != path.len() {
                    *path = new_path.to_string();
                }
            }
        }
    }
}

#[derive(Debug)]
pub struct Page {
    pub config: PageConfig,
    pub global: Arc<GlobalPageConfig>,
}

impl Page {
    pub fn template_context(&self, submodules: Vec<String>) -> serde_json::Value {
        let page = &self.config;
        let global = &self.global;

        let permission = global.permission.as_deref().or(page.permission.as_deref());

        let name = if page.path.0 == "/" {
            "home".to_string()
        } else {
            page.path
                .segments()
                .filter(|s| !s.starts_with(':'))
                .last()
                .unwrap()
                .to_case(Case::Snake)
        };
        let pascal_name = name.to_case(Case::Pascal);

        let query_type_name = page
            .query
            .as_ref()
            .map(|q| {
                q.struct_name()
                    .map(Cow::Borrowed)
                    .unwrap_or_else(|| Cow::Owned(format!("{}Query", pascal_name)))
            })
            .unwrap_or_default();

        let require_auth = page.require_auth.or(global.require_auth);

        let query_struct = page
            .query
            .as_ref()
            .filter(|q| q.is_definition())
            .map(|q| q.type_def(&query_type_name, "").0);

        let main_require_auth = require_auth.unwrap_or(permission.is_some());
        let args = rust_args(
            &page.path,
            &page.params,
            main_require_auth,
            "",
            &query_type_name,
        );

        let form = if let Some(form) = &page.form {
            let input_type_name = form
                .input
                .struct_name()
                .map(Cow::Borrowed)
                .unwrap_or_else(|| Cow::Owned(format!("{}Payload", pascal_name)));

            let form_struct = if form.input.is_definition() {
                form.input.type_def(&input_type_name, "").0
            } else {
                String::new()
            };

            let form_permission = permission.or(form.permission.as_deref());
            let form_args = rust_args(
                &page.path,
                &page.params,
                form.require_auth
                    .or(require_auth)
                    .unwrap_or(form_permission.is_some()),
                &input_type_name,
                "",
            );

            json!({
              "args": form_args,
              "permission": form_permission,
              "input_type_def": form_struct
            })
        } else {
            json!(null)
        };

        json!({
            "name": name,
            "has_handler": true,
            "path": if page.path.0 == "" { "/" } else { &page.path.0 },
            "args": args,
            "require_auth": main_require_auth,
            "permission": permission,
            "query_type_def": query_struct,
            "form": form,
            "actions": page.actions.iter().map(|a| a.template_context(&page, permission, require_auth)).collect::<Vec<_>>(),
            "submodules": submodules,
        })
    }
}

/// Form handler configuration
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PageForm {
    /// The payload type for this form.
    #[serde(default)]
    input: ObjectRefOrDef,

    /// Require authentication to submit the form, even if anonymous users can view the page.
    require_auth: Option<bool>,

    permission: Option<String>,
}

/// A custom action for a page
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PageAction {
    name: String,
    /// The subpath of this action, which will be placed under `{page path}/_action/{path}`
    /// If omitted, it will use `name` as the path.
    path: Option<String>,

    method: String,

    /// Require users to be authenticated to perform this action. If omitted and the parent page
    /// requires authentication, then this action will to.
    require_auth: Option<bool>,

    /// A permission needed to perform this action. If omitted and the
    /// action's parent page has a permission, that permission will be used.
    permission: Option<String>,

    /// Customize the types of certain path parameters.
    #[serde(default)]
    params: BTreeMap<String, String>,

    /// The query parameters that this action accepts.
    #[serde(default)]
    query: Option<ObjectRefOrDef>,

    /// The payload of this action.
    #[serde(default)]
    input: Option<ObjectRefOrDef>,
}

impl PageAction {
    pub fn template_context(
        &self,
        parent: &PageConfig,
        parent_permission: Option<&str>,
        parent_require_auth: Option<bool>,
    ) -> serde_json::Value {
        let parent_path = if parent.path.0 == "/" {
            ""
        } else {
            &parent.path.0
        };

        let full_path = EndpointPath(format!(
            "{parent_path}/_action/{path}",
            parent_path = parent_path,
            path = self.path.as_deref().unwrap_or(&self.name)
        ));

        let has_input = self.input.is_some();
        let pascal_name = self.name.to_case(Case::Pascal);

        let input_name = if has_input {
            format!("{pascal_name}ActionPayload")
        } else {
            String::new()
        };

        let query_name = if self.query.is_some() {
            format!("{pascal_name}ActionQuery")
        } else {
            String::new()
        };

        let input_struct = self
            .input
            .as_ref()
            .filter(|d| d.is_definition())
            .map(|i| i.type_def(&input_name, "").0)
            .unwrap_or_default();

        let query_struct = self
            .query
            .as_ref()
            .filter(|d| d.is_definition())
            .map(|i| i.type_def(&query_name, "").0)
            .unwrap_or_default();

        let permission = self.permission.as_deref().or(parent_permission);

        json!({
            "name": self.name,
            "path": full_path.0,
            "method": self.method.to_lowercase(),
            "permission": permission,
            "input_type_def": input_struct,
            "query_type_def": query_struct,
            "args": rust_args(
                &full_path,
                &self.params,
                self.require_auth.or(parent_require_auth).unwrap_or(permission.is_some()),
                &input_name,
                &query_name
            )
        })
    }
}

fn rust_args(
    path: &EndpointPath,
    path_params: &BTreeMap<String, String>,
    require_authed: bool,
    input_type_name: &str,
    query_type_name: &str,
) -> String {
    let mut args = vec![
        "State(state): State<ServerState>".to_string(),
        if require_authed {
            "auth: Authed".to_string()
        } else {
            "auth: Option<Authed>".to_string()
        },
    ];

    if let Some(path) = path.parse_to_rust_args("String", path_params) {
        args.push(path);
    }

    if !query_type_name.is_empty() {
        args.push(format!("Query(qs): Query<{}>", query_type_name));
    }

    if !input_type_name.is_empty() {
        args.push(format!(
            "ValidatedForm {{ data, form, errors }}: ValidatedForm<{}>",
            input_type_name
        ));
    }

    args.join(",\n")
}
