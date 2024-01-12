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
    models: &[(String, serde_json::Value)],
    renderer: &Renderer,
) -> Result<Vec<RenderedFile>, Report<Error>> {
    let mut context = tera::Context::new();
    context.insert("company_name", &config.company_name);
    context.insert("product_name", &config.product_name);
    context.insert("crate_name", &crate_name.to_case(Case::Snake));
    context.insert("default_port", &config.default_port);
    context.insert("load_dotenv", &config.dotenv);
    context.insert("email", &config.email);
    context.insert(
        "env_prefix",
        config.env_prefix.as_deref().unwrap_or_default(),
    );
    context.insert("users", &config.users);
    context.insert("db", &config.database);

    let user_model = &models
        .iter()
        .find(|(name, _)| name == "User")
        .expect("User model not found")
        .1;
    let role_model = &models
        .iter()
        .find(|(name, _)| name == "Role")
        .expect("Role model not found")
        .1;
    let org_model = &models
        .iter()
        .find(|(name, _)| name == "Organization")
        .expect("Organization model not found")
        .1;

    let all_models = models.iter().map(|(_, value)| value).collect::<Vec<_>>();
    context.insert("models", &all_models);
    context.insert("user_model", user_model);
    context.insert("role_model", role_model);
    context.insert("org_model", org_model);

    let base_path = PathBuf::from("src");

    let files = RootTemplates::iter().collect::<Vec<_>>();
    let mut output = files
        .into_par_iter()
        .filter(|file| file != "root/build.rs.tera")
        .map(|file| {
            let filename = file.strip_prefix("root/").unwrap();
            let filename = filename.strip_suffix(".tera").unwrap_or(filename);

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
