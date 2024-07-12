_list:
  @just --list

# Stage all the current files in git, then run Filigree
build-with-backup:
  cd ../filigree-test-apps && just build-with-backup

build-test-apps *FLAGS:
  cd ../filigree-test-apps && just build-test-app {{FLAGS}}

build-test-app DIR *FLAGS:
  cd ../filigree-test-apps && just build-test-app {{DIR}} {{FLAGS}}

write-files DIR *FLAGS:
  cd ../filigree-test-apps && just write-files {{DIR}} {{FLAGS}}

build-and-test DIR *FLAGS:
  cd ../filigree-test-apps && just build-and-test {{DIR}}

build-test-app-and-db DIR *FLAGS:
  cd ../filigree-test-apps && just build-test-app-and-db {{DIR}} {{FLAGS}}

