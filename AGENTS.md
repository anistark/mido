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
- Not a full editor. `e` edits the Markdown source in place, for a typo or a note, with a few Markdown assists, and `E` shells out to `$EDITOR` for anything bigger. No modes, no plugins, no language server. A vim-modal backend can come later behind the `Editor` trait, not as switches in the textarea one.
- Not a converter. Print mode writes styled text for a pager, not HTML.
- No network by surprise. Nothing is downloaded unless `--remote-images` is given.
- Config is for taste, not structure. Since 0.5 a theme recolors, a glyph tier swaps symbols, and the config sets width, gutter, keys and extensions. None of them change how an element is laid out. Every color is a token in `crates/mido-core/src/render/theme.rs`, and network access stays a flag, never a setting.

---

## Architecture

One pipeline, two outputs. Every stage is a plain function and each can be tested alone.

A Cargo workspace of two crates. `crates/mido-core` (below as `core/`, meaning `crates/mido-core/src/`) holds parse, the document model, layout, themes and glyphs, and depends on ratatui-core for style and text types only, never on crossterm, ratatui or anything that touches a terminal or the network. The `mido` package at the root is the binary and the viewer, and re-exports the core as `mido::markdown` and `mido::render` so `crate::render::theme` paths work in both.

```
source text
  -> parse      pulldown-cmark events -> Document        core/markdown/parse.rs
  -> Document   blocks and inlines, every block keeps    core/markdown/document.rs
                its byte range in the source
  -> layout     Document + width + theme -> lines, plus  core/render/layout.rs
                headings, links, images and a source map
  -> paint      ratatui frame in the viewer              src/app/draw.rs
  -> print      ANSI text on stdout with -p              src/render/ansi.rs

source text, while editing
  -> editor     an Editor backend holds the lines        src/edit/
  -> source     pulldown-cmark offsets -> styled runs    core/render/source.rs
  -> paint      the backend draws, mido colors the cells src/edit/textarea.rs
  -> save       encode with the file's layout, atomic    src/edit/file.rs
  -> preview    Preview::SetContent after a 40 ms pause, src/app/preview.rs
                parse here, layout on a worker thread
```

In the split the preview is the same reader on the same `Layout`, fed by `Preview` commands, and the two panes line up through `Layout::sources` and the preview's `line_starts`.

`Document` is the important decision. The outline, search, link following, history, print mode and the snapshot tests all walk it, and a block never loses its source range, which is how a resize keeps the reading position.

### Module map

| Path | What it owns |
| --- | --- |
| `src/main.rs`, `src/cli.rs` | clap entry and mode dispatch: file, folder, stdin, print, `docs`, `themes`, `new`, `completions`, `--man`. Loads the config and resolves the theme before the TUI starts |
| `src/config.rs` | Config dir lookup, `config.toml` then the nearest `.mido.toml`, merging (strict for the user config, setting by setting with warnings for the project file), theme resolution by name or path with `extends`, background detection through terminal-light |
| `src/text.rs` | Reading a file as text: UTF-8, UTF-16 with a byte order mark both ways, lossy UTF-8 with a flag, NUL bytes refused as binary |
| `src/themes.rs` | `mido themes`: the list with swatches |
| `core/markdown/` | `parse.rs` builds the tree from pulldown-cmark events, `document.rs` is the data model plus front matter fields, `slug.rs` makes GitHub-compatible anchors |
| `core/render/layout.rs` | `Document` to styled lines at a width, with the heading, link, image and source indices |
| `core/render/wrap.rs` | Styled word wrap over `Piece`s that never splits a style run |
| `core/render/theme.rs`, `core/render/themes/` | The `tokens!` list, theme files in TOML with `schema = 1`, palette and `extends`, the twelve built-in themes, truecolor, 256-color and mono tiers, the syntect theme per mido theme |
| `core/render/glyphs.rs` | The `ascii`, `unicode` and `nerd` symbol tables, carried on `Theme` as `theme.glyphs` |
| `core/render/badge.rs` | Badge detection, static shields URL and SVG parsing, badge colors, the badge walker |
| `core/render/syntax.rs`, `core/render/mermaid/` | syntect wrapper, Mermaid blocks to text through mmdflux and mermaid-text, in ASCII under the ascii tier |
| `src/render/ansi.rs` | Print mode, in the binary because it writes crossterm escapes |
| `src/app/mod.rs` | App state and event loop: navigation, history, link selection, relayout. `Settings` carries what the config decides into `App::with_settings` |
| `src/app/draw.rs` | Panels, cards, status bar, overlays, image slots, the editor pane and its prompts |
| `src/app/preview.rs` | The `Preview` commands and their one handler, the debounced hand-off of the editor's text, the layout worker thread, the split's areas, and scroll sync both ways |
| `src/app/editing.rs` | The edit session: entering and leaving, keys to commands, find, save and the overwrite check, prompts, paste, the mouse, and the panic guard that writes a recovery file |
| `src/edit/mod.rs` | The `Editor` trait, `Command`, `Motion` and the `Look` an editor paints with |
| `src/edit/textarea.rs` | The ratatui-textarea backend: undo steps, painting over the widget, the screen map for the mouse and scroll |
| `src/edit/assist.rs` | Markdown assists as plain functions of lines and cursor: list and quote continuation, nesting, tasks, pairs |
| `src/edit/file.rs`, `clipboard.rs` | Decoding and encoding a file's layout and encoding, the atomic save, OSC 52 plus the local clipboard command off SSH |
| `core/render/source.rs` | Markdown source to styled runs per line for the editor, syntect on fenced code with a cache |
| `src/app/keys.rs` | `Action`, the default bindings, `Keymap::with_overrides` for `[keys]`, and the rows that drive the help overlay and `docs/keys.md` |
| `src/app/outline.rs`, `finder.rs`, `search.rs`, `watch.rs`, `images.rs` | Foldable tree for both panels, fuzzy file picker, smart-case search, file watcher, image and badge fetching with the cache |
| `src/app/grep.rs`, `positions.rs` | Search over the source of every file in the folder, and the reading position per file kept in the user cache folder |
| `src/project/scan.rs` | Folder walk with the ignore crate, tree order, titles, entry file |
| `src/docs.rs`, `build.rs` | Embed `docs/*.md` for `mido docs` and generate the man page |

### Hard rules

- **`Document` is data only.** Nothing about how a block looks lives in `core/markdown/`, glyphs included. A new element gets a variant there, a parse arm, and a draw method in `layout.rs`, in that order. `docs/contributing.md` walks through it.
- **The renderer never uses a raw color.** Colors are tokens on `Theme` with a style method beside them. Badge colors and syntect's colors are the content-driven exceptions and go through `Theme::label_colored` and `Theme::rgb`, which adapt them to the color tier. `just colors`, and CI, fail on a `Color::` anywhere outside `core/render/theme.rs`.
- **The renderer never hard-codes a symbol.** Bullets, bars, rules, borders, markers and connectors come from `theme.glyphs`, and every one has an ASCII form.
- **mido-core stays terminal-free.** No crossterm, ratatui, notify or ureq in `crates/mido-core`. If a module needs them it belongs in the binary.
- **Print mode and the viewer paint the same lines.** A change to `layout.rs` is done for both at once. Only chrome, pictures and overlays live in `draw.rs`.
- **One keymap.** `src/app/keys.rs` drives the view dispatch, panel and overlay movement, the status bar hints, the help overlay and `docs/keys.md`, and a test fails when the page drifts. Only `Esc`, `Tab`, `Ctrl-c` and the panel fold keys are fixed. The key table in the README is kept by hand. Editor actions (`Action::in_editor`) are looked up apart from the reader's with `Keymap::edit_action`, must not be a plain letter, and in the editor `Ctrl-c` copies instead of quitting. Movement and the Markdown keys in the editor are fixed rows in the help.
- **The preview is driven only through `Preview`.** Opening, replacing the content, scrolling to a source line and reloading go through `App::preview`. The built-in editor is its first client, and a control socket for other editors is meant to send the same commands, so there is no second rendering path.
- **The app talks to an editor only through `Editor` and `Command`.** Nothing outside `src/edit/textarea.rs` touches ratatui-textarea, and assists return commands instead of editing, so another backend gets them for free.
- **Saving never loses bytes.** An unedited buffer saves byte for byte, `tests/edit.rs` checks it over every Markdown file in the repo, and a save goes through a temporary file and a rename. The buffer is never replaced while editing: a change on disk is noted and the next save asks.
- **Nothing runs without asking.** A link goes to the system opener straight away only for `http`, `https` and `mailto`, the `SAFE_SCHEMES` in `src/app/mod.rs`. Any other scheme and any local file that is not Markdown waits on `Prompt::Open`, because the opener runs programs. Print mode drops control characters from text and link targets before writing them, so a document cannot send escape sequences. The viewer gets the same from ratatui, which skips any grapheme with a control character.
- **Nothing reaches the network without `--remote-images`.** It is a flag only, never a config setting, because a `.mido.toml` arrives with any folder a user opens. Local images always load. Remote images and badge values are fetched only with the flag: images cached under the user cache directory with a 10 MB cap, badges with a 5 s timeout and a 64 KB cap and no disk cache.

---

## Build and test

```
just check        # just colors, cargo fmt --check, clippy and tests on the workspace: what CI runs
just run file.md  # cargo run with a file, README.md by default
just keys         # regenerate docs/keys.md and the reference half of docs/themes.md
just gallery      # draw every built-in theme to SVG in target/gallery
just docs         # the docs site on pnpm and Node 24, at localhost:8080
just demo         # re-record the landing page video with VHS 0.11.0
```

`just check` is the one to run before handing work back. Rust edition 2024, minimum 1.93, and CI runs an MSRV job with `cargo check --all-targets` on that version, so let chains and `Option::is_none_or` are fine and anything newer is not. Linux, macOS and Windows all run the tests.

Snapshots are the main test style, through insta, and the rendered text is the contract:

- Layout snapshots in `tests/layout.rs` render a fixture from `tests/fixtures/` at 60 columns and snapshot the plain lines.
- Screen snapshots in `tests/app.rs` drive the whole app through ratatui's `TestBackend` at 80, 120 or 140 columns.
- Diagram snapshots in `core/render/mermaid/snapshots/` cover each Mermaid type, named `mido_core__...` after the crate.
- Editor screen tests in `tests/editor.rs` drive the app on a file in a temporary folder through key presses, then read the file back.
- `tests/edit.rs` saves every Markdown file in the repo unedited and compares the bytes, and checks undo and redo with proptest. A failing case lands in `tests/edit.proptest-regressions`, which is committed so it is replayed.
- `tests/gallery.rs` draws `tests/fixtures/gallery.md` through the app in every built-in theme and checks each one paints its own colors. With `MIDO_GALLERY=<dir>` it writes an SVG per theme and an `index.html`, which the docs workflow publishes at `/gallery/`.

cargo-insta is not installed. A changed snapshot fails the test and writes a `.snap.new` beside it. Run `INSTA_UPDATE=always cargo test`, read `git diff tests/snapshots`, and commit the `.snap`, never the `.snap.new`. A bare `cargo test` stops at the first failing test binary, so add `--no-fail-fast` to see every suite.

`tests/docs.rs` runs the built binary over every page in `docs/` in print mode with `NO_COLOR` and rejects template syntax, raw HTML, a missing title and a leaked front matter line. It also checks that every CLI flag is on the command line page, that `docs/keys.md` matches the keymap, that everything from `## Built-in themes` down in `docs/themes.md` matches `theme::markdown()`, and that the bundled docs match the folder.

Internal planning notes go in `plan/`, which is gitignored along with `.claude/`.

---

## Rendering

The design is part of the product. `docs/rendering.md` says what each element looks like and is the reference for a user, and `src/render/theme.rs` says what color it takes.

- Every element has a treatment: H1 is a filled title chip, H2 a text-width underline, inline code sits padded on the code background, links show their URL dimmed, front matter and badges are two-tone label chips, an image alone in a paragraph becomes a picture and an inline one becomes `■ alt (path)`.
- Wrapping is in-house. `wrap()` takes `Piece`s: text pieces break at spaces, atomic pieces never break, and adjacent atomic pieces form one unbreakable word, which is how a two-part chip stays together. Trailing spaces are trimmed unless they carry a background.
- Layout owns the measure. Full terminal width by default, `--width` caps and centers, two columns of gutter each side from 60 wide. Exactly one blank line between blocks, two before H1 and H2.
- Color tiers: truecolor when `COLORTERM` says so, else 256, else 16. `NO_COLOR` or `--no-color` gives mono, and every element has a text-only form there: backticks for code, a heavy rule under H1, `[key: value]` for chips.
- Anything mido cannot render, raw HTML, math, a diagram type the crates miss, is shown as source, never dropped.

---

## Release

Versions follow SemVer with Keep a Changelog. Before 1.0 a minor bump may change keys or configuration.

The version lives in the root `Cargo.toml` only, in `[workspace.package]`, which both crates inherit, and in the `mido-core` dependency line beside it. Bump both together: a stale dependency line fails to resolve. `cargo update --workspace` moves the `Cargo.lock` entries, and the docs site reads the first `version =` line of `Cargo.toml` at build time. `CHANGELOG.md` needs a dated `## [x.y.z] - YYYY-MM-DD` section for the version before it can be published. Strings like `v0.4.0` in `tests/layout.rs` and `core/render/badge.rs` are badge example values, not the crate version, and stay put on a bump.

```
just publish        # check, then publish-crate, then gh-tag, stopping at the first failure
just publish-crate  # changelog gate, then cargo publish --workspace, dry run first: mido-core, then mido
just gh-tag         # git tag v<version>, refuse an existing tag, push main and the tag
```

Both release recipes require `main` and a clean tree.

---

## Code

- **Check crates.io before writing a module.** Prefer a maintained dependency and justify a hand-rolled one in the commit. The wrapper, the front matter fields and the badge parser are hand-rolled on purpose: they are small and the crates that exist pull in more than they give.
- **Avoid over-commenting.** No comments that narrate the next line, restate the diff or justify a change to a reviewer. Doc comments only where a name does not carry the meaning.
- **Keep section comments plain.** `// fakes`, not `// --- fakes ---------------`.
- **Standard Rust naming.** `snake_case` items, `CamelCase` types, names that need no decoding.
- **A new theme token goes in two places.** A line with its description in the `tokens!` list in `core/render/theme.rs`, which makes the field, the name, the tier mapping and the docs row, and a value in every file under `core/render/themes/`. `every_builtin_theme_defines_every_token` fails until each theme has it, then run `just keys`.
- **A new glyph goes in all three tiers** of `core/render/glyphs.rs`, with a plain ASCII form, and gets checked against the fonts table in `docs/themes.md`.
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

- **`cargo test` hides later suites behind the first failure.** The test binaries run in sequence and cargo stops at the first red one. `--no-fail-fast` shows them all.
- **The docs test forbids a raw front matter dump, not the word.** It rejects a printed line that starts with `layout:`. The collapsed chip row prints `▸ [layout: layouts/page.njk]` under `NO_COLOR`, which is allowed, and that chip shows on every docs page because that is what their front matter says.
- **A link wrapping only a badge shows no URL.** Ordinary links append ` (url)`. A badge chip carries the link on the chip instead, and `]` then `Enter` still follows it.
- **Badges are not images.** `image_urls()` skips them so nothing tries to decode an SVG, and `badge_urls()` finds them anywhere in the document, headings and table cells included. Static shields badges read label, value and color from the URL and need no network.
- **`sole_image` decides what becomes a picture.** An image alone in a paragraph, whitespace aside, reserves rows and is painted. Anything else is a placeholder. Sizes come from a picker query at startup, so with no graphics protocol every image is a placeholder and the layout has no image slots.
- **Front matter values are parsed by hand.** `FrontMatter::fields()` handles scalars, quoted strings, inline and block lists, block scalars and one level of nesting as dotted keys for YAML, and tables and arrays for TOML. It is not a YAML parser, and anything stranger shows in the card under `m`.
- **`just demo` needs VHS 0.11.0.** 0.12.0 writes no output.
- **`.gitattributes` forces LF on every platform.** `docs/keys.md` is compared byte for byte with the keymap, and Windows checkouts used to fail on CRLF.
- **Terminal safety is ratatui's init and restore plus a chained panic hook.** `ratatui::try_init()` installs the hook that leaves raw mode and the alternate screen on a panic, `App::run` chains one that drops mouse capture first, and the `$EDITOR` shell-out restores and re-enters the same way. Keep that pairing when adding a mode.
- **A `.mido.toml` never stops mido.** It comes with any folder, maybe written for a newer version, so `Config::merge_project` merges it one setting and one `[keys]` entry at a time and turns each failure into a line in `Config::warnings`. A theme it names that does not resolve falls back to the user's through `without_project_themes`. The user config stays strict on purpose: a typo there should be loud.
- **Tests must not touch the reading positions.** `App` only remembers positions after `remember_positions`, which `main` calls with the cache path. Screen tests that need it pass a `Positions` loaded from a temporary folder.
- **Tests must not read the developer's config.** The binary tests in `tests/docs.rs` run mido for real, so a personal `config.toml` could change what they print. Set `MIDO_CONFIG_DIR` to an empty folder when a test spawns the binary, and build `App` through `Settings::default()` in screen tests.
- **Background detection writes to the terminal.** `config::detect_background` sends an OSC 11 query and reads the answer in raw mode, so it runs once in `main` before `ratatui::try_init`, and only when stdin and stdout are both a tty. Never call it from inside the TUI.
- **`mido themes`, `mido docs`, `mido new` and `mido completions` are names, not subcommands.** A local path with that name wins over them. `new` and `completions` take their argument from the second positional, `Cli::file`.
- **The changelog regex in `just publish-crate` is not escaped.** The dots in the version match any character, which has never mattered.
- **syntect's Markdown grammar is far too slow to color the editor.** It costs about half a millisecond a line under fancy-regex, so a screen of source would miss a frame. `core/render/source.rs` styles the source from pulldown-cmark's offsets instead, which parses 5k lines in under a millisecond, and keeps syntect for fenced code, cached by language and body.
- **The editor draws twice over.** The textarea widget lays out glyphs, wraps and scrolls with every style of its own set blank, then `TextAreaEditor::render` walks the painted rows through `screen_to_data` and sets each cell's style: theme, syntax runs, cursor line, selection, find hits. The real terminal cursor is placed with `Frame::set_cursor_position`, so screen snapshots do not show it, and tests read it from the backend.
- **Undo steps are text hashes.** ratatui-textarea undoes one edit at a time, so a step records the hash of the lines before and after, and undo replays the textarea's history until the hash matches. A run of typing or erasing folds into one step, an assist runs inside `Editor::group`, and any edit, even one that changes nothing, clears the redo stack because it cuts the textarea's redo branch.
- **ratatui-textarea copies on some deletes.** `delete_str` and the word deletes write the deleted text to its yank buffer, and `delete_str` past the end of the text leaves a one-line chunk that panics a later paste in debug builds. The backend restores the yank around them and skips a paste with nothing to paste.
- **Typing latency is a budget.** A keystroke on a 5k-line file takes about 9 ms in a release build, 6.5 ms of it the textarea rebuilding its wrap map. A preview refresh adds about 2 ms of parsing on the main thread and 1 ms to install the worker's layout. Measure before adding per-keystroke work, and keep anything whole-document behind the editor's version counter.
- **The source map is finer than blocks.** A soft or hard break carries the offset of the source line it starts, the wrapper hands it on as a zero-width `Piece::Source`, and the wrapped line that holds the next word records it. Code lines carry their own offsets. Lines without one inherit the last, so `Layout::sources` only grows within a block, but footnotes at the end point back to where they were defined.
- **The live preview lays out off the main thread.** `App::receive_layout` installs the worker's result only if nothing newer replaced it, and any direct change through `set_text` drops the pending one. Tests call `App::flush_preview`, which lays out in place, because they have no event loop to wait in.
- **Where `Esc` goes in the editor depends on the last frame.** With the preview on screen it moves to the preview, otherwise it leaves the editor. Screen tests must draw after `v` before pressing keys, as the event loop does.
