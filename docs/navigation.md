---
layout: layouts/page.njk
title: Navigation
description: Following links, history, the fuzzy file finder, search in a file or a folder, copying, the table of contents, editing and live reload.
tags: docs
order: 5
---

# Navigation

Everything that moves you around a document or a folder: links, history, the file finder, search, copying, the table of contents, and getting out to an editor.

## Links

`]` selects the next link on the page and `[` the previous one, including links inside table cells. `Enter` or a click follows it. What happens depends on the target:

| Target | Result |
| --- | --- |
| A relative Markdown file, like `guide/keys.md` | Opens in place |
| A wikilink, like `[[Getting Started]]` or `[[keys#reading\|label]]` | Opens the file in the folder whose name or first heading matches, or `name.md` next to the current file |
| A footnote marker | Opens the note in a popup |
| An anchor, like `#install` | Jumps to that heading, using GitHub's slug rules |
| A folder | Opens its README |
| A web or mail link, `http`, `https` or `mailto` | Opens in your browser or mail app |
| Any other file, or any other kind of URL | Asks first, then opens with the system opener |

The system opener runs programs and scripts as readily as it opens documents, so a link in a cloned README to `setup.command`, `install.bat` or a `file://` app would run it. mido names the target and waits for `o` before handing it over. `Esc` or `n` cancels.

Links show their URL dimmed after the text, so you can see where they go before following them. The selected link's target is also shown in the status bar, which helps with wikilinks and links whose URL is hidden, like a badge.

## History

`H` goes back and `L` goes forward through the pages you have visited, restoring the scroll position each time. This works like a browser: following a link pushes onto the history, going back and then following a different link starts a new branch.

## File finder

`Ctrl-p` opens a fuzzy finder over every file in the folder. It matches on the path and on the first heading of each file, so typing part of a title finds the file even when the name is different. `Enter` opens the selection, `Esc` closes the finder.

## Search

`/` starts a search over the rendered text. Matches are highlighted as you type. `Enter` keeps the query so `n` and `N` step through the matches, and `Esc` cancels. The search is smart-case: all lowercase matches any case, a capital letter makes it exact.

In a folder, `F` searches every Markdown file in it. Each matching line is listed as `path:line` with the line beside it, up to 200 of them. `Enter` opens the file at that line and keeps the query, so `n` and `N` go on through the matches in that file, and `H` comes back. This search reads the Markdown source, so it also finds text inside link targets and code, and the in-file search afterwards matches the rendered text.

## Copying

`y` copies the selected link's target, or with no link selected, a code block on the screen. With several code blocks in view, pressing `y` again moves on to the next one, and the status bar says which one was copied. The copy has no trailing newline, so pasting a command into a shell does not run it straight away. The text goes to the clipboard through OSC 52, which works over SSH and in tmux, and on your own machine through `pbcopy`, `wl-copy`, `xclip` or `xsel` as well, for terminals that ignore OSC 52.

mido takes the mouse to scroll and follow links, so a plain drag does not select text. Hold `Shift` while dragging, or `Option` in iTerm2, to select with the terminal as usual. In Apple Terminal, turn off View, Allow Mouse Reporting for a moment instead.

## Reading position

A file opens again where you left it, and the status bar says so. Following a link to a heading, or a match from the folder search, goes there instead. `g` goes to the top. Set `remember_position = false` in the [config](configuration.md) to always start at the top.

## Table of contents

`t` opens the headings as a list in an overlay. `j` and `k` move, `Enter` jumps to the heading, `Esc` closes.

## Editing

`e` opens the file in mido's own editor, at the part you were reading, and `Esc` comes back. See [Editing](editing.md).

`E` suspends the viewer, opens the current file in `$VISUAL` or `$EDITOR`, and reloads the document when the editor exits. The terminal is restored cleanly on the way out and back.

## Live reload

The open file is watched, so edits saved from another window show up as soon as they land. `r` reloads by hand. In folder mode the whole tree is watched, so new and removed files appear in the sidebar too.
