use cargo_toml::Manifest;
use error_stack::{Report, ResultExt};
use semver::{Version, VersionReq};

use crate::Error;

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
    ("chrono", "0.4.33", &[]),
    ("clap", "4.4.11", &["env", "derive"]),
    ("dotenvy", "0.15.7", &[]),
    ("error-stack", "0.4.1", &["spantrace"]),
    ("eyre", "0.6.11", &[]),
    ("filigree", "0.0.1", &[]),
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
    ("sqlx-transparent-json-decode", "2.2.0", &[]),
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

pub fn add_fixed_deps(manifest: &Manifest) -> Result<(), Report<Error>> {
    for dep in DEPS {
        add_dep(manifest, dep)?;
    }

    Ok(())
}

pub fn add_dep(
    manifest: &Manifest,
    (name, version, features): &DepVersion,
) -> Result<(), Report<Error>> {
    let existing = manifest.dependencies.get(*name);
    let Some(existing) = existing else {
        run_cargo_add(name, version, features)?;
        return Ok(());
    };

    if !existing.is_crates_io() {
        // This is a git or path dependency, so don't change it.
        return Ok(());
    }

    let desired = VersionReq::parse(version).expect("version requirement");

    let Ok(existing_version) = Version::parse(existing.req()) else {
        // Let the user know that we were unable to parse the version in Cargo.toml
        // but don't fail since it could be intentionally this way.
        eprintln!(
                "WARN: Unable to parse version {} for {name} in Cargo.toml. Only plain versions are supported.",
                existing.req()
            );
        return Ok(());
    };

    if !desired.matches(&existing_version) {
        run_cargo_add(name, version, features)?;
        return Ok(());
    }

    let existing_features = existing.req_features();
    if !features
        .iter()
        .all(|feature| existing_features.iter().any(|f| f == feature))
    {
        run_cargo_add(name, version, features)?;
        return Ok(());
    }

    Ok(())
}

fn run_cargo_add(name: &str, version: &str, features: &[&str]) -> Result<(), Report<Error>> {
    let operation = if features.is_empty() {
        format!("Adding depdendency {name}@{version}")
    } else {
        format!("Adding depdendency {name}@{version} with features {features:?}")
    };

    println!("{operation}");

    let mut cmd = std::process::Command::new("cargo");
    cmd.arg("add");
    cmd.arg(&format!("{name}@{version}"));

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
