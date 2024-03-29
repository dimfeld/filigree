_list:
  @just --list

# Stage all the current files in git, then run Filigree
build-with-backup:
  git add .
  @just build-test-app

build-test-app:
  cd filigree-cli && cargo lbuild
  cd test-app && ../target/debug/filigree --overwrite && cargo lcheck

build-and-test *FLAGS:
  @just build-test-app
  cd test-app && cargo ltest {{FLAGS}}

build-test-app-and-db *FLAGS:
  cd filigree-cli && cargo lbuild
  cd test-app && ../target/debug/filigree --overwrite && (yes | sqlx database reset) && cargo ltest {{FLAGS}}

build-web-types:
  true
