use error_stack::Report;

use crate::{
    config::Config, model::generator::ModelGenerator, templates::Renderer, Error, RenderedFile,
};

pub fn build_auth(
    renderer: &Renderer,
    config: &Config,
    models: &[ModelGenerator],
) -> Result<Vec<RenderedFile>, Report<Error>> {
    let user_model = models
        .iter()
        .find(|model| model.model.name == "User")
        .expect("User model not found")
        .context
        .clone()
        .into_json();
    let role_model = models
        .iter()
        .find(|model| model.model.name == "Role")
        .expect("Role model not found")
        .context
        .clone()
        .into_json();
    let org_model = models
        .iter()
        .find(|model| model.model.name == "Organization")
        .expect("Organization model not found")
        .context
        .clone()
        .into_json();

    let mut context = tera::Context::new();
    context.insert("user_model", &user_model);
    context.insert("role_model", &role_model);
    context.insert("org_model", &org_model);

    todo!();
}
