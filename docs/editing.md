---
layout: layouts/page.njk
title: Editing
description: The built-in editor, its keys, the Markdown it helps with, saving, and new files.
tags: docs
order: 6
---

# Editing

mido has an editor of its own for fixing a typo or writing a note without leaving the reader. `e` swaps the rendered page for its Markdown source, at the part you were reading, and `Esc` swaps back. `E` still opens the file in `$VISUAL` or `$EDITOR` for anything bigger.

## Opening and leaving

`e` opens the current file. The source scrolls to where the page was, and the cursor sits on the first line in view. The status bar shows the file name, `[+]` once there are unsaved changes, and the line and column of the cursor.

`Esc` goes back to reading, and the page shows what you wrote. With unsaved changes it asks first: `s` saves and leaves, `d` throws the changes away and leaves, and `Esc` keeps editing. Standard input has no file to save to, so it cannot be edited.

## Keys

The editor works like most editors outside a terminal: what you type goes in, the arrows move, and `Shift` with a move selects. Dragging with the mouse selects too, and a click places the cursor.

| Key | Action |
| --- | --- |
| `Ctrl-s` | Save |
| `Esc` | Back to reading, asking first about unsaved changes |
| `Ctrl-z` / `Ctrl-y` | Undo / redo |
| `Ctrl-x` / `Ctrl-c` / `Ctrl-v` | Cut / copy / paste |
| `Ctrl-a` | Select everything |
| `Ctrl-f`, then `Ctrl-n` / `Ctrl-p` | Find, then the next / previous match |
| `Ctrl-t` | Check or uncheck a task, or make the line one |
| `Ctrl-←` / `Ctrl-→`, `Alt-b` / `Alt-f` | Word left / right |
| `Ctrl-↑` / `Ctrl-↓` | Paragraph up / down |
| `Home` / `End`, `Ctrl-Home` / `Ctrl-End` | Line start / end, top / bottom |
| `Alt-Backspace`, `Ctrl-w` / `Alt-Delete` | Delete a word back / forward |
| `F1` | Key help |

In the editor `Ctrl-c` copies instead of quitting. Every key with a name in the [keys page](keys.md#remapping) can be bound again in the `[keys]` table of the config. Editor keys need a `ctrl-` or `alt-` prefix, since a plain letter would type.

Undo steps follow what you did: a run of typing undoes a word at a time, and an assist like continuing a list undoes in one step.

## Markdown help

The source is colored with the theme: headings in their level's color with faint `#` marks, emphasis and code as they look on the page, link targets dimmed, list markers, quote bars, table borders and the front matter. Fenced code is highlighted for its language.

A few keys know Markdown:

- `Enter` on a list item starts the next one, `- ` for a bullet, the next number for a numbered list, and an empty box for a task. On a quote it starts the next quoted line. `Enter` on an item or quote line with nothing in it ends the list or quote instead. Inside a code fence and on indented lines it keeps the indent.
- `Tab` and `Shift-Tab` on a list item nest it one level deeper or shallower. Elsewhere `Tab` inserts spaces up to the next multiple of four.
- `Ctrl-t` checks or unchecks a task. On a list item without a box it adds one, and on a plain line it makes a task item.
- A backtick or an asterisk is typed in pairs with the cursor between them, and typing the closing one steps over it. With a selection, it wraps the selection: select a word and press `*` to emphasise it. `Backspace` between an empty pair removes both. An asterisk at the start of a line is left alone, since it is likely a bullet.

## Find

`Ctrl-f` opens a find prompt in the status bar. Matches are highlighted as you type and the first one after the cursor is selected, so typing replaces it. Like search in the reader, lowercase matches either case and any uppercase letter makes it exact. `Enter` keeps the matches, and `Ctrl-n` and `Ctrl-p` step through them. `Esc` in the prompt puts the cursor back where it was.

## Copy and paste

`Ctrl-c` and `Ctrl-x` copy to mido's own clipboard and to the system one, through the terminal's OSC 52 support, which also works over SSH and in tmux with `set-clipboard on`. `Ctrl-v` pastes what mido copied. To paste from the system clipboard, use the terminal's own paste, `Cmd-v` or `Ctrl-Shift-v`. mido reads it as one paste, so a pasted list is not continued line by line.

## Saving

`Ctrl-s` writes to a temporary file beside the original and renames it over the top, so a crash or a full disk never leaves half a file. Saving a file you did not change writes the same bytes back: line endings, a byte order mark and the trailing newline, or its absence, are kept as they were. A file with both kinds of line ending is saved with the more common one, and mido says so when it opens. Saving through a symbolic link writes the file it points to.

If the file changed on disk since you opened it, from another editor or a `git checkout`, mido does not overwrite it silently. It asks: `o` overwrites, `r` reads the file again and drops your changes, `Esc` cancels. The open buffer is never replaced under you while you edit. A note in the status bar tells you the file changed.

## New files

```sh
mido new notes.md      # create notes.md on first save and open it in the editor
mido ideas.md          # the same for a Markdown path that does not exist yet
```

The file and any missing folders are created on the first `Ctrl-s`. Until then nothing is written. A path without a Markdown extension that does not exist is an error, unless it comes after `new`.

## If mido crashes

If mido panics while you have unsaved changes, it restores the terminal, writes what you had to a hidden file beside the original, such as `.notes.md.mido-recover`, and prints its path. The next time you edit that file mido points it out. Saving removes it.
