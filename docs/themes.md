---
layout: layouts/page.njk
title: Themes
description: Built-in themes, light and dark detection, writing your own theme, glyph tiers and fonts.
tags: docs
order: 8
---

# Themes

A theme recolors mido. It never changes how an element is drawn: headings, code blocks, tables and chips keep the shapes described in [What renders](rendering.md), and a theme only chooses their colors and the syntax colors inside code blocks.

## Choosing a theme

```sh
mido --theme nord README.md     # for one run
mido themes                     # list every theme with a swatch of its colors
```

Or set it in the [config file](configuration.md):

```toml
theme = "tokyo-night"
```

The default is `auto`, which asks the terminal whether its background is light or dark and uses `mido-light` or `mido-dark` to match. `dark_theme` and `light_theme` change that pair, and `background` skips the question. `NO_COLOR` still turns every color off, whichever theme is set.

The [theme gallery](https://anistark.github.io/mido/gallery/) shows the same page in every built-in theme, drawn in CI from the screen buffer the tests use.

`mido-dark` is the theme mido has always had, and together with `mido-light` it draws on the terminal's own background. The other built-in themes paint their background too, so `nord` looks like Nord in any terminal.

## Writing a theme

A theme is a TOML file. Put it in the `themes` folder next to your config, `~/.config/mido/themes/rose.toml`, and use it by name with `--theme rose`, or give a path anywhere with `--theme ./rose.toml`. A user theme with the name of a built-in one replaces it.

The shortest theme extends another and changes a few tokens:

```toml
schema = 1
extends = "mido-dark"

[colors]
accent = "#f5c2e7"
h1 = "#f5c2e7"
```

A theme that stands alone sets `kind` and every token in the list below. mido refuses one that misses any and names the missing tokens:

```toml
schema = 1
kind = "dark"                # dark or light, picks the default syntax theme and chip text
syntax = "catppuccin-mocha"  # optional, see the syntax themes below

[palette]
rose = "#f5c2e7"
night = "#1e1e2e"

[colors]
bg = "night"
accent = "rose"
# ...and every other token
```

- `schema` is `1`. A future format change bumps it, and mido says so instead of misreading the file.
- `[palette]` names colors once, and any value in `[colors]` can use a palette name. A palette name wins over an ANSI color name, so a palette `red` means your red.
- A color is `#rrggbb`, `#rgb`, an ANSI name such as `blue` or `bright-magenta`, a 256-color index such as `208`, or `default` for the terminal's own color.
- `extends` takes a built-in name or a user theme name. Chains work, and a loop is reported.
- `syntax` names a syntect theme for code blocks. Without it a dark theme uses `catppuccin-mocha` and a light one `catppuccin-latte`. Code blocks keep the theme's `code_bg` behind the syntax colors.

The built-in themes are good starting points. Their files are in [crates/mido-core/src/render/themes](https://github.com/anistark/mido/tree/main/crates/mido-core/src/render/themes) in the repository.

Colors are written in truecolor. On a terminal without truecolor mido maps each one to the nearest of the 256 colors, so a theme needs no separate palette for it.

## Glyph tiers

`glyphs` in the config picks the symbols mido draws. Colors and glyphs are independent.

| Tier | What it uses |
| --- | --- |
| `unicode` | The default. Box drawing, bullets `•` `◦` `▪`, bars `▎`, task boxes `☐` `☑`, fold markers `▾` `▸`, the image marker `▣`, superscript footnote numbers |
| `ascii` | Plain ASCII only: tables and frames from `+` and `-`, `-` `*` `+` bullets, `[ ]` and `[x]` tasks, `>` and `v` markers, `[1]` footnotes, and Mermaid diagrams drawn in ASCII. For fonts without box drawing, serial consoles and logs |
| `nerd` | The Unicode set plus Nerd Font icons: folder and Markdown file icons in the files panel, an image icon, and GitHub-style icons on alerts. Needs a patched [Nerd Font](https://www.nerdfonts.com) |

## Fonts

The terminal owns the font, so mido cannot pick one. What it can do is stay inside the glyphs your font has. These fonts were checked against every symbol the `unicode` tier draws, including the Mermaid diagrams, by reading the font's character map. A missing glyph still shows, drawn from the terminal's fallback font, so it may look slightly different from its neighbours.

| Font | `unicode` tier | `nerd` tier | Notes |
| --- | --- | --- | --- |
| Menlo | Every glyph | No | The macOS Terminal default, complete coverage |
| MesloLGS NF | Every glyph | Every icon | Menlo with Nerd Font icons, the best fit for `glyphs = "nerd"` |
| JetBrains Mono | All but `▣` `☐` `☑` `✦` | No | The image marker, task boxes and the TIP icon come from the fallback font. Its Nerd Font build adds the icons |
| SF Mono | All but `▣` `▪` `◆` `☐` `☑` `✦`, and `△` `►` `◄` `◉` in some diagrams | No | Works, with a few more fallback glyphs |
| Source Code Pro | All but `▣` `▪` `▸` `▾` `●` `◦` `✦`, and `►` `◄` in diagrams | No | Fold markers and bullets come from the fallback font. Checked as the Powerline Awesome build |
| Monaco, Courier New, Andale Mono | Many missing, including rounded corners and heavy rules | No | Old fonts without the newer box drawing. Use `glyphs = "ascii"` |

Fira Code, Iosevka, Monaspace and Cascadia Code have not been checked yet. Reports for them, or for any other font, are welcome.

## Built-in themes

| Theme | Kind | Code | About |
| --- | --- | --- | --- |
| `mido-dark` | dark | `catppuccin-mocha` | mido's default dark theme, on the terminal's own background. |
| `mido-light` | light | `catppuccin-latte` | mido's light theme, on the terminal's own background. |
| `mido-reading` | light | `gruvbox-light` | A warm, low-contrast paper palette for long reads. Paints its own background. |
| `catppuccin-mocha` | dark | `catppuccin-mocha` | Catppuccin Mocha, from the official palette at catppuccin.com/palette. |
| `catppuccin-latte` | light | `catppuccin-latte` | Catppuccin Latte, from the official palette at catppuccin.com/palette. |
| `gruvbox` | dark | `gruvbox-dark` | Gruvbox dark, medium contrast, from github.com/morhetz/gruvbox. |
| `nord` | dark | `nord` | Nord, from nordtheme.com. The muted text shade sits between Snow Storm and Polar Night. |
| `tokyo-night` | dark | `catppuccin-macchiato` | Tokyo Night, from github.com/folke/tokyonight.nvim. two-face has no Tokyo Night syntax theme, and Catppuccin Macchiato uses the same hues for the same scopes. |
| `dracula` | dark | `dracula` | Dracula, from draculatheme.com/contribute. The muted text shade is derived from the foreground. |
| `solarized-dark` | dark | `solarized-dark` | Solarized dark, from ethanschoonover.com/solarized. The chip shade is derived from base02. |
| `solarized-light` | light | `solarized-light` | Solarized light, from ethanschoonover.com/solarized. The chip shade is derived from base2. |

## Tokens

Every token a theme sets, in the order `mido-dark.toml` lists them. A theme without `extends` must set all of them.

| Token | Paints |
| --- | --- |
| `bg` | Background behind the whole viewer, `default` keeps the terminal's own |
| `fg` | Body text |
| `fg_muted` | Secondary text: H5 and H6, quotes, front matter keys, panel connectors |
| `fg_faint` | Hints, placeholders, raw HTML, the overflow marker |
| `accent` | Status chip, focus, footnote markers, the image marker |
| `h1` | The H1 title chip |
| `h2` | H2 text and its underline, second-level list markers |
| `h3` | H3 text, third-level list markers |
| `h4` | H4 text |
| `h5` | H5 text |
| `h6` | H6 text |
| `h1_fg` | Text on the H1 chip and the status chip |
| `link` | Link text |
| `link_url` | The URL shown after a link |
| `code_fg` | Inline code text |
| `code_bg` | Inline code and code block background |
| `code_border` | The bar left of a code block |
| `surface` | Panel tint, table header band |
| `quote_bar` | The bar left of a quote |
| `quote_fg` | Quote text |
| `rule` | Thematic breaks |
| `table_border` | Table box drawing |
| `list_marker` | First-level list markers |
| `status_bg` | Status bar background |
| `status_fg` | Status bar text |
| `search_bg` | Search matches |
| `search_fg` | Text on a search match |
| `search_current_bg` | The current search match |
| `overlay_border` | Border of overlays: help, contents, finder |
| `selected_bg` | Selected row in a panel or overlay |
| `alert_note` | NOTE alerts |
| `alert_tip` | TIP alerts |
| `alert_important` | IMPORTANT alerts |
| `alert_warning` | WARNING alerts |
| `alert_caution` | CAUTION alerts |
| `math` | Inline and block math |
| `label_bg` | Key side of a front matter or badge chip |
| `label_value_bg` | Value side of a front matter chip |

## Syntax themes

`syntax` takes one of these names, the set bat ships:

`ansi`, `base16`, `base16-eighties-dark`, `base16-mocha-dark`, `base16-ocean-dark`, `base16-ocean-light`, `base16-256`, `catppuccin-frappe`, `catppuccin-latte`, `catppuccin-macchiato`, `catppuccin-mocha`, `coldark-cold`, `coldark-dark`, `darkneon`, `dracula`, `github`, `gruvbox-dark`, `gruvbox-light`, `inspiredgithub`, `1337`, `monokai-extended`, `monokai-extended-bright`, `monokai-extended-light`, `monokai-extended-origin`, `nord`, `onehalfdark`, `onehalflight`, `solarized-dark`, `solarized-light`, `sublime-snazzy`, `twodark`, `zenburn`
