# Changelog

All notable changes to mido are recorded here. The format follows [Keep a Changelog](https://keepachangelog.com), and versions follow semver. Before 1.0, a minor release may change keys or configuration.

## [0.2.0] - Unreleased

### Added

- Project mode: `mido <folder>` and bare `mido` open a folder, honouring `.gitignore` and skipping hidden files, starting on README.md, then index.md, then the first file.
- Files sidebar: a flat panel on the left headed by the opened folder's name, the tree hanging from it, folders first, foldable, the open file highlighted, and `T` to show first headings instead of names.
- Link following: `]` and `[` select links, including links inside table cells, and Enter or a click follows them. Relative links open in place, `#anchors` jump to headings using GitHub slugs, folders open their README, and web links open in the browser.
- History: `H` and `L` move back and forward through visited pages and positions.
- Fuzzy file finder on `Ctrl-p`, matching paths and first headings.
- Focus mode on `f`: hides both panels for a full-width document and restores them on a second press.
- `E` opens the file in `$VISUAL` or `$EDITOR` and reloads it afterwards.
- In a folder the whole tree is watched, so added and removed files appear in the files panel with folds preserved.
- Panels show only while the document keeps 60 columns, so narrow terminals stay readable.
- Mermaid code blocks render as text diagrams: flowcharts, sequence, class and state diagrams through mmdflux, and pie, gantt, mindmap, timeline, git graphs and other types through mermaid-text. Unsupported or too-wide diagrams keep their labelled source.

### Changed

- `b` toggles the files panel and `o` the outline panel.
- `h` opens the help overlay alongside `?`. Focus moves left with `Shift-Tab` or `Tab` then `←`, and folding in a panel uses `Space` and the arrow keys.
- Tab cycles through files, document and outline in that order.

## [0.1.0] - Unreleased

### Added

- Single-file viewer: `mido file.md`, and `mido -` for stdin.
- Heading design per level: a filled title chip for H1, a text-width underline for H2, faint `#` markers for H3 to H6.
- Element polish: list markers colored by depth, italic quotes, a header band on tables, superscript footnote markers.
- Viewer chrome: status bar with a file chip, hint titles and a selection marker on overlays, an overflow marker for lines wider than the screen, and an empty-file notice.
- Rendering for headings, paragraphs, emphasis, links, image placeholders, footnotes, nested and ordered lists, task lists, blockquotes, fenced and indented code with syntax highlighting, GFM tables, rules and raw HTML.
- Full-width layout with gutters that shrink on narrow terminals. `--width` caps the measure and centers it.
- Vim-style scrolling, mouse wheel, smart-case search, table of contents and help overlays.
- Outline panel: the headings as a foldable tree in a rounded, tinted card at the top right, with the text wrapping before it. It follows the reading position and is navigable with the keyboard and the mouse, shown automatically on wide terminals. `Tab` and `Shift-Tab` cycle focus between panes, and an arrow right after Tab picks the direction.
- Live reload when the file changes on disk.
- Print mode: `-p` writes styled text to stdout. Piping without it writes plain text.
- Truecolor with automatic 256-color fallback, and `NO_COLOR` support.
