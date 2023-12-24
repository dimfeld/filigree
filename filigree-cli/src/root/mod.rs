use std::path::PathBuf;

use error_stack::Report;
use rayon::prelude::*;

use crate::{templates::Renderer, Error, RenderedFile};

pub fn render_files(renderer: &Renderer) -> Result<Vec<RenderedFile>, Report<Error>> {
    let files = ["lib.rs.tera", "error.rs.tera"];

    let context = tera::Context::new();
    let base_path = PathBuf::from("src");

    files
        .into_par_iter()
        .map(|file| renderer.render(&base_path, "root", file, &context))
        .collect::<Result<Vec<_>, _>>()
}
