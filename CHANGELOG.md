# Changelog

All notable changes to mido are recorded here. The format follows [Keep a Changelog](https://keepachangelog.com), and versions follow semver. Before 1.0, a minor release may change keys or configuration.

## [0.4.1] - 2026-09-21

### Added

- Badges render as labels: an image from shields.io, badgen, docs.rs, GitHub Actions and friends becomes a two-tone chip with the badge name and its value, linked like the badge. Static shields badges read their text from the URL, and `--remote-images` fetches live values from the badge SVG, so a version badge shows the version.
- Front matter fields parse into key and value pairs for YAML and TOML, with lists joined by commas and nested tables flattened to dotted keys.

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
