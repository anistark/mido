default:
    @just --list

build:
    cargo build --release

install:
    cargo install --path .

run *args='README.md':
    cargo run -q -- {{args}}

test:
    cargo test

check:
    cargo fmt --check
    cargo clippy --all-targets -- -D warnings
    cargo test

docs:
    pnpm -C docs dev

docs-build:
    pnpm -C docs build

docs-read:
    cargo run -q -- docs/

keys:
    MIDO_UPDATE_DOCS=1 cargo test -q --test docs keys_page

man:
    mkdir -p target/man && cargo run -q -- --man > target/man/mido.1

demo:
    cargo build --release
    PATH="{{justfile_directory()}}/target/release:$PATH" vhs docs/tapes/landing.tape

version := `grep -m1 '^version = ' Cargo.toml | cut -d '"' -f 2`

_gates:
    #!/usr/bin/env bash
    set -euo pipefail
    branch=$(git branch --show-current)
    [[ "$branch" == "main" ]] || { echo "release from main, not $branch"; exit 1; }
    [[ -z "$(git status --porcelain)" ]] || { echo "commit or stash your changes first"; exit 1; }

publish-crate: _gates
    #!/usr/bin/env bash
    set -euo pipefail
    grep -Eq "^## \[{{version}}\] - [0-9]{4}-[0-9]{2}-[0-9]{2}" CHANGELOG.md || { echo "CHANGELOG.md needs a dated ## [{{version}}] section"; exit 1; }
    cargo publish --dry-run
    cargo publish
    echo "published mido {{version}}"

gh-tag: _gates
    #!/usr/bin/env bash
    set -euo pipefail
    git rev-parse -q --verify "refs/tags/v{{version}}" >/dev/null && { echo "v{{version}} already exists"; exit 1; }
    git tag "v{{version}}"
    git push origin main "v{{version}}"
    echo "tagged and pushed v{{version}}"

publish: check publish-crate gh-tag
