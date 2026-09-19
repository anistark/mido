---
layout: layouts/page.njk
title: Roadmap
description: The release sequence for mido, from reader to project browser to editor to Neovim preview.
tags: docs
order: 8
---

# Roadmap

mido grows in small releases, each with one theme. Before 1.0 a minor release may change keys or configuration, and every change is recorded in the [changelog](../CHANGELOG.md).

| Version | Theme | Headline |
| --- | --- | --- |
| 0.1 | Reader | `mido file.md` is the nicest way to read one Markdown file in a terminal |
| 0.2 | Project | `mido dir/` browses a whole tree with a sidebar and follows links |
| 0.3 | Docs | mido documents itself: bundled docs, this site, a man page |
| 0.4 | Themes, fonts and config | Dark, light and reading themes, all editable from one TOML file |
| 0.5 | Editor | Fix a typo or write a note without leaving mido |
| 0.6 | Split view | Editor and live preview side by side, scroll-synced |
| 0.7 | Rich content | Images, alerts, front matter, wikilinks, emoji, math fallbacks |
| 1.0 | Stable | Frozen config and theme schema, packages everywhere |
| 2.0 | Neovim | mido.nvim previews the unsaved buffer with cursor sync over a control socket |

## Principles

1. Looks like a document, not like `cat` with colors. Hierarchy, whitespace and a fixed measure come first.
2. Keyboard first, vim-flavored. Mouse works too.
3. Instant. A 5k-line file opens with no visible delay.
4. Zero config to start. Sensible defaults, everything overridable later from one file.
5. Works wherever a terminal does: SSH, tmux, 16-color fallback, no special font required.
6. Degrade honestly. Anything mido cannot render is shown as source, never silently dropped.

## Ideas after 1.0

Tabs and bookmarks, a presentation mode that splits slides on headings, export to HTML and PDF through the same renderer, opening URLs directly, git awareness in the sidebar, and a Markdown lint for broken links and orphan files.
