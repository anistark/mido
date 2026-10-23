# Changelog

All notable changes to mido are recorded here. The format follows [Keep a Changelog](https://keepachangelog.com), and versions follow semver. Before 1.0, a minor release may change keys or configuration.

## [Unreleased]

### Added

- `mido completions <shell>` prints a completion script for bash, zsh, fish, elvish or PowerShell.
- `--no-color` draws without color, the same as setting `NO_COLOR`.
- `mido-high-contrast`, a built-in theme for low vision: white on black, with every color that carries text at least 7:1 against what it sits on (WCAG AAA), code highlighting included.
- `F` searches every Markdown file in the folder, listing each matching line as `path:line`. `Enter` opens the file at that line with the query kept for `n` and `N`.
- `y` copies the selected link's target, or a code block on the screen, pressing it again for the next block in view. Code is copied without its trailing newline.
- A file opens again where you left it, kept for the last 500 files in the user cache folder. `remember_position = false` turns it off.
- UTF-16 files with a byte order mark are read and saved as UTF-16.
- mido-core: `Layout::code` lists each code block with the lines it takes and its source.

### Changed

- Following a link to a file that is not Markdown, or to a URL other than `http`, `https` or `mailto`, asks before the system opens it. The system opener runs programs, so a link to `setup.command` or `install.bat` used to run it with one `Enter`.
- A `.mido.toml` that sets something this version cannot use, an unknown setting, a bad value, an unknown key action or a missing theme, skips that setting with a warning in the status bar, or on stderr in print mode, instead of stopping mido. The user config stays strict.
- A file that is not valid UTF-8 opens with the bad bytes shown as replacement characters and a notice, instead of failing. The editor still refuses it, so no bytes are lost. A file with NUL bytes is refused as binary.
- Copying, in the editor or with `y`, also uses `pbcopy`, `wl-copy`, `xclip` or `xsel` outside SSH, for terminals that ignore OSC 52 such as Apple Terminal.

### Fixed

- A byte order mark at the start of a file no longer hides its first heading or its front matter.

### Security

- Print mode drops control characters from the text and from link targets, so a document can no longer send its own escape sequences to the terminal, such as one that sets the clipboard through OSC 52 or retitles the window. The viewer already dropped them.
- Remote images are fetched with `webpki-roots` 1.0.10 and `rustls-webpki` 0.103.16, which stop trusting the eight compromised eMudhra intermediate CAs. Older builds would accept a certificate forged with those keys.

## [0.6.0] - 2026-10-13

### Added

- `--help` and `--version` open with a pixel-font `mido` banner with the version beside it.
- A built-in editor. `e` swaps the page for its Markdown source at the part you were reading, colored with the theme, and `Esc` swaps back, asking first about unsaved changes. Undo and redo, selection with `Shift` or the mouse, cut, copy and paste, word and paragraph moves, and find in the file with `Ctrl-f`.
- Markdown help in the editor: `Enter` continues a list, task or quote and ends an empty one, `Tab` and `Shift-Tab` nest a list item, `Ctrl-t` checks a task or makes the line one, and backticks and asterisks are typed in pairs or wrap the selection.
- `Ctrl-s` saves through a temporary file and a rename, keeping line endings, a byte order mark and the trailing newline as they were. A file changed on disk since it was opened is not overwritten without asking.
- `mido new <file>`, and opening a Markdown path that does not exist, start the file in the editor and create it on the first save.
- Copying in the editor also puts the text on the system clipboard through OSC 52, and a terminal paste arrives as one paste.
- A panic while editing writes unsaved changes to a hidden `.<name>.mido-recover` file beside the original and prints its path.
- Editor keys in `[keys]`: `save`, `undo`, `redo`, `cut`, `copy`, `paste`, `select_all`, `find`, `find_next`, `find_prev` and `toggle_task`, looked up apart from the reader's keys, and `editor` for `e`.
- A live preview beside the editor. `v` opens the editor with the rendered page next to it, and `Ctrl-e` in the editor shows or hides it. The preview takes the text when typing pauses and lays it out on a background thread, so a long document never holds up a keystroke.
- The editor and the preview scroll together through the source map, whichever was scrolled last leading, down to a line inside a long paragraph or a code block. `s` turns sync off and on, `<` and `>` resize the split, and a click in the preview puts the cursor on that line.
- `split = "side-by-side"` or `"stacked"` and `split_ratio` in the config. On a narrow terminal the files panel gives way to the split first.
- Quitting with `Ctrl-c` while the editor has unsaved changes asks first.
- Print mode writes links as OSC 8 hyperlinks, so terminals that support them make the text clickable. Relative links to files that exist become `file://` URLs.
- The status bar names where the selected link goes.
- `docs/rendering.md` records the image protocol mido picks in the terminals checked so far: Kitty in Ghostty, half blocks in Apple Terminal.

### Changed

- Task marks are `○` and `✓`, the image marker is `■` and the TIP icon is `◊` in the `unicode` tier, so JetBrains Mono, Monaspace and Cascadia Code draw every glyph without a fallback font. The fonts table now covers Fira Code, Monaspace and Cascadia Code.
- mido-core: `Inline::SoftBreak` and `Inline::HardBreak` carry the byte offset of the source line they start, `BlockKind::CodeBlock` has the offset of each code line, and wrapped lines carry a source offset through `Piece::Source`, so `Layout::sources` maps lines inside a paragraph or a code block.
- mido-core: syntax highlighting results are cached by language, code and theme, so laying a document out again does not highlight unchanged code blocks twice.

## [0.5.1] - 2026-10-01

### Changed

- Table body rows are separated by a blank row inside the borders, so a cell that wraps no longer runs into the next row.

### Fixed

- Clicking a file in the files sidebar or a heading in the outline hands the keys back to the document, as `Enter` does. Before, focus stayed on the panel, so the arrow keys moved the panel selection instead of scrolling, and after an outline click `Down` jumped to the next heading from the clicked one rather than scrolling a line.

## [0.5.0] - 2026-09-25

### Added

- Themes: eleven built in, `mido-dark`, `mido-light`, `mido-reading` (a warm, low-contrast paper palette), Catppuccin Mocha and Latte, Gruvbox, Nord, Tokyo Night, Dracula, and Solarized dark and light. Pick one with `--theme <name>` or `theme` in the config. The themes that come from an editor palette paint their own background.
- Theme files in TOML with `schema = 1`: a `[palette]` of named colors, `[colors]` for every token, and `extends` to change a few tokens of another theme. User themes go in `~/.config/mido/themes/`, a path works too, and a theme missing a token is refused with the token named.
- `mido themes` lists every theme with its kind and a swatch of its colors.
- Light or dark detection: with the default `theme = "auto"`, mido asks the terminal for its background once at startup, waits at most 100 ms, falls back to `COLORFGBG`, then to dark, and picks `mido-light` or `mido-dark`. `background`, `dark_theme` and `light_theme` in the config override it.
- Code blocks take their syntax colors from a syntect theme chosen by the mido theme.
- Config at `~/.config/mido/config.toml` (`%APPDATA%\mido\config` on Windows, `MIDO_CONFIG_DIR` or `--config` to point elsewhere), and a `.mido.toml` in the opened folder or above it that overrides it. Settings: `theme`, `dark_theme`, `light_theme`, `background`, `glyphs`, `width`, `gutter`, `front_matter`, `extensions` and `[keys]`. An unknown setting is an error, not ignored.
- Glyph tiers: `glyphs = "ascii"` draws everything in plain ASCII, Mermaid diagrams included, and `glyphs = "nerd"` adds Nerd Font icons to the files panel, images and alerts.
- Key remapping in `[keys]`, by action name. The help overlay, the status bar hints and `docs/keys.md` show the keys actually bound.
- `front_matter = "hidden"` starts with the front matter out of sight, and `m` still shows it.
- `extensions` sets which files count as Markdown, replacing the fixed list.
- A theme gallery: the docs site shows the same page drawn in every built-in theme, generated in CI from the screen buffer the tests use.
- A fonts table in the docs recording which glyphs Menlo, MesloLGS NF, JetBrains Mono, SF Mono, Source Code Pro and older macOS fonts cover.

### Changed

- The parse, layout and theme code moved into a `mido-core` crate with no terminal dependency, published alongside `mido`, so other tools can embed the renderer.
- The help overlay lists both keys of a pair, so the panel row reads `j / k, ↓ / ↑ in a panel`, and `?` is listed before `h`.
- CI fails on a raw color outside the theme module.
- A table too wide for the screen keeps its short columns, 20 cells or less, on one line and shares the rest of the width in proportion to how much each column has to say, so a prose column no longer shrinks to the size of a label column.

## [0.4.1] - 2026-09-21

### Added

- Badges render as labels: an image from shields.io, badgen, docs.rs, GitHub Actions and friends becomes a two-tone chip with the badge name and its value, linked like the badge. Static shields badges read their text from the URL, and `--remote-images` fetches live values from the badge SVG, so a version badge shows the version. The value side takes the badge's color, with dark or light text picked for contrast.
- Front matter fields parse into key and value pairs for YAML and TOML, with lists joined by commas and nested tables flattened to dotted keys.
- A test walks every relative link and anchor in the docs, README, CONTRIBUTING and CHANGELOG and fails on a broken one.

### Changed

- The collapsed front matter line shows one label chip per field, key on the left and value on the right, instead of listing the keys. `m` still expands the card.

## [0.4.0] - 2026-09-20

### Added

- Single-file viewer: `mido file.md`, and `mido -` for stdin.
- Project mode: `mido <folder>` and bare `mido` open a folder, honouring `.gitignore` and skipping hidden files, starting on README.md, then index.md, then the first file.
- Rendering for headings, paragraphs, emphasis, links, footnotes, nested and ordered lists, task lists, blockquotes, fenced and indented code with syntax highlighting, GFM tables, rules and raw HTML.
- Heading design per level: a filled title chip for H1, a text-width underline for H2, faint `#` markers for H3 to H6.
- Element polish: list markers colored by depth, italic quotes, a header band on tables, superscript footnote markers.
- Images: `![alt](path)` on its own line is drawn through Kitty, iTerm2 or Sixel, with half-block characters everywhere else. Local files always load, remote ones with `--remote-images`, cached in the user cache directory with a 10 MB cap. `i` hides and shows them, the alt text becomes a caption, and print mode keeps the placeholder.
- GitHub alerts: `[!NOTE]`, `[!TIP]`, `[!IMPORTANT]`, `[!WARNING]` and `[!CAUTION]` blockquotes draw a colored bar, an icon and a label.
- Front matter in YAML or TOML shows as one collapsed line listing its keys. `m` expands it into a highlighted card.
- Emoji shortcodes such as `:tada:` render as emoji.
- Math: `$..$` shows as styled source, and a `$$` block on its own renders as a fenced block.
- Definition lists render with bold terms and indented definitions.
- Mermaid code blocks render as text diagrams: flowcharts, sequence, class and state diagrams through mmdflux, and pie, gantt, mindmap, timeline, git graphs and other types through mermaid-text. Unsupported or too-wide diagrams keep their labelled source.
- Full-width layout with gutters that shrink on narrow terminals. `--width` caps the measure and centers it.
- Viewer chrome: status bar with a file chip, hint titles and a selection marker on overlays, an overflow marker for lines wider than the screen, and an empty-file notice.
- Vim-style scrolling, mouse wheel, smart-case search and a table of contents overlay.
- Help overlay on `?` or `h`, grouping keys by task. `docs/keys.md` is generated from the same table, and a test fails when the two drift.
- Outline panel on `o`: the headings as a foldable tree in a rounded, tinted card at the top right, with the text wrapping before it. It follows the reading position and is navigable with the keyboard and the mouse, shown automatically on wide terminals.
- Files sidebar on `b`: a flat panel on the left headed by the opened folder's name, the tree hanging from it, folders first, foldable, the open file highlighted, and `T` to show first headings instead of names.
- `Tab` cycles focus through files, document and outline in that order, `Shift-Tab` or `Tab` then `←` moves it left, and folding in a panel uses `Space` and the arrow keys.
- Focus mode on `f`: hides both panels for a full-width document and restores them on a second press.
- Panels show only while the document keeps 60 columns, so narrow terminals stay readable.
- Link following: `]` and `[` select links, wikilinks and footnote references, including links inside table cells, and Enter or a click follows them. Relative links open in place, `#anchors` jump to headings using GitHub slugs, folders open their README, and web links open in the browser.
- Wikilinks: `[[Page]]` and `[[page#section|label]]` open the file in the folder whose name or first heading matches, or `page.md` next to the current file.
- Footnotes: select a reference with `]` or `[` and press Enter to read the note in a popup.
- History: `H` and `L` move back and forward through visited pages and positions.
- Fuzzy file finder on `Ctrl-p`, matching paths and first headings. The finder and the status bar show project paths with forward slashes on every platform, so `guide/faq.md` reads the same everywhere.
- `E` opens the file in `$VISUAL` or `$EDITOR` and reloads it afterwards.
- Live reload when the file changes on disk. In a folder the whole tree is watched, so added and removed files appear in the files panel with folds preserved.
- Print mode: `-p` writes styled text to stdout. Piping without it writes plain text.
- Truecolor with automatic 256-color fallback, and `NO_COLOR` support.
- Documentation site at https://anistark.github.io/mido, built with Eleventy from the Markdown pages in `docs/` and deployed by CI. The pages carry only front matter on top of plain Markdown, so `mido docs/` reads them too.
- `mido docs` opens the bundled documentation offline, extracted from the binary into a temporary folder, whenever nothing named `docs` exists in the current directory.
- `--man` prints the man page, generated at build time with clap_mangen.
- Contributor docs: the pipeline, adding a block type, writing snapshot tests.
- CI runs fmt, clippy and the tests on Linux, macOS and Windows, and renders every docs page in print mode.
- The landing page demo is recorded by VHS from `docs/tapes/landing.tape` on every deploy.
- MIT license.
