_list:
  @just --list

# Stage all the current files in git, then run Filigree
build-with-backup:
  git add .
  @just build-test-apps

build-test-apps *FLAGS:
  cd filigree-cli && cargo lbuild
  just build-test-app sveltekit
  just build-test-app htmx

build-test-app DIR *FLAGS:
  @just write-files {{DIR}} {{FLAGS}}
  cd test-apps/{{DIR}} && cargo lcheck

write-files DIR *FLAGS:
  cd filigree-cli && cargo lbuild
  cd test-apps/{{DIR}} && ../../target/debug/filigree write {{FLAGS}}

build-and-test DIR *FLAGS:
  @just build-test-app {{DIR}}
  cd test-apps/{{DIR}} && cargo ltest {{FLAGS}}

build-test-app-and-db DIR *FLAGS:
  cd filigree-cli && cargo lbuild
  cd test-apps/{{DIR}} && ../../target/debug/filigree write --overwrite && (yes | sqlx database reset) && cargo ltest {{FLAGS}}

build-web-types:
  true
