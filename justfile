_list:
  @just --list

build-test-app:
  cd filigree-cli && cargo lbuild
  cd test-app && ../target/debug/filigree && cargo lcheck

build-and-test *FLAGS:
  @just build-test-app
  cd test-app && cargo ltest {{FLAGS}}

build-test-app-and-db *FLAGS:
  cd filigree-cli && cargo lbuild
  cd test-app && ../target/debug/filigree && (yes | sqlx database reset) && cargo ltest {{FLAGS}}

build-web-types:
  true
