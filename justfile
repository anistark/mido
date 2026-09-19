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
