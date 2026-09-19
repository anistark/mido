---
layout: layouts/page.njk
title: Command line
description: The mido command, its flags, stdin, and print mode for pipes.
tags: docs
order: 2
---

# Command line

```
mido [OPTIONS] [PATH]
```

`PATH` is a Markdown file, a folder, or `-` for stdin. With no path, mido opens the current folder.

## Options

| Flag | Meaning |
| --- | --- |
| `-p`, `--print` | Print styled text to stdout instead of opening the viewer |
| `-w`, `--width <COLS>` | Cap the content width in columns. The default is the full terminal width |
| `-h`, `--help` | Show the help |
| `-V`, `--version` | Show the version |

## Examples

```sh
mido README.md          # open one file
mido docs/              # browse a folder: files on the left, outline on the right
mido                    # the current folder
mido -                  # read Markdown from stdin
mido -p README.md       # print styled text to stdout, pipe it to less -R
mido -w 80 notes.md     # cap the width at 80 columns
```

## Width

By default the text runs the full width of the terminal, with small gutters on each side. `--width` caps the measure and centers the column, for anyone who prefers a fixed reading width on a wide screen.

## Print mode

`-p` renders the document once and writes it to stdout with ANSI styling, so it can go through a pager or into another tool:

```sh
mido -p README.md | less -R
```

Piping or redirecting without `-p` writes plain text with no escape codes, so this never leaks colors into a file:

```sh
mido README.md > out.txt
```

## Exit

`q` quits the viewer. If an overlay is open, `Esc` closes it first.
