---
layout: layouts/page.njk
title: Panels
description: The files sidebar, the outline card, and how focus moves between the panes.
tags: docs
order: 4
---

# Panels

Two panels sit beside the document: a files sidebar on the left in folder mode, and an outline of the headings at the top right. Both are trees, fold the same way and share the same keys.

## Files sidebar

Open a folder and its Markdown files appear in a sidebar headed by the folder's name, with the tree hanging from it. Folders come first, the open file is highlighted, and connectors show the nesting.

- Files are found with the same rules as ripgrep: `.gitignore` is honoured, even outside a git repository, and hidden files are skipped.
- The extensions counted as Markdown are `md`, `markdown`, `mdown`, `mkd` and `mdx`.
- mido starts on README.md, then index.md, then the first file in the tree.
- `T` shows each file's first heading instead of its name.
- The whole tree is watched. Files that appear or disappear on disk show up in the panel with the folds preserved.

`b` toggles the sidebar. It takes a quarter of the terminal width, between 22 and 32 columns.

## Outline

The headings of the current document sit in a rounded, tinted card at the top right. The text wraps before it rather than running underneath. The card is as wide as the longest heading, at most 32 columns and a third of the screen, and as tall as its visible rows.

The section that contains the top of the viewport is highlighted, and if that heading is inside a folded section the folded ancestor is highlighted instead. `o` toggles the card.

## Folding

In either panel:

| Key | Action |
| --- | --- |
| `j` / `k` | Move up and down |
| `Enter` | Open the file or jump to the heading, and return to the document |
| `Space` | Fold or unfold the row |
| `←` | Collapse, or move to the parent |
| `→` | Expand |
| `-` / `=` | Fold everything / unfold everything |

Long entries wrap onto continuation lines instead of being clipped, with the tree guides carried down through them.

## Focus

`Tab` cycles focus through the files sidebar, the document and the outline, in that order, and wraps around. `Shift-Tab` goes the other way. An arrow key pressed right after `Tab` moves in that direction instead. `l` moves to the pane on the right.

The focused panel shows its border or rule in the accent color. `Enter` or `Tab` from a panel brings focus back to the document.

## Focus mode

`f` hides both panels and gives the document the full width. Press it again to restore them. `b` or `o` also leave focus mode and show that one panel.

## Narrow terminals

Panels appear only while the document keeps at least 60 columns. On a medium-width terminal the files sidebar wins over the outline. Forcing a panel on with `b` or `o` lowers the limit to 40 columns. Below that the document has the screen to itself.
