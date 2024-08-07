use std::{fs, path::Path};

use cargo_toml::{Dependency, Manifest};

fn main() {
    // Read the Cargo.toml file of the current package
    let cargo_toml_path = Path::new("Cargo.toml");
    let manifest = Manifest::from_path(cargo_toml_path).expect("Failed to read Cargo.toml");

    // Extract the filigree dependency
    let filigree_dep = manifest
        .dependencies
        .get("filigree")
        .expect("Failed to find filigree in dependencies");

    // Get the version of filigree
    let version = match filigree_dep {
        Dependency::Simple(ver) => ver.clone(),
        Dependency::Detailed(dep) => dep
            .version
            .clone()
            .expect("Failed to extract version from filigree dependency"),
        _ => panic!("Unsupported dependency format for filigree"),
    };

    // Set the filigree version in the Cargo environment
    println!("cargo:rustc-env=FILIGREE_VERSION={}", version);
    println!("cargo:rerun-if-changed=Cargo.toml");
}
