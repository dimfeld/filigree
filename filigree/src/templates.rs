use std::path::PathBuf;

use axum::response::Response;
use minijinja::Environment;
use minijinja_autoreload::AutoReloader;
use serde::Serialize;

/// Templates with optional support for hot reloading
pub enum Templates {
    /// A template store that does not support hot reloading.
    Fixed(minijinja::Environment<'static>),
    #[cfg(feature = "template_reload")]
    /// A template store that supports hot reloading.
    Reloading(minijinja_autoreload::AutoReloader),
}

impl Templates {
    /// Create a template store
    pub fn new(environment: Environment<'static>) -> Self {
        Self::Fixed(environment)
    }

    #[cfg(feature = "template_reload")]
    /// Create a template store with hot reloading
    pub fn new_with_reloader(
        vite_manifest_path: Option<PathBuf>,
        template_path: PathBuf,
        f: impl Fn(
                &mut minijinja::Environment<'static>,
            ) -> Result<minijinja::Environment<'static>, minijinja::Error>
            + Send
            + Sync
            + 'static,
    ) -> Self {
        let reloader = AutoReloader::new(move |notifier| {
            if let Some(vite_manifest_path) = vite_manifest_path.as_ref() {
                notifier.watch_path(&vite_manifest_path, false);
            }
            notifier.watch_path(&template_path, true);

            let mut env = minijinja::Environment::new();
            let base_path = template_path.clone();
            env.set_loader(move |name| {
                let (path, fragment) = template_fragments::split_path(name);
                let path = base_path.join(path);
                let source = std::fs::read_to_string(path).ok();
                match (source, fragment.is_empty()) {
                    (None, _) => Ok(None),
                    (Some(source), true) => Ok(Some(source)),
                    (Some(source), false) => {
                        let fragment_src = template_fragments::filter_template(&source, fragment)
                            .map_err(|e| {
                            minijinja::Error::new(
                                minijinja::ErrorKind::SyntaxError,
                                format!("Error extracting template fragment {fragment}"),
                            )
                            .with_source(e)
                        })?;
                        Ok(Some(fragment_src))
                    }
                }
            });

            f(&mut env)?;

            Ok(env)
        });

        Self::Reloading(reloader)
    }

    /// Get the reloader notifier, if any applies
    pub fn notifier(&self) -> Option<minijinja_autoreload::Notifier> {
        match self {
            Self::Fixed(_) => None,
            #[cfg(feature = "template_reload")]
            Self::Reloading(reloader) => Some(reloader.notifier()),
        }
    }

    /// Render a template with the supplied context
    pub fn render<T: Serialize>(
        &self,
        template: &str,
        data: T,
    ) -> Result<Response, minijinja::Error> {
        let body = match self {
            Self::Fixed(env) => env.get_template(template)?.render(data)?,
            #[cfg(feature = "template_reload")]
            Self::Reloading(reloader) => reloader
                .acquire_env()?
                .get_template(template)?
                .render(data)?,
        };

        Ok(axum::response::Response::builder()
            .header("content-type", "text/html; charset=utf-8")
            .body(axum::body::Body::from(body))
            .unwrap())
    }
}

/// Add a template and split out the fragments
pub fn add_template_and_fragments(
    env: &mut Environment<'static>,
    name: &'static str,
    template: &'static str,
) -> Result<(), minijinja::Error> {
    env.add_template(name, template)?;

    let fragments = template_fragments::split_templates(template).map_err(|e| {
        minijinja::Error::new(
            minijinja::ErrorKind::SyntaxError,
            format!("Error extracting template fragments from {}", name),
        )
        .with_source(e)
    })?;

    for (fragment_name, fragment) in fragments {
        let template_name = template_fragments::join_path(name, &fragment_name);
        env.add_template_owned(template_name, fragment)?;
    }

    Ok(())
}
