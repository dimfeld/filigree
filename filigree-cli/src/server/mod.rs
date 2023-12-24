use std::path::PathBuf;

use error_stack::Report;
use rayon::prelude::*;

use crate::{config::Config, templates::Renderer, Error, RenderedFile};

pub fn render_files(
    config: &Config,
    renderer: &Renderer,
) -> Result<Vec<RenderedFile>, Report<Error>> {
    let files = ["mod.rs.tera"];

    let context = tera::Context::new();
    let base_path = PathBuf::from("src/server");

    files
        .into_par_iter()
        .map(|file| renderer.render(&base_path, "server", file, &context))
        .collect::<Result<Vec<_>, _>>()
}
