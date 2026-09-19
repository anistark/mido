---
layout: layouts/page.njk
title: Keys
description: Every key in mido, grouped by task. Vim-flavored, with mouse support.
tags: docs
order: 3
kbd: true
---

# Keys

mido is keyboard first and vim-flavored. The mouse works too: the wheel scrolls, and a click follows a link or selects a row in a panel. Press `?` or `h` inside mido for the same list.

## Reading

| Key | Action |
| --- | --- |
| `j` / `k`, `↓` / `↑`, wheel | Scroll one line |
| `Ctrl-d` / `Ctrl-u` | Half page down / up |
| `Ctrl-f` / `Ctrl-b`, `Space` | Page down / up |
| `g` / `G` | Top / bottom |
| `t` | Table of contents |
| `r` | Reload the file |
| `h`, `?` | Help |
| `q` | Quit |

## Panels and focus

| Key | Action |
| --- | --- |
| `b` / `o` | Toggle the files / outline panel |
| `f` | Focus mode: hide the panels, full width |
| `Tab` / `Shift-Tab` | Cycle focus between the panes |
| `Tab` then `←` / `→` | Move focus in that direction |
| `l` | Focus the pane to the right |
| `j` / `k` in a panel | Move, `Enter` opens or returns |
| `Space`, `←` / `→` in a panel | Fold, collapse / expand a folder or section |
| `-` / `=` in a panel | Fold all / unfold all |
| `T` in the files panel | Show titles instead of file names |

## Links and history

| Key | Action |
| --- | --- |
| `]` / `[` | Select the next / previous link |
| `Enter`, click | Follow the selected link |
| `H` / `L` | Back / forward through visited pages |
| `Ctrl-p` | Find a file in the folder |

## Search and editing

| Key | Action |
| --- | --- |
| `/` | Search. `Enter` keeps the query, `Esc` cancels |
| `n` / `N` | Next / previous match |
| `E` | Edit the file in `$EDITOR`, reload on return |

Search is smart-case: a lowercase query matches any case, and a query with a capital letter matches exactly.

Before 1.0 a minor release may change keys. Every change is listed in the [changelog](../CHANGELOG.md).
