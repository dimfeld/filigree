_list:
  @just --list

build-test-app *FLAGS:
  cd filigree-cli && cargo build
  cd test-app && ../target/debug/filigree && cargo test {{FLAGS}}

build-test-app-and-db *FLAGS:
  cd filigree-cli && cargo build
  cd test-app && ../target/debug/filigree && (yes | sqlx database reset) && cargo test {{FLAGS}}

