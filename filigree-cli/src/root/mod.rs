use std::path::PathBuf;

use convert_case::{Case, Casing};
use error_stack::Report;
use rayon::prelude::*;

use crate::{config::Config, templates::Renderer, Error, RenderedFile};

pub fn render_files(
    crate_name: &str,
    config: &Config,
    renderer: &Renderer,
) -> Result<Vec<RenderedFile>, Report<Error>> {
    let files = ["main.rs.tera", "lib.rs.tera", "error.rs.tera"];

    let mut context = tera::Context::new();
    context.insert("crate_name", &crate_name.to_case(Case::Snake));
    context.insert("default_port", &config.default_port);
    context.insert("load_dotenv", &config.dotenv);
    context.insert(
        "env_prefix",
        config.env_prefix.as_deref().unwrap_or_default(),
    );

    let base_path = PathBuf::from("src");

    files
        .into_par_iter()
        .map(|file| renderer.render(&base_path, "root", file, &context))
        .collect::<Result<Vec<_>, _>>()
}
