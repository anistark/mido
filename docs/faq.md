---
layout: layouts/page.njk
title: FAQ
description: Answers about colors, fonts, narrow terminals, pipes and what mido does not do yet.
tags: docs
order: 7
---

# FAQ

## Where did the outline go?

The outline card and the files sidebar appear only while the document keeps 60 columns. Widen the terminal, or press `o` to force the outline on, which lowers the limit to 40 columns.

## The colors look wrong

mido picks truecolor when `COLORTERM` is `truecolor` or `24bit`, and 256 colors otherwise. Inside tmux or over SSH the variable is sometimes missing, so export it if your terminal supports truecolor. Set `NO_COLOR` to turn colors off entirely.

## Can I change the font?

No. The terminal owns the font. mido draws with box-drawing characters, bullets and bars from the basic Unicode blocks that every modern terminal font covers. JetBrains Mono, Fira Code, Iosevka, Monaspace, SF Mono and Cascadia Code all render it well.

## Why is my output plain when I redirect it?

Without `-p`, output that is not a terminal is written as plain text, so `mido file.md > out.txt` never leaks escape codes. Add `-p` to keep the styling, for example `mido -p file.md | less -R`.

## Can I edit the file?

`E` opens the file in `$VISUAL` or `$EDITOR` and reloads it when you come back. A built-in editor is planned.

## Do images show?

Yes, when the image sits on its own line. Kitty, iTerm2 and Sixel terminals draw the real picture, and everything else gets a half-block rendering. Press `i` to hide them, or to see which protocol mido detected. Remote images need `--remote-images`.

## Does it run on Windows?

mido is built on crossterm, which supports Windows terminals. It has been developed on macOS and Linux, so please report anything that looks off.

## Can I read the docs offline?

Yes. `mido docs` opens this documentation from a copy bundled in the binary, so it works with no network and always matches the version you have installed. If the current directory has its own `docs` folder, that folder opens instead.

## Where is the config file?

There is none yet. Themes, fonts and configuration are the theme of the 0.5 release. Until then the defaults are the design.
