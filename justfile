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
