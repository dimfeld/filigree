use std::path::PathBuf;

use error_stack::Report;
use rayon::prelude::*;

use crate::{
    config::Config,
    templates::{Renderer, ServerTemplates},
    Error, RenderedFile,
};

pub fn render_files(
    config: &Config,
    renderer: &Renderer,
) -> Result<Vec<RenderedFile>, Report<Error>> {
    let context = tera::Context::new();
    let base_path = PathBuf::from("src");

    let files = ServerTemplates::iter().collect::<Vec<_>>();
    files
        .into_par_iter()
        .map(|file| renderer.render(&base_path, &file, &context))
        .collect::<Result<Vec<_>, _>>()
}
