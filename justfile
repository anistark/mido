default:
    @just --list

build:
    cargo build --release

install:
    cargo install --path .

run *args='README.md':
    cargo run -q -- {{args}}

test:
    cargo test --workspace

check: colors
    cargo fmt --check
    cargo clippy --workspace --all-targets -- -D warnings
    cargo test --workspace

colors:
    #!/usr/bin/env bash
    if grep -rn --include='*.rs' 'Color::' src crates | grep -v '^crates/mido-core/src/render/theme.rs:'; then
        echo "raw colors outside crates/mido-core/src/render/theme.rs, add a theme token instead"
        exit 1
    fi

gallery dir='target/gallery':
    MIDO_GALLERY={{dir}} cargo test -q --test gallery
    @echo "open {{dir}}/index.html"

docs:
    pnpm -C docs dev

docs-build:
    pnpm -C docs build

docs-read:
    cargo run -q -- docs/

keys:
    MIDO_UPDATE_DOCS=1 cargo test -q --test docs -- keys_page themes_page

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
    cargo publish --workspace --dry-run
    cargo publish --workspace
    echo "published mido-core and mido {{version}}"

gh-tag: _gates
    #!/usr/bin/env bash
    set -euo pipefail
    git rev-parse -q --verify "refs/tags/v{{version}}" >/dev/null && { echo "v{{version}} already exists"; exit 1; }
    git tag "v{{version}}"
    git push origin main "v{{version}}"
    echo "tagged and pushed v{{version}}"

publish: check publish-crate gh-tag
