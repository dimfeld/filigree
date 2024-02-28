_list:
  @just --list

build-test-app:
  cd filigree-cli && cargo build
  cd test-app && ../target/debug/filigree

build-and-test *FLAGS:
  @just build-test-app
  cd test-app && cargo test {{FLAGS}}

build-test-app-and-db *FLAGS:
  cd filigree-cli && cargo build
  cd test-app && ../target/debug/filigree && (yes | sqlx database reset) && cargo test {{FLAGS}}

build-web-types:
  true
