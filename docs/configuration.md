---
layout: layouts/page.njk
title: Configuration
description: The config file, the per-project override, and every setting in them.
tags: docs
order: 7
---

# Configuration

mido needs no configuration. Everything below is optional, and a missing file means the defaults.

## Where the file lives

| Platform | Config file |
| --- | --- |
| Linux, macOS | `~/.config/mido/config.toml`, or `$XDG_CONFIG_HOME/mido/config.toml` when that is set |
| Windows | `%APPDATA%\mido\config\config.toml` |

`MIDO_CONFIG_DIR` points mido at another folder, and `--config <FILE>` reads one file instead of the one in the folder. User themes live next to the config, in a `themes` folder. See [Themes](themes.md).

## Per-project settings

A `.mido.toml` in the folder you open, or in any folder above it, is read after the user config and wins where both set something. For a single file mido looks beside the file and upwards. Commit one to a docs repository to give every reader the same width or theme. A relative theme path in it is read from the folder the `.mido.toml` sits in.

Command line flags win over both files.

## Settings

```toml
theme = "auto"               # a theme name, a path to a theme file, or auto
dark_theme = "mido-dark"     # used by auto on a dark terminal
light_theme = "mido-light"   # used by auto on a light terminal
background = "auto"          # auto, dark or light
glyphs = "unicode"           # ascii, unicode or nerd
width = 100                  # cap the measure in columns and center it
gutter = 2                   # columns of margin each side from 60 columns wide
front_matter = "collapsed"   # collapsed, expanded or hidden
extensions = ["md", "markdown", "mdown", "mkd", "mdx"]

[keys]
quit = ["q", "ctrl-q"]
```

| Setting | Default | Meaning |
| --- | --- | --- |
| `theme` | `auto` | The theme to use. `auto` picks `dark_theme` or `light_theme` from the terminal background. `--theme` overrides it |
| `dark_theme` | `mido-dark` | The theme `auto` uses on a dark background |
| `light_theme` | `mido-light` | The theme `auto` uses on a light background |
| `background` | `auto` | Skip detection and treat the terminal as `dark` or `light` |
| `glyphs` | `unicode` | The symbol set, see [glyph tiers](themes.md#glyph-tiers) |
| `width` | none | Cap the measure, the same as `--width`. At least 20 |
| `gutter` | `2` | Blank columns each side of the text on terminals 60 columns or wider. Narrower terminals use at most one, and the right side always keeps a column for the scrollbar |
| `front_matter` | `collapsed` | How front matter starts: a row of chips, the full source card, or not at all. `m` still toggles it |
| `extensions` | `md`, `markdown`, `mdown`, `mkd`, `mdx` | File extensions that count as Markdown in the files panel, the finder, the watcher and link following |
| `[keys]` | the keymap | Rebind actions, see [remapping](keys.md#remapping) |

An unknown setting, a misspelled value or a key that does not parse stops mido with the file and the problem named, rather than being ignored.

## Background detection

With `theme = "auto"` and `background = "auto"`, mido asks the terminal for its background color once at startup with an OSC 11 query and waits at most 100 ms for the answer. If the terminal does not answer, it reads `COLORFGBG`, and if that is unset too it assumes dark. The query is only sent when both stdin and stdout are a terminal, so pipes and redirects never see it. Set `background` when your terminal answers wrongly or slowly.

## Remote images are not a setting

`--remote-images` stays a flag on purpose. A `.mido.toml` comes with whatever folder you open, and a file in a cloned repository should not be able to make mido reach the network.
