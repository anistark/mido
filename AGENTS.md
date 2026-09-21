# AGENTS.md

Context file for coding agents working on mido. Read this before touching anything.
Keep it current: if you change the pipeline, a boundary, or a decision recorded here, update this file in the same commit.

This is the primary agent context file. `CLAUDE.md` points here and holds nothing of its own.

---

## What mido is

mido: Markdown In, Document Out. A terminal Markdown reader in Rust on ratatui, published on crates.io as `mido`. It opens one file or a whole folder and renders it the way a good reader app does: real heading hierarchy, a comfortable measure, highlighted code, tables that line up, pictures where the terminal can draw them, and everything it cannot render shown as source rather than dropped.

Principles, in order: looks like a document, not `cat` with colors. Keyboard first, vim-flavored, and the mouse works too. Instant on a 5k-line file. Zero config to start. Works over SSH, in tmux and on a 16-color terminal with no special font. Degrades honestly.

Non-goals, which are what stop a plausible request from turning it into something else:

- Not a pager or a syntax highlighter. Structure and whitespace come first, color second.
- Not an editor. `E` shells out to `$EDITOR`. A built-in editor is planned for 0.6 and a split view for 0.7, and nothing before then should grow editing state.
- Not a converter. Print mode writes styled text for a pager, not HTML.
- No network by surprise. Nothing is downloaded unless `--remote-images` is given.
- No config file yet. Themes, fonts and config arrive in 0.5. Until then every visual decision is a token in `src/render/theme.rs`, not a flag.

---

## Architecture

One pipeline, two outputs. Every stage is a plain function and each can be tested alone.

```
source text
  -> parse      pulldown-cmark events -> Document        src/markdown/parse.rs
  -> Document   blocks and inlines, every block keeps    src/markdown/document.rs
                its byte range in the source
  -> layout     Document + width + theme -> lines, plus  src/render/layout.rs
                headings, links, images and a source map
  -> paint      ratatui frame in the viewer              src/app/draw.rs
  -> print      ANSI text on stdout with -p              src/render/ansi.rs
```

`Document` is the important decision. The outline, search, link following, history, print mode and the snapshot tests all walk it, and a block never loses its source range, which is how a resize keeps the reading position.

### Module map

| Path | What it owns |
| --- | --- |
| `src/main.rs`, `src/cli.rs` | clap entry and mode dispatch: file, folder, stdin, print, `docs`, `--man` |
| `src/markdown/` | `parse.rs` builds the tree from pulldown-cmark events, `document.rs` is the data model plus front matter fields, `slug.rs` makes GitHub-compatible anchors |
| `src/render/layout.rs` | `Document` to styled lines at a width, with the heading, link, image and source indices |
| `src/render/wrap.rs` | Styled word wrap over `Piece`s that never splits a style run |
| `src/render/theme.rs` | Semantic color tokens to ratatui styles, with truecolor, 256-color and mono tiers |
| `src/render/badge.rs` | Badge detection, static shields URL and SVG parsing, badge colors, the badge walker |
| `src/render/syntax.rs`, `src/render/mermaid/` | syntect wrapper, Mermaid blocks to text through mmdflux and mermaid-text |
| `src/render/ansi.rs` | Print mode |
| `src/app/mod.rs` | App state and event loop: navigation, history, link selection, relayout |
| `src/app/draw.rs` | Panels, cards, status bar, overlays, image slots |
| `src/app/keys.rs` | The keymap table, source of truth for the help overlay and `docs/keys.md` |
| `src/app/outline.rs`, `finder.rs`, `search.rs`, `watch.rs`, `images.rs` | Foldable tree for both panels, fuzzy file picker, smart-case search, file watcher, image and badge fetching with the cache |
| `src/project/scan.rs` | Folder walk with the ignore crate, tree order, titles, entry file |
| `src/docs.rs`, `build.rs` | Embed `docs/*.md` for `mido docs` and generate the man page |

### Hard rules

- **`Document` is data only.** Nothing about how a block looks lives in `src/markdown/`. A new element gets a variant there, a parse arm, and a draw method in `layout.rs`, in that order. `docs/contributing.md` walks through it.
- **The renderer never uses a raw color.** Colors are tokens on `Theme` with a style method beside them. Badge colors are the one content-driven exception and go through `Theme::label_colored`, which adapts them to the color tier.
- **Print mode and the viewer paint the same lines.** A change to `layout.rs` is done for both at once. Only chrome, pictures and overlays live in `draw.rs`.
- **One keymap.** `src/app/keys.rs` drives the help overlay and `docs/keys.md`, and a test fails when the page drifts. The key table in the README is kept by hand.
- **Nothing reaches the network without `--remote-images`.** Local images always load. Remote images and badge values are fetched only with the flag: images cached under the user cache directory with a 10 MB cap, badges with a 5 s timeout and a 64 KB cap and no disk cache.

---

## Build and test

```
just check        # cargo fmt --check, clippy with warnings denied, cargo test: what CI runs
just run file.md  # cargo run with a file, README.md by default
just keys         # regenerate docs/keys.md from the keymap after editing it
just docs         # the docs site on pnpm and Node 24, at localhost:8080
just demo         # re-record the landing page video with VHS 0.11.0
```

`just check` is the one to run before handing work back. Rust edition 2024, minimum 1.93, and CI runs an MSRV job with `cargo check --all-targets` on that version, so let chains and `Option::is_none_or` are fine and anything newer is not. Linux, macOS and Windows all run the tests.

Snapshots are the main test style, through insta, and the rendered text is the contract:

- Layout snapshots in `tests/layout.rs` render a fixture from `tests/fixtures/` at 60 columns and snapshot the plain lines.
- Screen snapshots in `tests/app.rs` drive the whole app through ratatui's `TestBackend` at 80, 120 or 140 columns.
- Diagram snapshots in `src/render/mermaid/` cover each Mermaid type.

cargo-insta is not installed. A changed snapshot fails the test and writes a `.snap.new` beside it. Run `INSTA_UPDATE=always cargo test`, read `git diff tests/snapshots`, and commit the `.snap`, never the `.snap.new`. A bare `cargo test` stops at the first failing test binary, so add `--no-fail-fast` to see every suite.

`tests/docs.rs` runs the built binary over every page in `docs/` in print mode with `NO_COLOR` and rejects template syntax, raw HTML, a missing title and a leaked front matter line. It also checks that every CLI flag is on the command line page, that `docs/keys.md` matches the keymap, and that the bundled docs match the folder.

Internal planning notes go in `plan/`, which is gitignored along with `.claude/`.

---

## Rendering

The design is part of the product. `docs/rendering.md` says what each element looks like and is the reference for a user, and `src/render/theme.rs` says what color it takes.

- Every element has a treatment: H1 is a filled title chip, H2 a text-width underline, inline code sits padded on the code background, links show their URL dimmed, front matter and badges are two-tone label chips, an image alone in a paragraph becomes a picture and an inline one becomes `▣ alt (path)`.
- Wrapping is in-house. `wrap()` takes `Piece`s: text pieces break at spaces, atomic pieces never break, and adjacent atomic pieces form one unbreakable word, which is how a two-part chip stays together. Trailing spaces are trimmed unless they carry a background.
- Layout owns the measure. Full terminal width by default, `--width` caps and centers, two columns of gutter each side from 60 wide. Exactly one blank line between blocks, two before H1 and H2.
- Color tiers: truecolor when `COLORTERM` says so, else 256, else 16. `NO_COLOR` gives mono, and every element has a text-only form there: backticks for code, a heavy rule under H1, `[key: value]` for chips.
- Anything mido cannot render, raw HTML, math, a diagram type the crates miss, is shown as source, never dropped.

---

## Release

Versions follow SemVer with Keep a Changelog. Before 1.0 a minor bump may change keys or configuration.

The version lives in `Cargo.toml` only. `cargo update --workspace` moves the `Cargo.lock` entry, and the docs site reads it from `Cargo.toml` at build time. `CHANGELOG.md` needs a dated `## [x.y.z] - YYYY-MM-DD` section for the version before it can be published. Strings like `v0.4.0` in `tests/layout.rs` and `src/render/badge.rs` are badge example values, not the crate version, and stay put on a bump.

```
just publish        # check, then publish-crate, then gh-tag, stopping at the first failure
just publish-crate  # changelog gate, cargo publish --dry-run, cargo publish
just gh-tag         # git tag v<version>, refuse an existing tag, push main and the tag
```

Both release recipes require `main` and a clean tree.

---

## Code

- **Check crates.io before writing a module.** Prefer a maintained dependency and justify a hand-rolled one in the commit. The wrapper, the front matter fields and the badge parser are hand-rolled on purpose: they are small and the crates that exist pull in more than they give.
- **Avoid over-commenting.** No comments that narrate the next line, restate the diff or justify a change to a reviewer. Doc comments only where a name does not carry the meaning.
- **Keep section comments plain.** `// fakes`, not `// --- fakes ---------------`.
- **Standard Rust naming.** `snake_case` items, `CamelCase` types, names that need no decoding.
- **A new theme token goes in three places.** The `Theme` struct, `dark()`, and the list in `map_colors`, or it stays truecolor in the 256-color and mono tiers.
- **A new element updates `docs/rendering.md` and `CHANGELOG.md` in the same commit.**

## Git

- **Never commit unless explicitly asked. Always use the `/commit-msg` skill to commit and stick to its instructions.**
- **Work directly on `main`.** One maintainer and a linear history. Open a branch only when asked, named by convention: `feat/<topic>`, `fix/<topic>`, `docs/<topic>`, `build/<topic>`, or `release/<version>` for a version bump.
- **Keep [CHANGELOG.md](CHANGELOG.md) updated.** Notable changes land under `[Unreleased]` in the same commit as the change. State what changed, no overselling.
- **Do not restructure the `## [version] - date` headings.** `just publish-crate` parses that exact form.
- **Commit subjects follow `type: summary` as in the history**: `feat`, `fix`, `docs`, `build`, `test`, `refactor`, `chore`, with no scope.

## Docs

- **`docs/` holds the public docs only.** The Markdown pages are the site, the bundled `mido docs`, and a mido project of their own, so they carry front matter and nothing site-only. Planning, drafts and research go in `plan/`.
- **After adding a docs page, `touch build.rs`** so the bundle picks it up.
- **The README is the showcase.** `tests/fixtures/readme.md` is a frozen copy so the real one can change freely. Do not sync them.

## Writing

- **No em-dashes or semicolons in prose.** Documentation, comments, commit messages and the changelog. Use a full stop, a comma or a colon, or rewrite the sentence.
- Prose only. A semicolon in Rust or in a code sample is syntax.

## Gotchas

- **`cargo test` hides later suites behind the first failure.** Five test binaries run in sequence and cargo stops at the first red one. `--no-fail-fast` shows them all.
- **The docs test forbids a raw front matter dump, not the word.** It rejects a printed line that starts with `layout:`. The collapsed chip row prints `▸ [layout: layouts/page.njk]` under `NO_COLOR`, which is allowed, and that chip shows on every docs page because that is what their front matter says.
- **A link wrapping only a badge shows no URL.** Ordinary links append ` (url)`. A badge chip carries the link on the chip instead, and `]` then `Enter` still follows it.
- **Badges are not images.** `image_urls()` skips them so nothing tries to decode an SVG, and `badge_urls()` finds them anywhere in the document, headings and table cells included. Static shields badges read label, value and color from the URL and need no network.
- **`sole_image` decides what becomes a picture.** An image alone in a paragraph, whitespace aside, reserves rows and is painted. Anything else is a placeholder. Sizes come from a picker query at startup, so with no graphics protocol every image is a placeholder and the layout has no image slots.
- **Front matter values are parsed by hand.** `FrontMatter::fields()` handles scalars, quoted strings, inline and block lists, block scalars and one level of nesting as dotted keys for YAML, and tables and arrays for TOML. It is not a YAML parser, and anything stranger shows in the card under `m`.
- **`just demo` needs VHS 0.11.0.** 0.12.0 writes no output.
- **`.gitattributes` forces LF on every platform.** `docs/keys.md` is compared byte for byte with the keymap, and Windows checkouts used to fail on CRLF.
- **Terminal safety is ratatui's init and restore plus a chained panic hook.** `ratatui::try_init()` installs the hook that leaves raw mode and the alternate screen on a panic, `App::run` chains one that drops mouse capture first, and the `$EDITOR` shell-out restores and re-enters the same way. Keep that pairing when adding a mode.
- **The changelog regex in `just publish-crate` is not escaped.** The dots in the version match any character, which has never mattered.
