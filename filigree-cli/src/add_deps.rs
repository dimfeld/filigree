use cargo_toml::Manifest;
use error_stack::{Report, ResultExt};
use semver::{Version, VersionReq};

use crate::Error;

type DepVersion = (&'static str, &'static str, &'static [&'static str]);

const DEPS: &[DepVersion] = &[
    ("async-trait", "0.1.75", &[]),
    ("axum", "0.7.3", &["tokio", "http1", "http2", "macros"]),
    ("chrono", "0.4.31", &[]),
    ("clap", "4.4.11", &["env", "derive"]),
    ("dotenvy", "0.15.7", &[]),
    ("error-stack", "0.4.1", &[]),
    ("eyre", "0.6.11", &[]),
    ("filigree", "0.0.1", &[]),
    ("futures", "0.3.30", &[]),
    ("hyper", "1.1.0", &["server", "http1", "http2"]),
    ("reqwest", "0.11.23", &["cookies", "json"]),
    ("rust-embed", "8.1.0", &[]),
    ("serde", "1.0.193", &["derive"]),
    ("serde_json", "1.0.108", &[]),
    ("sqlx", "0.7.3", &["chrono", "postgres"]),
    ("tera", "1.19.1", &[]),
    ("thiserror", "1.0.52", &[]),
    ("tokio", "1.35.1", &["full"]),
    ("tower", "0.4.13", &[]),
    ("tower-cookies", "0.10.0", &[]),
    ("tower-http", "0.5.0", &["full"]),
    ("tracing", "0.1.40", &[]),
    ("tracing-subscriber", "0.3.18", &["chrono"]),
    ("uuid", "1.6.1", &[]),
];

pub fn add_deps(manifest: &Manifest) -> Result<(), Report<Error>> {
    for (name, version, features) in DEPS {
        let existing = manifest.dependencies.get(*name);
        let Some(existing) = existing else {
            run_cargo_add(name, version, features)?;
            continue;
        };

        let desired = VersionReq::parse(version).expect("version requirement");

        let existing_version = Version::parse(existing.req())
            .change_context(Error::ReadConfigFile)
            .attach_printable_lazy(|| {
                format!("Invalid req {} = {} in Cargo.toml", name, existing.req())
            })?;

        if !desired.matches(&existing_version) {
            run_cargo_add(name, version, features)?;
            continue;
        }

        let existing_features = existing.req_features();
        if !features
            .iter()
            .all(|feature| existing_features.iter().any(|f| f == feature))
        {
            run_cargo_add(name, version, features)?;
            continue;
        }
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
