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

`PATH` is a Markdown file, a folder, `-` for stdin, or `docs` for the bundled documentation. With no path, mido opens the current folder.

## Options

| Flag | Meaning |
| --- | --- |
| `-p`, `--print` | Print styled text to stdout instead of opening the viewer |
| `-w`, `--width <COLS>` | Cap the content width in columns. The default is the full terminal width |
| `--man` | Print the man page as roff to stdout |
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
mido docs               # read this documentation offline
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

## Bundled documentation

`mido docs` opens this documentation in project mode with no network, unpacked from the binary into a temporary folder. A file or folder called `docs` in the current directory always wins, so inside a project with its own docs folder `mido docs` opens that folder, exactly like `mido docs/`.

## Man page

The man page is generated at build time from the command definition. `mido --man` writes it as roff, so it can be read directly or installed:

```sh
mido --man | man -l -
mido --man > ~/.local/share/man/man1/mido.1
```

## Exit

`q` quits the viewer. If an overlay is open, `Esc` closes it first.
