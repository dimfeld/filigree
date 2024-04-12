use std::path::Path;

use filigree::{
    html_elements,
    vite_manifest::{watch::ManifestWatcher, Manifest, ManifestError},
};
use hypertext::{rsx, Renderable, Rendered};

use crate::auth::Authed;

pub static MANIFEST: Manifest = Manifest::new();

pub fn init_manifest(
    manifest_path: &Path,
    watch: bool,
) -> Result<Option<ManifestWatcher>, error_stack::Report<ManifestError>> {
    let base_url = "/";
    MANIFEST.read_manifest(base_url, manifest_path)?;

    let watcher = if watch {
        Some(filigree::vite_manifest::watch::watch_manifest(
            base_url.to_string(),
            manifest_path.to_path_buf(),
            &MANIFEST,
        ))
    } else {
        None
    };

    Ok(watcher)
}

/// The HTML shell that every page should be wrapped in to enable basic functionality.
pub fn page_wrapper(title: &str, slot: impl Renderable) -> Rendered<String> {
    let client_tags = MANIFEST.index();
    rsx! {
        <!DOCTYPE html>
        <html>
            <head>
                <meta charset="utf-8"/>
                <meta name="viewport" content="width=device-width, initial-scale=1"/>
                {client_tags}
                <title>{title}</title>
            </head>
            <body>{slot}</body>
        </html>
    }
    .render()
}

/// The root layout of the application
pub fn root_layout(auth: Option<&Authed>, slot: impl Renderable) -> impl Renderable {
    rsx! {
        {slot}
    }
}

/// The root layout of the application, as a full HTML page
pub fn root_layout_page(
    auth: Option<&Authed>,
    title: &str,
    slot: impl Renderable,
) -> Rendered<String> {
    page_wrapper(title, root_layout(auth, slot))
}
