use std::path::PathBuf;

use error_stack::Report;
use rayon::prelude::*;

use crate::{
    config::Config,
    templates::{Renderer, UsersTemplates},
    Error, RenderedFile,
};

pub fn render_files(
    _config: &Config,
    renderer: &Renderer,
    models: &[(String, serde_json::Value)],
) -> Result<Vec<RenderedFile>, Report<Error>> {
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

    let mut context = tera::Context::new();
    context.insert("models", &all_models);
    context.insert("user_model", user_model);
    context.insert("role_model", role_model);
    context.insert("org_model", org_model);

    let files = UsersTemplates::iter().collect::<Vec<_>>();
    let dir = PathBuf::from("src");
    files
        .into_par_iter()
        .map(|file| renderer.render(&dir, &file, &context))
        .collect::<Result<Vec<_>, _>>()
}
