---
layout: layouts/page.njk
title: Getting started
description: Install mido, open a Markdown file or a folder, and find your way around.
tags: docs
order: 1
---

# Getting started

mido reads Markdown in the terminal. Point it at a file and it renders the document with real headings, wrapped paragraphs, highlighted code and aligned tables. Point it at a folder and it becomes a documentation browser.

## Install

mido is on [crates.io](https://crates.io/crates/mido). With a Rust toolchain of 1.93 or newer:

```sh
cargo install mido
```

To build the latest commit instead of the latest release:

```sh
cargo install --git https://github.com/anistark/mido
```

Or from a clone:

```sh
git clone https://github.com/anistark/mido
cd mido
cargo install --path .
```

Either way the `mido` binary lands in `~/.cargo/bin`, which is on your `PATH` if `cargo` is.

## Open a file

```sh
mido README.md
```

The document fills the terminal. Scroll with `j` and `k`, the arrow keys or the mouse wheel. `g` and `G` jump to the top and the bottom. Press `q` to leave.

If the terminal is wide enough, an outline of the headings appears in a card at the top right. It follows your reading position as you scroll.

## Open a folder

```sh
mido docs/
mido
```

With a folder, or no argument at all, mido lists the Markdown files in a sidebar on the left and opens README.md, then index.md, then the first file it finds. Relative links between files open in place and `H` takes you back. See [Panels](panels.md) and [Navigation](navigation.md).

## Get help

Press `?` or `h` at any time for the key overlay, grouped by task. `t` opens a table of contents you can jump from. The full list is on the [Keys](keys.md) page.

This documentation ships inside the binary. `mido docs` opens it offline, in project mode, whenever there is no `docs` folder in the current directory.

## Read from a pipe

```sh
curl -s https://raw.githubusercontent.com/anistark/mido/main/README.md | mido -
```

A single `-` reads the document from stdin. To send rendered output somewhere else instead of opening the viewer, see print mode on the [Command line](cli.md#print-mode) page.

## Colors

mido uses truecolor when the terminal advertises it through `COLORTERM`, and falls back to 256 colors otherwise. Setting `NO_COLOR` turns colors off and keeps the structure through bold, italic and rules.

It also asks the terminal whether its background is light or dark and picks `mido-light` or `mido-dark` to match. `mido themes` lists the other built-in themes, and `--theme nord` tries one. To keep a choice, put it in the [config file](configuration.md), and see [Themes](themes.md) to write your own.
