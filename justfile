_list:
  @just --list

build-test-app:
  cd filigree-cli && cargo build
  cd test-app && ../target/debug/filigree && cargo check

build-test-app-and-db:
  cd filigree-cli && cargo build
  cd test-app && ../target/debug/filigree && (yes | sqlx database reset) && cargo check

