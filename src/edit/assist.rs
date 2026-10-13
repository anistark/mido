//! Markdown assists as plain functions of the lines and the cursor. Each returns the commands
//! that do the job, or `None` to let the key do what it does anywhere else.

use super::{Command, Motion};

const PAIRS: [char; 2] = ['`', '*'];

#[derive(Debug, Clone, PartialEq, Eq)]
enum Marker {
    Bullet(char),
    Ordered { number: u64, delimiter: char },
}

/// The leading part of a line that a new line continues: quote markers, indentation, a list
/// marker and a task box. Columns are in characters.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Prefix {
    quote: String,
    indent: usize,
    marker: Option<Marker>,
    /// Marker plus the spaces after it.
    marker_width: usize,
    task: Option<(usize, bool)>,
    content: usize,
}

fn prefix(line: &str) -> Prefix {
    let chars: Vec<char> = line.chars().collect();
    let mut at = 0;
    loop {
        let mut look = at;
        while look < chars.len() && look - at < 3 && chars[look] == ' ' {
            look += 1;
        }
        if chars.get(look) != Some(&'>') {
            break;
        }
        at = look + 1;
        if chars.get(at) == Some(&' ') {
            at += 1;
        }
    }
    let quote: String = chars[..at].iter().collect();
    let indent_start = at;
    while chars.get(at) == Some(&' ') {
        at += 1;
    }
    let indent = at - indent_start;
    let mut prefix = Prefix {
        quote,
        indent,
        marker: None,
        marker_width: 0,
        task: None,
        content: at,
    };
    let marker_start = at;
    let marker = match chars.get(at) {
        Some(&c @ ('-' | '*' | '+')) => {
            at += 1;
            Marker::Bullet(c)
        }
        Some(c) if c.is_ascii_digit() => {
            let digits: String = chars[at..]
                .iter()
                .take_while(|c| c.is_ascii_digit())
                .collect();
            at += digits.len();
            match (digits.len(), chars.get(at)) {
                (1..=9, Some(&d @ ('.' | ')'))) => {
                    at += 1;
                    Marker::Ordered {
                        number: digits.parse().unwrap_or(1),
                        delimiter: d,
                    }
                }
                _ => return prefix,
            }
        }
        _ => return prefix,
    };
    if chars.get(at) != Some(&' ') {
        return prefix;
    }
    while chars.get(at) == Some(&' ') {
        at += 1;
    }
    prefix.marker = Some(marker);
    prefix.marker_width = at - marker_start;
    if let ['[', mark @ (' ' | 'x' | 'X'), ']', rest @ ..] = &chars[at..]
        && rest.first().is_none_or(|c| *c == ' ')
    {
        prefix.task = Some((at, *mark != ' '));
        at += 3 + usize::from(!rest.is_empty());
    }
    prefix.content = at;
    prefix
}

fn leading_spaces(line: &str) -> usize {
    line.chars().take_while(|c| *c == ' ' || *c == '\t').count()
}

/// Whether a line sits between code fences.
pub fn in_fence(lines: &[String], row: usize) -> bool {
    lines[..row.min(lines.len())]
        .iter()
        .filter(|l| {
            let t = l.trim_start();
            t.starts_with("```") || t.starts_with("~~~")
        })
        .count()
        % 2
        == 1
}

/// `Enter`: continue a list item, task or quote, end an empty one, and keep the indent.
pub fn enter(lines: &[String], (row, col): (usize, usize)) -> Option<Vec<Command>> {
    let line = &lines[row];
    let len = line.chars().count();
    if in_fence(lines, row) {
        let indent: String = line.chars().take(leading_spaces(line).min(col)).collect();
        return (!indent.is_empty()).then(|| vec![Command::Newline, Command::Text(indent)]);
    }
    let p = prefix(line);
    if p.quote.is_empty() && p.marker.is_none() {
        let indent: String = line.chars().take(leading_spaces(line).min(col)).collect();
        return (!indent.is_empty()).then(|| vec![Command::Newline, Command::Text(indent)]);
    }
    if col < p.content {
        return None;
    }
    if line.chars().skip(p.content).all(char::is_whitespace) {
        let keep = match p.marker {
            Some(_) => p.quote.clone(),
            None => {
                let trimmed = p.quote.trim_end().trim_end_matches('>');
                trimmed.trim_end().to_string()
            }
        };
        let keep = if keep.is_empty() {
            keep
        } else {
            format!("{} ", keep.trim_end())
        };
        return Some(vec![
            Command::Jump(row, 0),
            Command::DeleteForward(len),
            Command::Text(keep),
        ]);
    }
    let mut next = format!("{}{}", p.quote, " ".repeat(p.indent));
    match &p.marker {
        Some(Marker::Bullet(c)) => next.push(*c),
        Some(Marker::Ordered { number, delimiter }) => {
            next.push_str(&format!("{}{delimiter}", number + 1));
        }
        None => {}
    }
    if p.marker.is_some() {
        next.push(' ');
    }
    if p.task.is_some() {
        next.push_str("[ ] ");
    }
    let rest = line.chars().skip(col).take_while(|c| *c == ' ').count();
    let mut commands = vec![Command::Newline];
    if rest > 0 {
        commands.push(Command::DeleteForward(rest));
    }
    commands.push(Command::Text(next));
    Some(commands)
}

/// `Tab` and `Shift-Tab` on a list item: nest it one level deeper or shallower.
pub fn indent(lines: &[String], (row, col): (usize, usize), outdent: bool) -> Option<Vec<Command>> {
    let line = &lines[row];
    let p = prefix(line);
    let at = p.quote.chars().count();
    let width = match p.marker {
        Some(_) => p.marker_width.clamp(2, 4),
        None if outdent => 4,
        None => return None,
    };
    if !outdent {
        return Some(vec![
            Command::Jump(row, at),
            Command::Text(" ".repeat(width)),
            Command::Jump(row, col + width),
        ]);
    }
    let n = width.min(p.indent);
    if n == 0 {
        return Some(Vec::new());
    }
    Some(vec![
        Command::Jump(row, at),
        Command::DeleteForward(n),
        Command::Jump(row, col.saturating_sub(n).max(at)),
    ])
}

/// Checks or unchecks a task, makes a list item a task, or a plain line a task item.
pub fn toggle_task(lines: &[String], (row, col): (usize, usize)) -> Vec<Command> {
    let p = prefix(&lines[row]);
    if let Some((at, done)) = p.task {
        return vec![
            Command::Jump(row, at + 1),
            Command::DeleteForward(1),
            Command::Insert(if done { ' ' } else { 'x' }),
            Command::Jump(row, col),
        ];
    }
    let (at, text) = match p.marker {
        Some(_) => (p.content, "[ ] "),
        None => (p.quote.chars().count() + p.indent, "- [ ] "),
    };
    let shift = if col >= at { text.len() } else { 0 };
    vec![
        Command::Jump(row, at),
        Command::Text(text.to_string()),
        Command::Jump(row, col + shift),
    ]
}

/// Typing a backtick or an asterisk: step over the closing one, or insert both and stand
/// between them.
pub fn pair(lines: &[String], (row, col): (usize, usize), c: char) -> Option<Vec<Command>> {
    if !PAIRS.contains(&c) || in_fence(lines, row) {
        return None;
    }
    let chars: Vec<char> = lines[row].chars().collect();
    let prev = col.checked_sub(1).and_then(|i| chars.get(i)).copied();
    let next = chars.get(col).copied();
    if next == Some(c) && prev.is_some_and(|p| !p.is_whitespace()) {
        return Some(vec![Command::Move(Motion::Right, false)]);
    }
    if c == '*' && chars[..col].iter().all(|c| *c == ' ' || *c == '>') {
        return None;
    }
    let opens = prev.is_none_or(|p| p.is_whitespace() || "([{".contains(p));
    let closes = next.is_none_or(|n| n.is_whitespace() || ")]}.,;:!?".contains(n));
    (opens && closes).then(|| {
        vec![
            Command::Text(format!("{c}{c}")),
            Command::Move(Motion::Left, false),
        ]
    })
}

/// Wraps the selection in a pair: `*word*`, `` `code` ``.
pub fn wrap(
    ((r0, c0), (r1, c1)): ((usize, usize), (usize, usize)),
    c: char,
) -> Option<Vec<Command>> {
    PAIRS.contains(&c).then(|| {
        vec![
            Command::Deselect,
            Command::Jump(r1, c1),
            Command::Insert(c),
            Command::Jump(r0, c0),
            Command::Insert(c),
            Command::Jump(r1, c1 + if r0 == r1 { 2 } else { 1 }),
        ]
    })
}

/// `Backspace` inside an empty pair removes both halves.
pub fn unpair(lines: &[String], (row, col): (usize, usize)) -> Option<Vec<Command>> {
    let chars: Vec<char> = lines[row].chars().collect();
    let prev = chars.get(col.checked_sub(1)?)?;
    (PAIRS.contains(prev) && chars.get(col) == Some(prev))
        .then(|| vec![Command::Delete, Command::Backspace])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::edit::{Editor, TextAreaEditor};

    fn apply(
        text: &str,
        cursor: (usize, usize),
        f: impl Fn(&[String], (usize, usize)) -> Option<Vec<Command>>,
    ) -> (String, (usize, usize)) {
        let mut editor = TextAreaEditor::new(text.split('\n').map(str::to_string).collect());
        editor.run(Command::Jump(cursor.0, cursor.1));
        match f(editor.lines(), editor.cursor()) {
            Some(commands) => editor.group(commands),
            None => editor.run(Command::Newline),
        }
        (editor.text(), editor.cursor())
    }

    #[test]
    fn prefixes_are_read() {
        let p = prefix("> - [x] done");
        assert_eq!(p.quote, "> ");
        assert_eq!(p.marker, Some(Marker::Bullet('-')));
        assert_eq!(p.task, Some((4, true)));
        assert_eq!(p.content, 8);
        let p = prefix("   12) twelve");
        assert_eq!(p.indent, 3);
        assert_eq!(
            p.marker,
            Some(Marker::Ordered {
                number: 12,
                delimiter: ')'
            })
        );
        assert_eq!(p.marker_width, 4);
        assert_eq!(prefix("-not a list").marker, None);
        assert!(prefix("2024. a year").marker.is_some());
        assert_eq!(prefix("1234567890. too long").marker, None);
    }

    #[test]
    fn enter_continues_lists_tasks_and_quotes() {
        assert_eq!(apply("- one", (0, 5), enter), ("- one\n- ".into(), (1, 2)));
        assert_eq!(
            apply("  * one", (0, 7), enter),
            ("  * one\n  * ".into(), (1, 4))
        );
        assert_eq!(
            apply("9. nine", (0, 7), enter),
            ("9. nine\n10. ".into(), (1, 4))
        );
        assert_eq!(
            apply("- [x] done", (0, 10), enter),
            ("- [x] done\n- [ ] ".into(), (1, 6))
        );
        assert_eq!(
            apply("> quoted", (0, 8), enter),
            ("> quoted\n> ".into(), (1, 2))
        );
        assert_eq!(
            apply("> - item", (0, 8), enter),
            ("> - item\n> - ".into(), (1, 4))
        );
        assert_eq!(
            apply("- one two", (0, 5), enter),
            ("- one\n- two".into(), (1, 2))
        );
    }

    #[test]
    fn enter_ends_an_empty_item_and_keeps_indent() {
        assert_eq!(
            apply("- one\n- ", (1, 2), enter),
            ("- one\n".into(), (1, 0))
        );
        assert_eq!(apply("> - ", (0, 4), enter), ("> ".into(), (0, 2)));
        assert_eq!(apply("> ", (0, 2), enter), ("".into(), (0, 0)));
        assert_eq!(
            apply("    code", (0, 8), enter),
            ("    code\n    ".into(), (1, 4))
        );
        assert_eq!(apply("- one", (0, 1), enter), ("-\n one".into(), (1, 0)));
        assert_eq!(
            apply("```\n- x\n```", (1, 3), enter),
            ("```\n- x\n\n```".into(), (2, 0))
        );
    }

    #[test]
    fn tab_nests_list_items() {
        let tab = |l: &[String], c| indent(l, c, false);
        let untab = |l: &[String], c| indent(l, c, true);
        assert_eq!(apply("- one", (0, 5), tab), ("  - one".into(), (0, 7)));
        assert_eq!(apply("1. one", (0, 6), tab), ("   1. one".into(), (0, 9)));
        assert_eq!(apply("  - one", (0, 7), untab), ("- one".into(), (0, 5)));
        assert_eq!(apply("- one", (0, 5), untab), ("- one".into(), (0, 5)));
        assert!(indent(&["text".to_string()], (0, 0), false).is_none());
    }

    #[test]
    fn tasks_toggle() {
        let toggle = |l: &[String], c| Some(toggle_task(l, c));
        assert_eq!(apply("- [ ] a", (0, 7), toggle), ("- [x] a".into(), (0, 7)));
        assert_eq!(apply("- [x] a", (0, 3), toggle), ("- [ ] a".into(), (0, 3)));
        assert_eq!(apply("- a", (0, 3), toggle), ("- [ ] a".into(), (0, 7)));
        assert_eq!(apply("  a", (0, 3), toggle), ("  - [ ] a".into(), (0, 9)));
    }

    #[test]
    fn pairs_open_close_and_unpair() {
        let tick = |l: &[String], c| pair(l, c, '`');
        let star = |l: &[String], c| pair(l, c, '*');
        assert_eq!(apply("say ", (0, 4), tick), ("say ``".into(), (0, 5)));
        assert_eq!(apply("a `b`", (0, 4), tick), ("a `b`".into(), (0, 5)));
        assert_eq!(apply("", (0, 0), star).0, "\n");
        assert_eq!(apply("word", (0, 4), star).0, "word\n");
        assert_eq!(apply("x ``", (0, 3), unpair), ("x ".into(), (0, 2)));
        assert_eq!(
            apply("a word", (0, 0), |_, _| wrap(((0, 2), (0, 6)), '*')),
            ("a *word*".into(), (0, 8))
        );
    }
}
