use std::path::Path;

use cargo_toml::Manifest;
use error_stack::Report;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::{add_deps::add_dep, Error};

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct WebConfig {
    /// The frontend framework to use
    pub framework: Option<WebFramework>,

    /// When using a separate frontend server, set to true if the server should forward requests that
    /// it doesn't handle to the frontend. This should be used for SvelteKit if you aren't setting
    /// up the equivalent behavior with a reverse proxy.
    ///
    /// When omitted, the default value is 5173 when `framework` is set to `sveltekit`, or disabled otherwise.
    pub port: Option<u16>,

    /// Serve frontend static assets from this directory when in production mode. If omitted, defaults to:
    ///
    /// - "<web_directory>/build/client" when framework is sveltekit
    /// - "<web_directory>/build" when framework is htmx
    ///
    /// This can be set at runtime using the WEB_ASSET_DIR environment variable
    pub files: Option<String>,

    pub forward_to_frontend: Option<bool>,
}

impl WebConfig {
    pub fn template_context(&self, web_relative_to_api: &Path) -> serde_json::Value {
        json!({
            "framework": self.framework,
            "port": self.port(),
            "files": self.files(web_relative_to_api),
            "has_api_pages": self.has_api_pages(),
            "generate_js_types_code": self.generate_js_types_code(),
        })
    }

    pub fn files(&self, web_relative_to_api: &Path) -> Option<String> {
        if self.files.is_some() {
            return self.files.clone();
        }

        match self.framework {
            Some(WebFramework::SvelteKit) => Some(
                web_relative_to_api
                    .join("build")
                    .join("client")
                    .to_string_lossy()
                    .to_string(),
            ),
            Some(WebFramework::Htmx) => Some(
                web_relative_to_api
                    .join("build")
                    .to_string_lossy()
                    .to_string(),
            ),
            _ => None,
        }
    }

    pub fn port(&self) -> Option<u16> {
        if let Some(port) = self.port {
            return Some(port);
        }

        match self.framework {
            Some(WebFramework::SvelteKit) => Some(5173),
            _ => None,
        }
    }

    pub fn add_deps(&self, cwd: &Path, manifest: &mut Manifest) -> Result<(), Report<Error>> {
        match self.framework {
            Some(WebFramework::Htmx) => Self::add_htmx_deps(cwd, manifest)?,
            _ => {}
        }

        if self.generate_js_types_code() {
            add_dep(cwd, manifest, "schemars-zod", "0.1.5", &[])?;
        }

        Ok(())
    }

    fn add_htmx_deps(cwd: &Path, manifest: &mut Manifest) -> Result<(), Report<Error>> {
        add_dep(cwd, manifest, "hypertext", "0.5.0", &["axum"])?;
        add_dep(cwd, manifest, "axum-htmx", "0.5.0", &[])?;

        Ok(())
    }

    pub fn filigree_features(&self) -> Vec<&'static str> {
        match self.framework {
            Some(WebFramework::Htmx) => vec!["htmx", "hypertext", "watch-manifest"],
            _ => vec![],
        }
    }

    /// If this application renders pages from the API. This controls if the API `pages` templates
    /// are rendered
    pub fn has_api_pages(&self) -> bool {
        match self.framework {
            Some(WebFramework::Htmx) => true,
            Some(WebFramework::SvelteKit) => false,
            None => false,
        }
    }

    pub fn generate_js_types_code(&self) -> bool {
        matches!(self.framework, Some(WebFramework::SvelteKit))
    }
}

/// The frontend framework to use
#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub enum WebFramework {
    /// This application uses Hypertext/RSX/Maud and HTMX to render its frontend
    #[serde(rename = "htmx")]
    Htmx,
    /// This application uses a SvelteKit with a separate server for its frontend
    #[serde(rename = "sveltekit")]
    SvelteKit,
}
