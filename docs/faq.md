---
layout: layouts/page.njk
title: FAQ
description: Answers about colors, fonts, narrow terminals, pipes and what mido does not do yet.
tags: docs
order: 10
---

# FAQ

## Where did the outline go?

The outline card and the files sidebar appear only while the document keeps 60 columns. Widen the terminal, or press `o` to force the outline on, which lowers the limit to 40 columns.

## The colors look wrong

mido picks truecolor when `COLORTERM` is `truecolor` or `24bit`, and 256 colors otherwise. Inside tmux or over SSH the variable is sometimes missing, so export it if your terminal supports truecolor. Set `NO_COLOR`, or pass `--no-color`, to turn colors off entirely.

## Can I change the font?

No. The terminal owns the font. mido draws with box-drawing characters, bullets and bars from the basic Unicode blocks, and the [fonts table](themes.md#fonts) lists what each common font covers. If yours lacks box drawing, set `glyphs = "ascii"` in the [config](configuration.md). If it is a Nerd Font, `glyphs = "nerd"` adds icons.

## Can I change the colors?

Yes. Pick one of the [built-in themes](themes.md#built-in-themes) with `--theme` or `theme` in the config, or write your own. `mido themes` lists them.

## Why is my output plain when I redirect it?

Without `-p`, output that is not a terminal is written as plain text, so `mido file.md > out.txt` never leaks escape codes. Add `-p` to keep the styling, for example `mido -p file.md | less -R`.

## How do I copy text?

`y` copies a code block on the screen, or the selected link. To select any text with the mouse, hold `Shift` while you drag, or `Option` in iTerm2, since mido uses the mouse for scrolling and links. See [copying](navigation.md#copying).

## What about files that are not UTF-8?

A UTF-16 file with a byte order mark, as Windows tools write, reads and saves as UTF-16. A file in another encoding opens with the bytes that are not UTF-8 shown as `�`, and the editor will not open it, so saving cannot change bytes you never touched. A file with NUL bytes is taken for binary and refused.

## Can I edit the file?

Yes. `e` opens it in mido's own [editor](editing.md), which knows enough Markdown to continue lists and toggle tasks, and `Ctrl-s` saves. `E` opens the file in `$VISUAL` or `$EDITOR` instead and reloads it when you come back.

## Do images show?

Yes, when the image sits on its own line. Kitty, iTerm2 and Sixel terminals draw the real picture, and everything else gets a half-block rendering. Press `i` to hide them, or to see which protocol mido detected. Remote images need `--remote-images`.

## Does it run on Windows?

mido is built on crossterm, which supports Windows terminals. It has been developed on macOS and Linux, so please report anything that looks off.

## Can I read the docs offline?

Yes. `mido docs` opens this documentation from a copy bundled in the binary, so it works with no network and always matches the version you have installed. If the current directory has its own `docs` folder, that folder opens instead.

## Where is the config file?

`~/.config/mido/config.toml` on Linux and macOS, `%APPDATA%\mido\config\config.toml` on Windows, and a `.mido.toml` in a project folder overrides it. None is needed, and [Configuration](configuration.md) lists every setting.
