---
layout: layouts/page.njk
title: Navigation
description: Following links, history, the fuzzy file finder, search, the table of contents, editing and live reload.
tags: docs
order: 5
---

# Navigation

Everything that moves you around a document or a folder: links, history, the file finder, search, the table of contents, and getting out to an editor.

## Links

`]` selects the next link on the page and `[` the previous one, including links inside table cells. `Enter` or a click follows it. What happens depends on the target:

| Target | Result |
| --- | --- |
| A relative Markdown file, like `guide/keys.md` | Opens in place |
| A wikilink, like `[[Getting Started]]` or `[[keys#reading\|label]]` | Opens the file in the folder whose name or first heading matches, or `name.md` next to the current file |
| A footnote marker | Opens the note in a popup |
| An anchor, like `#install` | Jumps to that heading, using GitHub's slug rules |
| A folder | Opens its README |
| Any other file | Opens with the system opener |
| A web URL | Opens in your browser |

Links show their URL dimmed after the text, so you can see where they go before following them.

## History

`H` goes back and `L` goes forward through the pages you have visited, restoring the scroll position each time. This works like a browser: following a link pushes onto the history, going back and then following a different link starts a new branch.

## File finder

`Ctrl-p` opens a fuzzy finder over every file in the folder. It matches on the path and on the first heading of each file, so typing part of a title finds the file even when the name is different. `Enter` opens the selection, `Esc` closes the finder.

## Search

`/` starts a search over the rendered text. Matches are highlighted as you type. `Enter` keeps the query so `n` and `N` step through the matches, and `Esc` cancels. The search is smart-case: all lowercase matches any case, a capital letter makes it exact.

## Table of contents

`t` opens the headings as a list in an overlay. `j` and `k` move, `Enter` jumps to the heading, `Esc` closes.

## Editing

`E` suspends the viewer, opens the current file in `$VISUAL` or `$EDITOR`, and reloads the document when the editor exits. The terminal is restored cleanly on the way out and back.

## Live reload

The open file is watched, so edits saved from another window show up as soon as they land. `r` reloads by hand. In folder mode the whole tree is watched, so new and removed files appear in the sidebar too.
