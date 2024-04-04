use std::path::PathBuf;

use convert_case::{Case, Casing};
use error_stack::{Report, ResultExt};
use itertools::Itertools;
use rayon::prelude::*;

use crate::{
    config::Config,
    model::generator::ModelGenerator,
    templates::{Renderer, RootApiTemplates, RootWebTemplates},
    write::{RenderedFile, RenderedFileLocation},
    Error,
};

pub fn render_files(
    crate_name: &str,
    config: &Config,
    web_relative_to_api: PathBuf,
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
    context.insert("secrets", &config.secrets);
    let frontend_asset_dir = config.server.frontend_asset_dir.clone().unwrap_or_else(|| {
        web_relative_to_api
            .join("build")
            .join("client")
            .to_string_lossy()
            .to_string()
    });
    context.insert("frontend_asset_dir", &frontend_asset_dir);

    let job_list = config
        .job
        .iter()
        .map(|(k, _)| k.to_case(Case::Snake))
        .sorted()
        .collect::<Vec<_>>();
    context.insert("job_list", &job_list);

    let job_workers = crate::config::job::workers_context(&config.worker, &config.job);
    context.insert("job_workers", &job_workers);

    if config.use_queue {
        context.insert("queue", &config.queue.template_context());
    }

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

    context.insert("web_relative_to_api", &web_relative_to_api);

    let mut shared_types = config.shared_types.clone();
    for model in models {
        let module = model.module_name();
        let types = model
            .shared_types
            .iter()
            .map(|s| format!("crate::models::{}::{}", module, s));
        shared_types.extend(types);
    }

    context.insert("shared_types", &shared_types);

    let storage_context = config
        .storage
        .template_context()
        .change_context(Error::Config)?;
    context.insert("storage", &storage_context);

    let base_path = PathBuf::from("src");
    // These files don't go in src and so should not have it prepended.
    let non_base_files = ["build.rs", "tailwind.config.js"];

    let web_files = RootWebTemplates::iter().map(|f| (RenderedFileLocation::Web, f));
    let api_files = RootApiTemplates::iter().map(|f| (RenderedFileLocation::Api, f));

    let files = web_files.chain(api_files).collect::<Vec<_>>();

    let job_template_path = "root/jobs/_one_job.rs.tera";
    let skip_files = [
        "root/auth/fetch_base.sql.tera",
        "root/build.rs.tera",
        job_template_path,
    ];

    let mut output = files
        .into_par_iter()
        .filter(|(_, file)| !skip_files.contains(&file.as_ref()))
        .map(|(location, file)| {
            let filename = file.strip_prefix("root/").unwrap();
            let filename = filename.strip_suffix(".tera").unwrap_or(filename);

            let path = if non_base_files.contains(&filename) {
                PathBuf::from(filename)
            } else {
                base_path.join(filename)
            };
            renderer.render_with_full_path(path, &file, location, &context)
        })
        .collect::<Result<Vec<_>, _>>()?;

    let job_template = config
        .job
        .iter()
        .map(|(k, v)| {
            let module_name = k.to_case(Case::Snake);
            let context = tera::Context::from_value(v.template_context(k)).unwrap();

            let output_path = base_path.join(format!("jobs/{module_name}.rs"));

            renderer.render_with_full_path(
                output_path,
                job_template_path,
                RenderedFileLocation::Api,
                &context,
            )
        })
        .collect::<Result<Vec<_>, _>>()?;
    output.extend(job_template);

    Ok(output)
}
