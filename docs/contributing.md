---
layout: layouts/page.njk
title: Contributing
description: How mido is put together, how to add a block type, and how to write and review snapshot tests.
tags: docs
order: 10
---

# Contributing

mido is small enough to read in an afternoon. This page explains how the pieces fit, then walks through the two changes people make most often: teaching mido a new Markdown element, and writing the snapshot test that proves it.

## Set up

```sh
git clone https://github.com/anistark/mido
cd mido
cargo run -- README.md
just check
```

`just check` runs `cargo fmt --check`, clippy with warnings denied, and the tests, which is what CI runs. Rust 1.93 or newer. The docs site needs pnpm and Node 24, and `just docs` starts it. `just demo` re-records the landing page video from `docs/tapes/landing.tape` and needs VHS 0.11.0 with ttyd and ffmpeg, because VHS 0.12.0 writes no output.

## The pipeline

One pipeline, two outputs. Every stage is a plain function, so each can be tested on its own.

The repository is a Cargo workspace of two crates. `crates/mido-core` holds parse, the document model, layout, the themes and the glyph tiers, with no terminal dependency beyond ratatui-core's style and text types, so another tool can embed the renderer. The `mido` binary at the root holds the viewer, the config, print mode and the project scan, and re-exports the core as `mido::markdown` and `mido::render`. Paths below that start with `core/` are under `crates/mido-core/src/`.

```
source text
  -> parse      pulldown-cmark events -> Document        core/markdown/parse.rs
  -> Document   blocks and inlines, each block with its   core/markdown/document.rs
                byte range in the source
  -> layout     Document + width + theme -> lines, plus  core/render/layout.rs
                headings, links and a source map
  -> paint      ratatui frame in the viewer              src/app/draw.rs
  -> print      ANSI text on stdout with -p              src/render/ansi.rs
```

The `Document` in the middle is the important decision. Everything else walks it: the outline, search, link following, history, print mode and the snapshot tests. A block never loses its source range, which is how a resize keeps your reading position and how the outline knows which section you are in.

Around the pipeline:

- `src/app/mod.rs` holds the state and the event loop. `src/app/keys.rs` is the keymap, the source of truth for the help overlay and the [Keys](keys.md) page.
- `src/app/outline.rs` is the foldable tree used by both panels, `finder.rs` the fuzzy file picker, `search.rs` smart-case search, `watch.rs` the file watcher.
- `src/project/scan.rs` walks a folder with the ignore crate and picks the entry file.
- `src/config.rs` reads the config and the project `.mido.toml` and resolves theme names and paths, and `src/themes.rs` prints `mido themes`.
- `core/render/wrap.rs` wraps styled text without splitting a style run, `syntax.rs` wraps syntect, `theme.rs` maps semantic tokens to styles and reads theme files, `themes/` holds the built-in themes, `glyphs.rs` the symbol tiers, and `mermaid/` turns diagram blocks into text.
- `src/docs.rs` embeds `docs/*.md` for `mido docs`, and `build.rs` copies them in and generates the man page.

## Add a block type

Say you want to render GitHub alerts, the `> [!NOTE]` blockquotes. The change touches each stage once.

1. **Model it.** Add a variant to `BlockKind` in `core/markdown/document.rs`. Keep it data only: what the block contains, nothing about how it looks.
2. **Parse it.** In `core/markdown/parse.rs` the `Builder` turns pulldown-cmark events into a tree of `Node` frames. A block with children needs a `Node` variant that collects them, a `start` arm for its `Tag`, and an `end_frame` arm that pushes the finished `Block` with its byte span through `self.block`. Blocks without children, like a rule, are pushed straight from the event.
3. **Draw it.** In `core/render/layout.rs` add an arm to `Renderer::block` and a method beside `quote`, `list` and `table`. Build styled pieces, wrap them with `wrap` at `self.avail()`, and emit lines with `emit_line`. For nested content push a prefix with `push_prefix`, render the inner blocks with `self.blocks`, then `pop_prefix`. Colors come from `core/render/theme.rs`: add a semantic token to the `tokens!` list there rather than using a raw color, and give it a value in every file under `core/render/themes/`. A test fails when a built-in theme misses one, and `just colors` fails on a raw `Color::` anywhere else. Symbols come from `core/render/glyphs.rs`, with an ASCII form for each.
4. **Check the outputs.** Print mode uses the same lines, so it is done. The viewer paints the same lines, so it is done too, unless the block interacts with links or headings, in which case the indices in `Layout` need the new entries.
5. **Test it.** A unit test in `parse.rs` for the tree, a fixture and a layout snapshot for the drawing, and a screen snapshot if the chrome changes. See the next section.
6. **Document it.** Add the element to [What renders](rendering.md) and a line to `CHANGELOG.md` under the unreleased version, in the same commit.

## Write a snapshot test

Snapshots are the main test style, through insta. The rendered text is the contract, so a change that looks different fails until someone approves it.

There are three kinds:

- **Layout snapshots** in `tests/layout.rs` render a fixture from `tests/fixtures/` at 60 columns with `layout` and snapshot the plain lines. Add a fixture file for a new element and a line to `snapshots_at_60_columns`.
- **Screen snapshots** in `tests/app.rs` drive the whole app through ratatui's `TestBackend` at 80, 120 or 140 columns, press keys with `app.press`, and snapshot the terminal buffer. Use these when panels, overlays or the status bar change.
- **Diagram snapshots** in `core/render/mermaid/` cover each Mermaid type.
- **The theme gallery** in `tests/gallery.rs` draws one screen per built-in theme and checks each paints its own colors. `just gallery` writes the pictures to `target/gallery`, and the docs site publishes them.

Writing one:

```rust
#[test]
fn alerts_render_with_a_label() {
    let text = std::fs::read_to_string("tests/fixtures/alerts.md").unwrap();
    let lines = layout(&parse(&text), &Theme::dark(), 60).plain_lines();
    insta::assert_snapshot!("alerts", lines.join("\n"));
}
```

Run `cargo test --workspace`. A new or changed snapshot is written next to the old one as a `.snap.new` file and the test fails. Review it with `cargo insta review` if you have cargo-insta installed, or set `INSTA_UPDATE=always` on one run and read the diff with `git diff tests/snapshots`. Commit the `.snap` file, never the `.snap.new`. Width matters: the fixtures also run through `lines_never_exceed_the_width`, which is the test that catches a wrap bug.

## Keys and docs

The keymap in `src/app/keys.rs` drives the help overlay and the [Keys](keys.md) page, and the theme list and tokens drive the reference half of [Themes](themes.md). After editing either run `just keys`, which regenerates the tables in `docs/keys.md` and `docs/themes.md`. A test compares the two and fails when they drift, and another checks that every command line flag appears in [Command line](cli.md).

The docs pages are plain Markdown plus front matter. A test renders each one in print mode and rejects template syntax or raw HTML, because the same files are bundled into the binary for `mido docs`. Another walks every relative link and `#anchor` across the docs, README, CONTRIBUTING and CHANGELOG and fails on one that does not land on a file or a heading. After adding a page, run `touch build.rs` once so the bundle picks it up.

## Before you open a pull request

- `just check` passes.
- `CHANGELOG.md` has a line for the change.
- Commit messages follow the `type: summary` shape in the history, for example `feat: fold sections in the outline`.
