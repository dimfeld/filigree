use error_stack::Report;

use crate::{
    config::Config, model::generator::ModelGenerator, templates::Renderer, Error, RenderedFile,
};

pub fn build_auth(
    renderer: &Renderer,
    config: &Config,
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

    let mut context = tera::Context::new();
    context.insert("user_model", user_model);
    context.insert("role_model", role_model);
    context.insert("org_model", org_model);

    todo!();
}
