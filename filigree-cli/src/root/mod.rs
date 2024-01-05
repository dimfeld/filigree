use std::path::PathBuf;

use convert_case::{Case, Casing};
use error_stack::Report;
use rayon::prelude::*;

use crate::{
    config::Config,
    templates::{Renderer, RootTemplates},
    Error, RenderedFile,
};

pub fn render_files(
    crate_name: &str,
    config: &Config,
    renderer: &Renderer,
) -> Result<Vec<RenderedFile>, Report<Error>> {
    let mut context = tera::Context::new();
    context.insert("crate_name", &crate_name.to_case(Case::Snake));
    context.insert("default_port", &config.default_port);
    context.insert("load_dotenv", &config.dotenv);
    context.insert(
        "env_prefix",
        config.env_prefix.as_deref().unwrap_or_default(),
    );
    context.insert(
        "require_email_verification",
        &config.require_email_verification,
    );
    context.insert("db", &config.database);

    let base_path = PathBuf::from("src");

    let files = RootTemplates::iter().collect::<Vec<_>>();
    let mut output = files
        .into_par_iter()
        .filter(|file| file != "root/build.rs.tera")
        .map(|file| {
            let filename = file
                .strip_prefix("root/")
                .unwrap()
                .strip_suffix(".tera")
                .unwrap();
            let path = base_path.join(filename);
            renderer.render_with_full_path(path, &file, &context)
        })
        .collect::<Result<Vec<_>, _>>()?;

    // build.rs doesn't go in src
    let build_rs = renderer.render_with_full_path(
        PathBuf::from("build.rs"),
        "root/build.rs.tera",
        &context,
    )?;
    output.push(build_rs);

    Ok(output)
}
