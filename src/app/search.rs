use crate::render::layout::Layout;
use crate::render::wrap::width;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Match {
    pub line: usize,
    pub start: u16,
    pub end: u16,
}

pub fn find(layout: &Layout, query: &str) -> Vec<Match> {
    let insensitive = !query.chars().any(char::is_uppercase);
    let needle: Vec<char> = query.chars().map(|c| fold(c, insensitive)).collect();
    let mut out = Vec::new();
    for (line_no, line) in layout.plain_lines().iter().enumerate() {
        let chars: Vec<(usize, char)> = line
            .char_indices()
            .map(|(i, c)| (i, fold(c, insensitive)))
            .collect();
        let mut i = 0;
        while i + needle.len() <= chars.len() {
            if chars[i..i + needle.len()]
                .iter()
                .map(|(_, c)| *c)
                .eq(needle.iter().copied())
            {
                let start = chars[i].0;
                let end = chars.get(i + needle.len()).map_or(line.len(), |(b, _)| *b);
                out.push(Match {
                    line: line_no,
                    start: width(&line[..start]) as u16,
                    end: width(&line[..end]) as u16,
                });
                i += needle.len();
            } else {
                i += 1;
            }
        }
    }
    out
}

fn fold(c: char, insensitive: bool) -> char {
    if insensitive {
        c.to_lowercase().next().unwrap_or(c)
    } else {
        c
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::markdown::parse;
    use crate::render::{layout::layout, theme::Theme};

    #[test]
    fn smart_case_and_display_columns() {
        let out = layout(
            &parse("# Héllo\n\nsay hello, HELLO and héllo\n"),
            &Theme::dark(),
            80,
        );
        let hits = find(&out, "hello");
        assert_eq!(hits.len(), 2);
        assert_eq!(
            hits[0],
            Match {
                line: 2,
                start: 4,
                end: 9
            }
        );
        assert_eq!(find(&out, "HELLO").len(), 1);
        assert_eq!(find(&out, "héllo").len(), 2);
    }
}
