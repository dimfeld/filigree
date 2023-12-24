_list:
  @just --list

build-test-app:
  cd filigree-cli && cargo build
  cd test-app && ../target/debug/filigree --force-all && (yes | sqlx database reset) && cargo check
