use std::path::Path;

use cargo_toml::{Dependency, DependencyDetail, Manifest};
use error_stack::{Report, ResultExt};
use semver::{Version, VersionReq};

use crate::{config::Config, Error};

pub type DepVersion<'a> = (&'a str, &'a str, &'a [&'a str]);

const DEPS: &[DepVersion<'static>] = &[
    ("async-trait", "0.1.75", &[]),
    ("axum", "0.7.3", &["tokio", "http1", "http2", "macros"]),
    ("axum-extra", "0.9.2", &["query"]),
    ("axum-jsonschema", "0.8.0", &[]),
    (
        "axum-sqlx-tx",
        "0.8.0",
        &["postgres", "runtime-tokio-rustls"],
    ),
    ("bytes", "1.5.0", &[]),
    ("chrono", "0.4.33", &[]),
    ("clap", "4.4.11", &["env", "derive"]),
    ("dialoguer", "0.11.0", &[]),
    ("dotenvy", "0.15.7", &[]),
    ("error-stack", "0.4.1", &["spantrace"]),
    ("eyre", "0.6.11", &[]),
    ("futures", "0.3.30", &[]),
    ("http", "1.0.0", &[]),
    ("hyper", "1.1.0", &["server", "http1", "http2"]),
    ("percent-encoding", "2.3.1", &[]),
    ("reqwest", "0.11.23", &["cookies", "json"]),
    ("rust-embed", "8.1.0", &[]),
    ("schemars", "0.8.16", &["chrono", "url", "uuid1"]),
    ("serde", "1.0.193", &["derive"]),
    ("serde_json", "1.0.113", &[]),
    ("serde_with", "3.6.1", &["json", "schemars_0_8"]),
    ("sqlx", "0.7.3", &["chrono", "postgres"]),
    ("sqlx-transparent-json-decode", "2.2.2", &[]),
    ("tera", "1.19.1", &[]),
    ("thiserror", "1.0.56", &[]),
    ("tokio", "1.36.0", &["full"]),
    ("tower", "0.4.13", &[]),
    ("tower-cookies", "0.10.0", &[]),
    ("tower-http", "0.5.1", &["full"]),
    ("tracing", "0.1.40", &[]),
    ("tracing-subscriber", "0.3.18", &["chrono"]),
    ("url", "2.5.0", &[]),
    ("uuid", "1.6.1", &[]),
];

const DEV_DEPS: &[DepVersion<'static>] = &[("temp-dir", "0.1.13", &[])];

pub fn add_fixed_deps(
    cwd: &Path,
    config: &Config,
    manifest: &mut Manifest,
) -> Result<(), Report<Error>> {
    let mut filigree_features = vec![];

    match config.error_reporting.provider {
        crate::config::ErrorReportingProvider::Sentry => {
            add_dep(
                cwd,
                manifest,
                "sentry",
                "0.32.2",
                &[
                    "tokio",
                    "tower",
                    "tower-http",
                    "tower-axum-matched-path",
                    "tracing",
                ],
            )?;

            add_dep(
                cwd,
                manifest,
                "sentry-tower",
                "0.32.2",
                &["http", "axum-matched-path"],
            )?;

            filigree_features.push("sentry");
        }
        crate::config::ErrorReportingProvider::None => {}
    };

    filigree_features.extend(config.web.filigree_features());

    add_dep(cwd, manifest, "filigree", "0.1.1", &filigree_features)?;

    for (name, version, features) in DEPS {
        add_dep(cwd, manifest, name, version, features)?;
    }

    if config.use_queue {
        crate::config::job::add_deps(cwd, manifest)?;
    }

    for (name, version, features) in DEV_DEPS {
        add_dev_dep(cwd, manifest, name, version, features)?;
    }

    Ok(())
}

pub fn add_dep(
    cwd: &Path,
    manifest: &mut Manifest,
    name: &str,
    version: &str,
    features: &[&str],
) -> Result<(), Report<Error>> {
    let existing = manifest.dependencies.get(name);
    let added = add_dep_internal(cwd, existing, name, version, features, "")?;

    if added {
        manifest.dependencies.insert(
            name.to_string(),
            cargo_toml::Dependency::Detailed(DependencyDetail {
                version: Some(version.to_string()),
                features: features.iter().map(|s| s.to_string()).collect(),
                ..Default::default()
            }),
        );
    }

    Ok(())
}

pub fn add_dev_dep(
    cwd: &Path,
    manifest: &mut Manifest,
    name: &str,
    version: &str,
    features: &[&str],
) -> Result<(), Report<Error>> {
    let existing = manifest.dev_dependencies.get(name);
    let added = add_dep_internal(cwd, existing, name, version, features, "--dev")?;

    if added {
        manifest.dev_dependencies.insert(
            name.to_string(),
            cargo_toml::Dependency::Detailed(DependencyDetail {
                version: Some(version.to_string()),
                features: features.iter().map(|s| s.to_string()).collect(),
                ..Default::default()
            }),
        );
    }

    Ok(())
}

fn add_dep_internal(
    cwd: &Path,
    existing: Option<&Dependency>,
    name: &str,
    version: &str,
    features: &[&str],
    mode_flag: &str,
) -> Result<bool, Report<Error>> {
    let Some(existing) = existing else {
        run_cargo_add(cwd, name, version, features, mode_flag)?;
        return Ok(true);
    };

    if !existing.is_crates_io() {
        // This is a git or path dependency, so don't change it.
        return Ok(false);
    }

    let desired = VersionReq::parse(version).expect("version requirement");

    let Ok(existing_version) = Version::parse(existing.req()) else {
        // Let the user know that we were unable to parse the version in Cargo.toml
        // but don't fail since it could be intentionally this way.
        eprintln!(
                "WARN: Unable to parse version {} for {name} in Cargo.toml. Only plain versions are supported.",
                existing.req()
            );
        return Ok(false);
    };

    if !desired.matches(&existing_version) {
        run_cargo_add(cwd, name, version, features, mode_flag)?;
        return Ok(true);
    }

    let existing_features = existing.req_features();
    if !features
        .iter()
        .all(|feature| existing_features.iter().any(|f| f == feature))
    {
        run_cargo_add(cwd, name, version, features, mode_flag)?;
        return Ok(true);
    }

    Ok(false)
}

fn run_cargo_add(
    cwd: &Path,
    name: &str,
    version: &str,
    features: &[&str],
    mode_flag: &str,
) -> Result<(), Report<Error>> {
    let operation = if features.is_empty() {
        format!("Adding depdendency {name}@{version}")
    } else {
        format!("Adding depdendency {name}@{version} with features {features:?}")
    };

    println!("{operation}");

    let mut cmd = std::process::Command::new("cargo");
    cmd.current_dir(cwd);
    cmd.arg("add");
    cmd.arg(&format!("{name}@{version}"));

    if !mode_flag.is_empty() {
        cmd.arg(mode_flag);
    }

    for feature in features {
        cmd.arg("-F");
        cmd.arg(feature);
    }

    let result = cmd
        .spawn()
        .change_context(Error::Cargo)
        .attach_printable_lazy(|| operation.clone())?
        .wait()
        .change_context(Error::Cargo)
        .attach_printable_lazy(|| operation.clone())?;
    if !result.success() {
        Err(Error::Cargo).attach_printable(operation)?
    }

    Ok(())
}
