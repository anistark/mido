# mido

**mido**: Markdown In, Document Out.

A terminal Markdown reader that renders documents the way a good reader app does: real heading hierarchy, a comfortable line measure, syntax-highlighted code, and tables that line up. Built in Rust on [ratatui](https://ratatui.rs).

## Install

```sh
cargo install --path .
```

## Use

```sh
mido README.md          # open the viewer
mido -                  # read Markdown from stdin
mido -p README.md       # print styled text to stdout, pipe it to less -R
mido -w 80 notes.md     # cap the width (default: full terminal width)
```

Piping without `-p` prints plain text, so `mido file.md > out.txt` never leaks escape codes.

## Keys

| Key | Action |
| --- | --- |
| `j` / `k`, arrows, wheel | Scroll one line |
| `Ctrl-d` / `Ctrl-u` | Half page |
| `Ctrl-f` / `Ctrl-b`, `Space` | Full page |
| `g` / `G` | Top / bottom |
| `b` | Toggle the outline panel |
| `Tab` / `Shift-Tab` | Cycle focus between the panes |
| `Tab` then `←` / `→` | Move focus in that direction |
| `h` / `l` | Focus the outline / the document |
| `Space`, `h` / `l` in the outline | Fold, collapse / expand a section |
| `-` / `=` in the outline | Fold all / unfold all |
| `/` then `n` / `N` | Search, next, previous |
| `t` | Table of contents |
| `r` | Reload |
| `?` | Help |
| `q` | Quit |

The file is watched, so edits in another window show up as you save.

On terminals 100 columns or wider an outline of the headings sits in a tinted card at the top right, and the text wraps before it. It follows your reading position. Press `Tab` or `h` to move into it, `j` / `k` to jump between sections, `Space` to fold a section, and `Tab` or `Enter` to come back.

## What renders

- Headings, paragraphs, *emphasis*, **strong**, ~~strikethrough~~ and `inline code`
- Links with their URL, images as placeholders, footnotes[^1]
- Nested lists, ordered lists, task lists
  - [x] like this one
  - [ ] and this one
- Blockquotes, nested as deep as you like
- Fenced code with syntax highlighting for the languages bat ships
- GFM tables with alignment and wrapped cells

> Anything mido cannot render, such as raw HTML, is shown as source rather than dropped.

[^1]: Footnotes collect at the end of the document.
