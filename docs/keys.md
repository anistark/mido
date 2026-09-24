---
layout: layouts/page.njk
title: Keys
description: Every key in mido, grouped by task. Vim-flavored, with mouse support.
tags: docs
order: 3
kbd: true
---

# Keys

mido is keyboard first and vim-flavored. The mouse works too: the wheel scrolls, and a click follows a link or selects a row in a panel. Press `?` or `h` inside mido for the same list, grouped the same way.

Search is smart-case: a lowercase query matches any case, and a query with a capital letter matches exactly. Before 1.0 a minor release may change keys, and every change is listed in the [changelog](../CHANGELOG.md).

The tables below are generated from the keymap in the source, so they cannot drift from what the binary does.

## Reading

| Key | Action |
| --- | --- |
| `j` / `k`, `↓` / `↑`, wheel | Scroll one line |
| `Ctrl-d` / `Ctrl-u` | Half page down / up |
| `Ctrl-f` / `Ctrl-b`, `Space` | Page down / up |
| `g` / `G` | Top / bottom |
| `t` | Table of contents |
| `i` | Show or hide images |
| `m` | Expand or collapse the front matter |
| `r` | Reload the file |

## Panels and focus

| Key | Action |
| --- | --- |
| `b` / `o` | Toggle the files / outline panel |
| `f` | Focus mode: hide the panels, full width |
| `Tab` / `Shift-Tab` | Cycle focus between the panes |
| `Tab` then `←` / `→` | Move focus in that direction |
| `l` | Focus the pane to the right |
| `j` / `k`, `↓` / `↑` in a panel | Move, Enter opens or returns |
| `Space`, `←` / `→` in a panel | Fold, collapse / expand a folder or section |
| `-` / `=` in a panel | Fold all / unfold all |
| `T` in the files panel | Show titles instead of file names |

## Links and history

| Key | Action |
| --- | --- |
| `]` / `[` | Select the next / previous link |
| `Enter`, click | Follow the selected link or open a footnote |
| `H` / `L` | Back / forward through visited pages |
| `Ctrl-p` | Find a file in the folder |

## Search and editing

| Key | Action |
| --- | --- |
| `/` | Search, Enter to keep, Esc to cancel |
| `n` / `N` | Next / previous match |
| `E` | Edit the file in $EDITOR, reload on return |

## Help and quit

| Key | Action |
| --- | --- |
| `?`, `h` | This help |
| `q` | Quit |

## Remapping

Every key above that is not fixed can be bound in the `[keys]` table of the [config file](configuration.md). An action listed there loses its default keys, and a key it takes is removed from the action that had it. An empty list unbinds the action.

```toml
[keys]
scroll_down = ["j", "down", "ctrl-n"]
quit = ["q", "ctrl-q"]
images = []
```

Keys are written as a single character, which keeps its case, a name (`space`, `enter`, `esc`, `tab`, `backspace`, `up`, `down`, `left`, `right`, `pageup`, `pagedown`, `home`, `end`, `f1` to `f12`), or either with a `ctrl-` or `alt-` prefix. `Esc`, `Tab`, `Ctrl-c` and the panel keys are fixed. `Enter`, `PgDn`, `PgUp`, `Home` and `End` scroll, page and jump unless a binding takes them.

| Action | Default | What it does |
| --- | --- | --- |
| `scroll_down` | `j`, `↓` | Scroll down one line, move down in a panel or list |
| `scroll_up` | `k`, `↑` | Scroll up one line, move up in a panel or list |
| `half_page_down` | `Ctrl-d` | Half page down |
| `half_page_up` | `Ctrl-u` | Half page up |
| `page_down` | `Ctrl-f`, `Space` | Page down |
| `page_up` | `Ctrl-b` | Page up |
| `top` | `g` | Go to the top |
| `bottom` | `G` | Go to the bottom |
| `toc` | `t` | Table of contents |
| `images` | `i` | Show or hide images |
| `front_matter` | `m` | Expand or collapse the front matter |
| `reload` | `r` | Reload the file |
| `files` | `b` | Toggle the files panel |
| `outline` | `o` | Toggle the outline panel |
| `focus_mode` | `f` | Focus mode |
| `focus_right` | `l` | Focus the pane to the right |
| `next_link` | `]` | Select the next link |
| `prev_link` | `[` | Select the previous link |
| `back` | `H` | Back through visited pages |
| `forward` | `L` | Forward through visited pages |
| `finder` | `Ctrl-p` | Find a file in the folder |
| `search` | `/` | Search |
| `next_match` | `n` | Next match |
| `prev_match` | `N` | Previous match |
| `edit` | `E` | Edit the file in $EDITOR |
| `help` | `?`, `h` | Key help |
| `quit` | `q` | Quit |
