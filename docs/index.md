---
layout: layouts/home.njk
title: mido
tags: docs
order: 0
---

# mido

**mido**: Markdown In, Document Out.

A terminal Markdown reader that renders documents the way a good reader app does: real heading hierarchy, a comfortable line measure, syntax-highlighted code, and tables that line up. Built in Rust on [ratatui](https://ratatui.rs).

## Install

```sh
cargo install mido
```

Then open a file, or a whole folder:

```sh
mido README.md
mido docs/
```

This documentation is itself a mido project. Run `mido docs/` inside the repository to read it in the terminal.
