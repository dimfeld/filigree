use std::path::PathBuf;

use convert_case::{Case, Casing};
use error_stack::{Report, ResultExt};
use itertools::Itertools;
use rayon::prelude::*;

use crate::{
    config::Config,
    model::generator::ModelGenerator,
    templates::{Renderer, RootApiTemplates, RootWebTemplates},
    Error, RenderedFile, RenderedFileLocation,
};

pub fn render_files(
    crate_name: &str,
    config: &Config,
    models: &[ModelGenerator],
    renderer: &Renderer,
) -> Result<Vec<RenderedFile>, Report<Error>> {
    let mut context = tera::Context::new();
    context.insert("company_name", &config.company_name);
    context.insert("product_name", &config.product_name);
    context.insert(
        "user_agent",
        config
            .server
            .user_agent
            .as_ref()
            .unwrap_or(&config.product_name),
    );
    context.insert("crate_name", &crate_name.to_case(Case::Snake));
    context.insert("email", &config.email);
    context.insert("server", &config.server);

    let server_hosts = config
        .server
        .hosts
        .iter()
        .map(|host| format!(r##""{host}".to_string()"##))
        .join(", ");
    context.insert("server_hosts", &server_hosts);
    context.insert(
        "env_prefix",
        config.server.env_prefix.as_deref().unwrap_or_default(),
    );
    context.insert("users", &config.users);
    context.insert("db", &config.database);

    let user_model = models
        .iter()
        .find(|m| m.name == "User")
        .expect("User model not found");
    let role_model = models
        .iter()
        .find(|m| m.name == "Role")
        .expect("Role model not found");
    let org_model = models
        .iter()
        .find(|m| m.name == "Organization")
        .expect("Organization model not found");

    let all_models = models
        .iter()
        .map(|gen| gen.template_context().clone().into_json())
        .collect::<Vec<_>>();

    context.insert("models", &all_models);
    context.insert(
        "user_model",
        &user_model.template_context().clone().into_json(),
    );
    context.insert(
        "role_model",
        &role_model.template_context().clone().into_json(),
    );
    context.insert(
        "org_model",
        &org_model.template_context().clone().into_json(),
    );

    let storage_context = config
        .storage
        .template_context()
        .change_context(Error::Config)?;
    context.insert("storage", &storage_context);

    let base_path = PathBuf::from("src");

    let files = RootApiTemplates::iter()
        .map(|f| (RenderedFileLocation::Api, f))
        .chain(RootWebTemplates::iter().map(|f| (RenderedFileLocation::Web, f)))
        .collect::<Vec<_>>();
    let mut output = files
        .into_par_iter()
        .filter(|(_, file)| file != "root/build.rs.tera" && file != "root/auth/fetch_base.sql.tera")
        .map(|(location, file)| {
            let filename = file.strip_prefix("root/").unwrap();
            let filename = filename.strip_suffix(".tera").unwrap_or(filename);

            let path = base_path.join(filename);
            renderer.render_with_full_path(path, &file, location, &context)
        })
        .collect::<Result<Vec<_>, _>>()?;

    // build.rs doesn't go in src
    let build_rs = renderer.render_with_full_path(
        PathBuf::from("build.rs"),
        "root/build.rs.tera",
        RenderedFileLocation::Api,
        &context,
    )?;
    output.push(build_rs);

    Ok(output)
}
