use ratatui::style::Style;
use ratatui::text::Span;
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

#[derive(Debug, Clone)]
pub enum Piece {
    Text {
        text: String,
        style: Style,
        atomic: bool,
    },
    Break,
}

impl Piece {
    pub fn text(text: impl Into<String>, style: Style) -> Self {
        Piece::Text {
            text: text.into(),
            style,
            atomic: false,
        }
    }

    pub fn atomic(text: impl Into<String>, style: Style) -> Self {
        Piece::Text {
            text: text.into(),
            style,
            atomic: true,
        }
    }
}

pub fn width(s: &str) -> usize {
    UnicodeWidthStr::width(s)
}

pub fn wrap(pieces: &[Piece], width: usize) -> Vec<Vec<Span<'static>>> {
    let mut w = Wrapper {
        width: width.max(1),
        lines: Vec::new(),
        cur: Vec::new(),
        cur_width: 0,
        word: Vec::new(),
    };
    for piece in pieces {
        match piece {
            Piece::Break => {
                w.end_word();
                w.flush();
            }
            Piece::Text {
                text,
                style,
                atomic: true,
            } => w.word.push((text.clone(), *style)),
            Piece::Text {
                text,
                style,
                atomic: false,
            } => {
                for token in tokens(text) {
                    if token.starts_with(' ') {
                        w.end_word();
                        w.space(token, *style);
                    } else {
                        w.word.push((token.to_string(), *style));
                    }
                }
            }
        }
    }
    w.end_word();
    if !w.cur.is_empty() {
        w.flush();
    }
    w.lines
}

fn tokens(text: &str) -> impl Iterator<Item = &str> {
    let mut rest = text;
    std::iter::from_fn(move || {
        if rest.is_empty() {
            return None;
        }
        let is_space = rest.starts_with(' ');
        let end = rest
            .find(|c: char| (c == ' ') != is_space)
            .unwrap_or(rest.len());
        let (token, tail) = rest.split_at(end);
        rest = tail;
        Some(token)
    })
}

struct Wrapper {
    width: usize,
    lines: Vec<Vec<Span<'static>>>,
    cur: Vec<Span<'static>>,
    cur_width: usize,
    word: Vec<(String, Style)>,
}

impl Wrapper {
    fn flush(&mut self) {
        while self
            .cur
            .last()
            .is_some_and(|s| s.content.trim().is_empty() && s.style.bg.is_none())
        {
            self.cur.pop();
        }
        let mut merged: Vec<Span<'static>> = Vec::with_capacity(self.cur.len());
        for span in self.cur.drain(..) {
            match merged.last_mut() {
                Some(last) if last.style == span.style => {
                    last.content.to_mut().push_str(&span.content)
                }
                _ => merged.push(span),
            }
        }
        self.lines.push(merged);
        self.cur_width = 0;
    }

    fn push(&mut self, text: &str, style: Style) {
        self.cur_width += width(text);
        self.cur.push(Span::styled(text.to_string(), style));
    }

    fn space(&mut self, text: &str, style: Style) {
        if self.cur_width > 0 {
            self.push(text, style);
        }
    }

    fn end_word(&mut self) {
        if self.word.is_empty() {
            return;
        }
        let word = std::mem::take(&mut self.word);
        let total: usize = word.iter().map(|(t, _)| width(t)).sum();
        if self.cur_width > 0 && self.cur_width + total > self.width {
            self.flush();
        }
        if total <= self.width {
            for (text, style) in &word {
                self.push(text, *style);
            }
            return;
        }
        for (text, style) in &word {
            let mut chunk = String::new();
            let mut chunk_width = 0;
            for ch in text.chars() {
                let cw = UnicodeWidthChar::width(ch).unwrap_or(0);
                if self.cur_width + chunk_width + cw > self.width
                    && self.cur_width + chunk_width > 0
                {
                    self.push(&chunk, *style);
                    self.flush();
                    chunk.clear();
                    chunk_width = 0;
                }
                chunk.push(ch);
                chunk_width += cw;
            }
            self.push(&chunk, *style);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plain(lines: &[Vec<Span<'static>>]) -> Vec<String> {
        lines
            .iter()
            .map(|l| l.iter().map(|s| s.content.as_ref()).collect())
            .collect()
    }

    #[test]
    fn wraps_at_word_boundaries_and_trims() {
        let pieces = [Piece::text(
            "the quick brown fox jumps over the lazy dog",
            Style::new(),
        )];
        assert_eq!(
            plain(&wrap(&pieces, 10)),
            ["the quick", "brown fox", "jumps over", "the lazy", "dog"]
        );
    }

    #[test]
    fn style_changes_mid_word_do_not_break() {
        let pieces = [
            Piece::text("ab", Style::new()),
            Piece::text("cd ef", Style::new().bold()),
        ];
        assert_eq!(plain(&wrap(&pieces, 4)), ["abcd", "ef"]);
    }

    #[test]
    fn long_words_are_hard_split_and_breaks_respected() {
        let pieces = [
            Piece::text("abcdefghij", Style::new()),
            Piece::Break,
            Piece::text("x", Style::new()),
        ];
        assert_eq!(plain(&wrap(&pieces, 4)), ["abcd", "efgh", "ij", "x"]);
    }

    #[test]
    fn glued_fragments_move_to_the_next_line_together() {
        let pieces = [
            Piece::text("see the ", Style::new()),
            Piece::text("link", Style::new().bold()),
            Piece::text(" ", Style::new()),
            Piece::text("(https://x.y)", Style::new().dim()),
            Piece::text(".", Style::new()),
        ];
        assert_eq!(
            plain(&wrap(&pieces, 20)),
            ["see the link", "(https://x.y)."]
        );
    }

    #[test]
    fn atomic_pieces_keep_inner_spaces() {
        let pieces = [
            Piece::text("see", Style::new()),
            Piece::text(" ", Style::new()),
            Piece::atomic(" a b ", Style::new()),
        ];
        assert_eq!(plain(&wrap(&pieces, 6)), ["see", " a b "]);
    }

    #[test]
    fn wide_characters_count_double() {
        let pieces = [Piece::text("日本語 テキスト", Style::new())];
        assert_eq!(plain(&wrap(&pieces, 8)), ["日本語", "テキスト"]);
    }
}
