# Changelog

All notable changes to mido are recorded here. The format follows [Keep a Changelog](https://keepachangelog.com), and versions follow semver. Before 1.0, a minor release may change keys or configuration.

## [Unreleased]

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
