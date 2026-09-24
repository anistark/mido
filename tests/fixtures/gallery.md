---
title: Field notes
tags: [theme, gallery]
---

# Field notes

mido renders **Markdown** the way a reader app does, with `inline code`, *emphasis* and [links](https://anistark.github.io/mido) that keep their URL.

## Checklist

- [x] Pick a theme with `--theme`
- [ ] Write one of your own
  - nested items take the next marker

## Code

```rust
fn main() {
    let theme = "nord"; // any built-in name
    println!("reading in {theme}");
}
```

| Theme | Kind | Paints its background |
| --- | :---: | --- |
| mido-dark | dark | no |
| nord | dark | yes |

> [!TIP]
> `mido themes` shows a swatch of every theme.

> A quote keeps its bar and turns italic.

### Math and more

Inline math like $e^{i\pi} + 1 = 0$ stays as styled source.
