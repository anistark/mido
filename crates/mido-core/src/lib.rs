//! Parse, layout and theme engine behind the mido Markdown reader: Markdown to a `Document`,
//! a `Document` to styled lines at a width, and the themes and glyph tiers that color them.
//! No terminal or TUI dependency, only ratatui-core's style and text types.

pub mod markdown;
pub mod render;
