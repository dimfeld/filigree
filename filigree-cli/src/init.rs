use std::{
    io::Write,
    path::{Path, PathBuf},
};

use clap::Args;
use dialoguer::theme::ColorfulTheme;
use error_stack::{Report, ResultExt};
use rand::Rng;

use crate::{
    config::{
        web::{WebConfig, WebFramework},
        Config, DatabaseConfig, EmailConfig, ErrorReportingConfig, ServerConfig,
    },
    print_env::{EnvVarOverrides, PrintConfig},
    Error,
};

#[derive(Args, Debug)]
pub struct Command {
    /// Create the project even if the target directory is not empty
    #[clap(short, long)]
    force: bool,

    /// Where to create the project, or the current directory if omitted.
    dir: Option<String>,
}

fn valid_id(input: &String, message: &str) -> Result<(), String> {
    if input
        .chars()
        .any(|c| c.is_whitespace() || (c.is_ascii_punctuation() && c != '_' && c != '-'))
    {
        Err(message.to_string())
    } else {
        Ok(())
    }
}

fn no_whitespace(input: &String) -> Result<(), String> {
    if input.chars().any(|c| c.is_whitespace()) {
        Err("Name cannot contain whitespace".to_string())
    } else {
        Ok(())
    }
}

pub fn run(cmd: Command) -> Result<(), Report<Error>> {
    // todo check for directory not empty

    let dir = Path::new(cmd.dir.as_deref().unwrap_or("."));

    // Ask basic questions:
    let app_name = dialoguer::Input::with_theme(&ColorfulTheme::default())
        .with_prompt("What identifier would you like to use for your application?")
        .validate_with(|input: &String| {
            valid_id(input, "Application name must be a valid identifier")
        })
        .interact()
        .change_context(Error::Input)?;

    let framework = dialoguer::Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Select a web framework")
        .item("htmx")
        .item("SvelteKit")
        .item("None")
        .interact()
        .change_context(Error::Input)?;

    let framework = match framework {
        0 => Some(WebFramework::Htmx),
        1 => Some(WebFramework::SvelteKit),
        _ => None,
    };

    let api_dir = dialoguer::Input::with_theme(&ColorfulTheme::default())
        .with_prompt("API source directory?")
        .allow_empty(true)
        .with_initial_text(".")
        .validate_with(no_whitespace)
        .interact_text()
        .change_context(Error::Input)?;

    let web_dir = dialoguer::Input::with_theme(&ColorfulTheme::default())
        .with_prompt("Web source directory?")
        .allow_empty(true)
        .with_initial_text("web")
        .validate_with(no_whitespace)
        .interact_text()
        .change_context(Error::Input)?;

    let create_database = dialoguer::Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt("Create a database for this project?")
        .interact()
        .change_context(Error::Input)?;

    let database_url = if create_database {
        let existing = std::env::var("DATABASE_URL").unwrap_or(String::new());
        let database_url = dialoguer::Input::<String>::with_theme(&ColorfulTheme::default())
            .with_prompt("Connect to this PostgreSQL server, or leave blank to use psql default")
            .with_initial_text(existing)
            .allow_empty(true)
            .interact_text()
            .change_context(Error::Input)?;

        let database_name = dialoguer::Input::with_theme(&ColorfulTheme::default())
            .with_prompt("What would you like to name your database?")
            .default(app_name.clone())
            .validate_with(|input: &String| {
                valid_id(input, "Database name cannot contain spaces or punctuation")
            })
            .interact_text()
            .change_context(Error::Input)?;

        let create_db_user = dialoguer::Confirm::with_theme(&ColorfulTheme::default())
            .with_prompt("Create a user for this database?")
            .default(true)
            .interact()
            .change_context(Error::Input)?;

        let db_user = create_db_user.then(|| {
            let is_superuser = dialoguer::Confirm::with_theme(&ColorfulTheme::default())
                .with_prompt("Should the database user be a superuser? This is necessary for the #[sqlx:test] macro to work.")
                .default(true)
                .interact()
                .change_context(Error::Input)?;

            let user_name = dialoguer::Input::with_theme(&ColorfulTheme::default())
                .with_prompt("Enter the username")
                .with_initial_text(&app_name)
                .validate_with(|input: &String| {
                    valid_id(input, "Username cannot contain spaces or punctuation")
                })
                .interact_text()
                .change_context(Error::Input)?;

            let password = dialoguer::Password::with_theme(&ColorfulTheme::default())
                .with_prompt(
                    "Enter a password for the database user, or leave blank to generate a random password",
                )
                .allow_empty_password(true)
                .interact()
                .change_context(Error::Input)?;

            let password = if password.is_empty() {
                rand::thread_rng()
                    .sample_iter(&rand::distributions::Alphanumeric)
                    .take(40)
                    .map(char::from)
                    .collect()
            } else {
                password
            };

            Ok::<_, Report<Error>>((user_name, is_superuser, password))
        }).transpose()
        .change_context(Error::Input)?;

        let sql = if let Some((user, is_superuser, password)) = &db_user {
            let superuser = if *is_superuser { "SUPERUSER" } else { "" };
            format!(
                "CREATE USER \"{user}\" WITH {superuser} PASSWORD '{password}';\nCREATE DATABASE {database_name} WITH OWNER \"{user}\";\n"

            )
        } else {
            format!("CREATE DATABASE {database_name};\n")
        };

        let mut psql = std::process::Command::new("psql");
        psql.arg("-a");
        if !database_url.is_empty() {
            psql.arg("-d").arg(&database_url);
        }

        let mut psql = psql
            .stdin(std::process::Stdio::piped())
            .spawn()
            .change_context(Error::Psql)?;

        psql.stdin
            .take()
            .unwrap()
            .write_all(sql.as_bytes())
            .change_context(Error::Psql)?;

        psql.wait().change_context(Error::Psql)?;

        let mut url = if database_url.is_empty() {
            url::Url::parse("postgres://localhost:5432").unwrap()
        } else {
            url::Url::parse(&database_url).unwrap()
        };

        url.set_path(&database_name);

        if let Some((user, _, password)) = &db_user {
            url.set_username(&user).ok();
            url.set_password(Some(password.as_str())).ok();
        }

        Some(url.to_string())
    } else {
        None
    };

    let config = Config {
        product_name: app_name.clone(),
        company_name: String::new(),
        api_dir: PathBuf::from(api_dir.clone()),
        web_dir: PathBuf::from(web_dir.clone()),
        server: ServerConfig {
            dotenv: true,
            hosts: vec!["localhost".to_string()],
            env_prefix: None,
            ..Default::default()
        },
        error_reporting: ErrorReportingConfig::default(),
        web: WebConfig {
            framework,
            ..Default::default()
        },

        tracing: Default::default(),
        formatter: Default::default(),
        database: DatabaseConfig {
            migrate_on_start: true,
            ..Default::default()
        },
        email: EmailConfig {
            provider: crate::config::EmailProvider::None,
            from: "support@example.com".to_string(),
        },
        secrets: Default::default(),
        shared_types: Default::default(),
        default_auth_scope: crate::model::ModelAuthScope::Model,
        users: crate::config::UsersConfig::default(),
        extend: crate::config::ExtendConfig::default(),
        storage: crate::config::storage::StorageConfig::default(),
        queue: crate::config::job::QueueConfig::default(),
        job: Default::default(),
        worker: Default::default(),
        use_queue: false,
    };

    // Create a basic Cargo.toml
    std::fs::DirBuilder::new()
        .recursive(true)
        .create(&api_dir)
        .change_context(Error::WriteFile)
        .attach_printable("Failed to create API directory")?;
    std::fs::DirBuilder::new()
        .recursive(true)
        .create(&web_dir)
        .change_context(Error::WriteFile)
        .attach_printable("Failed to create Web directory")?;

    let cargo_init = std::process::Command::new("cargo")
        .arg("init")
        .arg("--bin")
        .arg("--name")
        .arg(&app_name)
        .arg(&api_dir)
        .status()
        .change_context(Error::Cargo)?;
    if !cargo_init.success() {
        return Err(Report::new(Error::Cargo));
    }

    let rustfmt_toml_path = Path::new(&api_dir).join("rustfmt.toml");
    if !rustfmt_toml_path.exists() {
        println!("Creating rustfmt.toml");
        std::fs::write(&rustfmt_toml_path, "edition = \"2021\"\n")
            .change_context(Error::WriteFile)
            .attach_printable_lazy(|| rustfmt_toml_path.display().to_string())?;
    }

    // Create a basic web directory
    match framework {
        Some(WebFramework::SvelteKit) => {
            let create_web = dialoguer::Confirm::new()
                .with_prompt("Create SvelteKit project?")
                .interact()
                .change_context(Error::Input)?;

            if create_web {
                let run = std::process::Command::new("npm")
                    .arg("create")
                    .arg("svelte@latest")
                    .arg(&web_dir)
                    .status()
                    .change_context(Error::Npm)?;
                if !run.success() {
                    return Err(Report::new(Error::Npm)
                        .attach_printable("Failed to initialize SvelteKit project"));
                }
            }
        }
        // For HTMX we just use the templates
        _ => {}
    };

    let framework_str = framework
        .map(|f| format!("framework = \"{}\"", f))
        .unwrap_or_default();

    let config_toml = indoc::formatdoc! {r##"
        product_name = "{app_name}"
        company_name = ""

        default_auth_scope = "{auth_scope}"

        [error_reporting]
        provider = "none"

        [secrets]

        [server]
        dotenv = true
        hosts = ["localhost"]

        [web]
        {framework_str}

        [tracing]
        provider = "none"
        api_service_name = "{app_name}-api"


        [formatter]
        rust = ["rustfmt", "--edition", "2021"]
        js = ["prettier", "--stdin-filepath=stdin.ts"]
        sql = ["pg_format"]

        [database]
        migrate_on_start = true

        [email]
        provider = "none"
        from = "support@example.com"
    "##,
    auth_scope = config.default_auth_scope,
    };

    println!("Writing filigree/config.toml");
    std::fs::DirBuilder::new()
        .recursive(true)
        .create(&dir.join("filigree"))
        .change_context(Error::WriteFile)
        .attach_printable("Creating directories")?;
    let config_file_path = dir.join("filigree/config.toml");
    let config_file = std::fs::File::create_new(&config_file_path);
    match config_file {
        Ok(mut file) => {
            file.write_all(config_toml.as_bytes())
                .change_context(Error::WriteFile)
                .attach_printable_lazy(|| config_file_path.display().to_string())?;
        }
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
            println!("Skipping config.toml creation because it already exists");
        }
        Err(e) => {
            return Err(e)
                .change_context(Error::WriteFile)
                .attach_printable_lazy(|| config_file_path.display().to_string());
        }
    }

    let web_relative_to_api = pathdiff::diff_paths(&web_dir, &api_dir).unwrap();

    let pc = PrintConfig {
        mode: crate::print_env::EnvPrintMode::Shell,
        env_prefix: config.server.env_prefix.clone().unwrap_or_default(),
        print_comments: true,
    };

    let mut env_vars: Vec<u8> = Vec::new();
    crate::print_env::write_env_vars(
        &mut env_vars,
        config,
        web_relative_to_api,
        EnvVarOverrides {
            database_url: database_url.clone(),
            dev: Some(true),
        },
        &pc,
    )
    .change_context(Error::WriteFile)?;
    let env_path = dir.join(".env");

    println!("Writing .env");
    let mut env_file = std::fs::File::options()
        .append(true)
        .create(true)
        .open(&env_path)
        .change_context(Error::WriteFile)
        .attach_printable_lazy(|| env_path.display().to_string())?;

    env_file
        .write_all(&env_vars)
        .change_context(Error::WriteFile)
        .attach_printable_lazy(|| env_path.display().to_string())?;

    println!("Adding entries to .gitignore");
    let extra_git_ignore = indoc::indoc! {"
        .env
        .env.*
    "};

    let gitignore_path = dir.join(".gitignore");
    let mut gitignore = std::fs::File::options()
        .append(true)
        .create(true)
        .open(&gitignore_path)
        .change_context(Error::WriteFile)
        .attach_printable_lazy(|| gitignore_path.display().to_string())?;
    gitignore
        .write_all(extra_git_ignore.as_bytes())
        .change_context(Error::WriteFile)
        .attach_printable_lazy(|| gitignore_path.display().to_string())?;

    println!("Initialization complete!");
    println!("Your configuration file is as `filigree/config.toml.`");
    println!("Once you've set it up, un `filigree write` to scaffold yuor project.");

    Ok(())
}
