use unicode_width::UnicodeWidthStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Text,
    Border,
    Edge,
}

#[derive(Debug, Clone)]
pub struct Drawing {
    pub rows: Vec<Vec<(Kind, String)>>,
}

impl Drawing {
    fn from_text(text: &str, ascii: bool) -> Self {
        let mut rows: Vec<Vec<(Kind, String)>> = text
            .lines()
            .map(|line| {
                let mut runs: Vec<(Kind, String)> = Vec::new();
                for ch in line.trim_end().chars() {
                    let kind = if ascii {
                        classify_ascii(ch)
                    } else {
                        classify(ch)
                    };
                    match runs.last_mut() {
                        Some((k, s)) if *k == kind => s.push(ch),
                        _ => runs.push((kind, ch.to_string())),
                    }
                }
                runs
            })
            .collect();
        while rows.last().is_some_and(Vec::is_empty) {
            rows.pop();
        }
        Self { rows }
    }

    pub fn width(&self) -> usize {
        self.rows
            .iter()
            .map(|r| r.iter().map(|(_, s)| s.width()).sum())
            .max()
            .unwrap_or(0)
    }

    pub fn plain(&self) -> Vec<String> {
        self.rows
            .iter()
            .map(|r| r.iter().map(|(_, s)| s.as_str()).collect())
            .collect()
    }
}

fn classify(ch: char) -> Kind {
    match ch {
        '▲' | '▼' | '◀' | '▶' | '◄' | '►' | '▴' | '▾' | '◂' | '▸' | '△' | '▽' | '◁' | '▷' | '●'
        | '◉' | '○' | '✕' | '◆' | '◇' => Kind::Edge,
        '\u{2500}'..='\u{257F}' => Kind::Border,
        _ => Kind::Text,
    }
}

fn classify_ascii(ch: char) -> Kind {
    match ch {
        '>' | '<' | '^' => Kind::Edge,
        '+' | '-' | '|' | '=' => Kind::Border,
        _ => Kind::Text,
    }
}

pub fn render(source: &str, max_width: usize, ascii: bool) -> Option<Drawing> {
    let format = if ascii {
        mmdflux::OutputFormat::Ascii
    } else {
        mmdflux::OutputFormat::Text
    };
    let text = mmdflux::detect_diagram(source)
        .and_then(|_| {
            mmdflux::render_diagram(source, format, &mmdflux::RenderConfig::default()).ok()
        })
        .or_else(|| {
            if ascii {
                mermaid_text::render_ascii_with_width(source, Some(max_width)).ok()
            } else {
                mermaid_text::render_with_width(source, Some(max_width)).ok()
            }
        })?;
    let drawing = Drawing::from_text(&text, ascii);
    (drawing.width() > 0).then_some(drawing)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plain(source: &str) -> String {
        render(source, 80, false)
            .expect("renders")
            .plain()
            .join("\n")
    }

    #[test]
    fn flowchart_top_down_with_branches_and_labels() {
        insta::assert_snapshot!(plain(
            "graph TD\n  A[Start] --> B{Ready?}\n  B -->|yes| C(Go)\n  B -->|no| D[Wait]\n  D --> B\n  C --> E((Done))\n"
        ));
    }

    #[test]
    fn flowchart_left_right_chain_with_fan_out() {
        insta::assert_snapshot!(plain(
            "flowchart LR\n  In --> Parse --> Layout\n  Layout --> Paint & Print\n  Parse -.-> Cache\n"
        ));
    }

    #[test]
    fn sequence_with_notes_blocks_and_numbers() {
        insta::assert_snapshot!(plain(
            "sequenceDiagram\n  autonumber\n  participant A as Alice\n  participant B as Bob\n  A->>B: Hello Bob, how are you?\n  Note right of B: Bob thinks\n  loop every minute\n    B-->>A: Great!\n    A-xB: Ping\n  end\n  Note over A,B: The end\n"
        ));
    }

    #[test]
    fn class_state_and_pie_render_too() {
        insta::assert_snapshot!(
            "class",
            plain(
                "classDiagram\n  class Animal {\n    +String name\n    +speak()\n  }\n  Animal <|-- Dog\n"
            )
        );
        insta::assert_snapshot!(
            "state",
            plain(
                "stateDiagram-v2\n  [*] --> Idle\n  Idle --> Running : start\n  Running --> [*]\n"
            )
        );
        insta::assert_snapshot!(
            "pie",
            plain("pie title Pets\n  \"Dogs\" : 3\n  \"Cats\" : 2\n")
        );
    }

    #[test]
    fn kinds_split_borders_arrows_and_text() {
        let drawing = render("graph LR\n  A[Read] --> B[Render]\n", 80, false).unwrap();
        let kinds: Vec<Kind> = drawing.rows.iter().flatten().map(|(k, _)| *k).collect();
        assert!(
            kinds.contains(&Kind::Border)
                && kinds.contains(&Kind::Edge)
                && kinds.contains(&Kind::Text)
        );
    }

    #[test]
    fn unknown_or_broken_input_falls_back() {
        assert!(render("zzzDiagram\n  a -> b\n", 80, false).is_none());
        assert!(render("", 80, false).is_none());
        assert!(render("graph TD\n", 80, false).is_none());
    }

    #[test]
    fn ascii_drawings_stay_ascii() {
        for (source, framed) in [
            ("graph LR\n  A[Read] --> B[Render]\n", true),
            ("sequenceDiagram\n  A->>B: Hi\n", true),
            ("pie title Pets\n  \"Dogs\" : 3\n", false),
        ] {
            let drawing = render(source, 80, true).expect("renders");
            let text = drawing.plain().join("\n");
            assert!(text.is_ascii(), "{text}");
            let bordered = drawing
                .rows
                .iter()
                .flatten()
                .any(|(k, _)| *k == Kind::Border);
            assert_eq!(bordered, framed, "{text}");
        }
    }
}
