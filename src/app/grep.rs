//! Search across every Markdown file in the folder. It matches the source line by line,
//! smart-case like `/`, and reads the files once when it opens so each keystroke only scans
//! memory.

use crate::project::{Project, display_path};
use crate::text;

const MAX_HITS: usize = 200;
const CONTEXT: usize = 24;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hit {
    pub entry: usize,
    /// Byte offset in the file where the matching line starts.
    pub offset: usize,
    pub label: String,
    pub indices: Vec<u32>,
}

#[derive(Default)]
pub struct Grep {
    pub query: String,
    pub hits: Vec<Hit>,
    pub selected: usize,
    /// More matches than `MAX_HITS` were found.
    pub more: bool,
    pub files: usize,
    texts: Vec<(usize, String)>,
}

impl Grep {
    pub fn open(&mut self, project: &Project) {
        self.texts = project
            .entries
            .iter()
            .enumerate()
            .filter(|(_, e)| !e.is_dir)
            .filter_map(|(i, e)| Some((i, text::read(&project.absolute(&e.path)).ok()?.text)))
            .collect();
        self.query.clear();
        self.search(project);
    }

    pub fn search(&mut self, project: &Project) {
        self.hits.clear();
        self.selected = 0;
        self.more = false;
        self.files = 0;
        if self.query.is_empty() {
            return;
        }
        let insensitive = !self.query.chars().any(char::is_uppercase);
        let needle: Vec<char> = self.query.chars().map(|c| fold(c, insensitive)).collect();
        for (entry, text) in &self.texts {
            let path = display_path(&project.entries[*entry].path);
            let mut found = false;
            let mut offset = 0;
            for (number, line) in text.split('\n').enumerate() {
                let start = offset;
                offset += line.len() + 1;
                let chars: Vec<char> = line.trim_end_matches('\r').chars().collect();
                let Some(at) = find(&chars, &needle, insensitive) else {
                    continue;
                };
                found = true;
                if self.hits.len() == MAX_HITS {
                    self.more = true;
                    continue;
                }
                self.hits
                    .push(hit(*entry, start, &path, number, &chars, at, needle.len()));
            }
            self.files += usize::from(found);
        }
    }

    pub fn step(&mut self, delta: isize) {
        if self.hits.is_empty() {
            return;
        }
        let last = self.hits.len() - 1;
        self.selected = self.selected.saturating_add_signed(delta).min(last);
    }

    pub fn chosen(&self) -> Option<&Hit> {
        self.hits.get(self.selected)
    }

    pub fn summary(&self) -> String {
        let count =
            |n: usize, one: &str, many: &str| format!("{n} {}", if n == 1 { one } else { many });
        format!(
            "{}{} in {}",
            count(self.hits.len(), "match", "matches"),
            if self.more { "+" } else { "" },
            count(self.files, "file", "files")
        )
    }
}

/// The label for a matching line, `path:line  text`, with the text cut to keep the match in
/// view, and the character positions of the match in it.
fn hit(
    entry: usize,
    offset: usize,
    path: &str,
    number: usize,
    chars: &[char],
    at: usize,
    len: usize,
) -> Hit {
    let lead = chars.iter().take_while(|c| c.is_whitespace()).count();
    let from = if at > lead + CONTEXT {
        at - CONTEXT
    } else {
        lead
    };
    let prefix = format!(
        "{path}:{}  {}",
        number + 1,
        if from > lead { "…" } else { "" }
    );
    let shift = prefix.chars().count() + at - from;
    Hit {
        entry,
        offset,
        label: prefix + &chars[from..].iter().collect::<String>(),
        indices: (shift..shift + len).map(|i| i as u32).collect(),
    }
}

fn find(chars: &[char], needle: &[char], insensitive: bool) -> Option<usize> {
    (0..=chars.len().checked_sub(needle.len())?).find(|&i| {
        chars[i..i + needle.len()]
            .iter()
            .map(|&c| fold(c, insensitive))
            .eq(needle.iter().copied())
    })
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

    #[test]
    fn finds_lines_across_files_with_smart_case() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("guide")).unwrap();
        std::fs::write(
            dir.path().join("README.md"),
            "# Home\n\nInstall with cargo.\n",
        )
        .unwrap();
        std::fs::write(
            dir.path().join("guide/setup.md"),
            "# Setup\r\n\r\n    Run `cargo install mido` and then read the long explanation of Cargo.\r\n",
        )
        .unwrap();
        let project = Project::scan(dir.path());
        let mut grep = Grep::default();
        grep.open(&project);
        assert!(grep.hits.is_empty());

        grep.query = "cargo".into();
        grep.search(&project);
        assert_eq!(grep.summary(), "2 matches in 2 files");
        let labels: Vec<&str> = grep.hits.iter().map(|h| h.label.as_str()).collect();
        assert!(
            labels.contains(&"README.md:3  Install with cargo."),
            "{labels:?}"
        );
        let setup = grep
            .hits
            .iter()
            .find(|h| h.label.starts_with("guide"))
            .unwrap();
        assert_eq!(
            setup.label,
            "guide/setup.md:3  Run `cargo install mido` and then read the long explanation of Cargo."
        );
        assert_eq!(setup.offset, "# Setup\r\n\r\n".len());
        let marked: String = setup
            .indices
            .iter()
            .map(|&i| setup.label.chars().nth(i as usize).unwrap())
            .collect();
        assert_eq!(marked, "cargo");

        grep.query = "Cargo".into();
        grep.search(&project);
        assert_eq!(grep.summary(), "1 match in 1 file");
        let marked: String = grep.hits[0]
            .indices
            .iter()
            .map(|&i| grep.hits[0].label.chars().nth(i as usize).unwrap())
            .collect();
        assert_eq!(marked, "Cargo");
        assert!(grep.hits[0].label.contains("  …"), "{}", grep.hits[0].label);
    }
}
