use nucleo_matcher::pattern::{CaseMatching, Normalization, Pattern};
use nucleo_matcher::{Config, Matcher, Utf32String};

use crate::project::{Project, display_path};

const MAX_RESULTS: usize = 200;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hit {
    pub entry: usize,
    pub label: String,
    pub indices: Vec<u32>,
}

pub struct Finder {
    matcher: Matcher,
    pub query: String,
    pub hits: Vec<Hit>,
    pub selected: usize,
}

impl Finder {
    pub fn new() -> Self {
        let mut config = Config::DEFAULT;
        config.set_match_paths();
        Self {
            matcher: Matcher::new(config),
            query: String::new(),
            hits: Vec::new(),
            selected: 0,
        }
    }

    pub fn open(&mut self, project: &Project) {
        self.query.clear();
        self.selected = 0;
        self.search(project);
    }

    pub fn search(&mut self, project: &Project) {
        let candidates: Vec<(usize, String)> = project
            .entries
            .iter()
            .enumerate()
            .filter(|(_, e)| !e.is_dir)
            .map(|(i, e)| (i, label(&display_path(&e.path), e.title.as_deref())))
            .collect();
        self.hits = if self.query.trim().is_empty() {
            candidates
                .into_iter()
                .take(MAX_RESULTS)
                .map(|(entry, label)| Hit {
                    entry,
                    label,
                    indices: Vec::new(),
                })
                .collect()
        } else {
            let pattern = Pattern::parse(&self.query, CaseMatching::Smart, Normalization::Smart);
            let mut scored: Vec<(u32, usize, String, Vec<u32>)> = Vec::new();
            let mut indices = Vec::new();
            for (entry, label) in candidates {
                let haystack = Utf32String::from(label.as_str());
                indices.clear();
                if let Some(score) =
                    pattern.indices(haystack.slice(..), &mut self.matcher, &mut indices)
                {
                    indices.sort_unstable();
                    indices.dedup();
                    scored.push((score, entry, label, indices.clone()));
                }
            }
            scored.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
            scored
                .into_iter()
                .take(MAX_RESULTS)
                .map(|(_, entry, label, indices)| Hit {
                    entry,
                    label,
                    indices,
                })
                .collect()
        };
        self.selected = self.selected.min(self.hits.len().saturating_sub(1));
    }

    pub fn step(&mut self, delta: isize) {
        if self.hits.is_empty() {
            return;
        }
        let last = self.hits.len() - 1;
        self.selected = self.selected.saturating_add_signed(delta).min(last);
    }

    pub fn chosen(&self) -> Option<usize> {
        self.hits.get(self.selected).map(|h| h.entry)
    }
}

fn label(path: &str, title: Option<&str>) -> String {
    match title {
        Some(t) if !t.is_empty() => format!("{path}  {t}"),
        _ => path.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project::Entry;
    use std::path::PathBuf;

    fn project() -> Project {
        let file = |path: &str, title: Option<&str>| Entry {
            path: PathBuf::from(path),
            name: path.rsplit('/').next().unwrap().to_string(),
            depth: path.matches('/').count(),
            is_dir: false,
            title: title.map(str::to_string),
        };
        Project {
            root: PathBuf::from("/tmp/x"),
            entries: vec![
                Entry {
                    path: PathBuf::from("docs"),
                    name: "docs".into(),
                    depth: 0,
                    is_dir: true,
                    title: None,
                },
                file("docs/getting-started.md", Some("Getting Started")),
                file("docs/reference.md", Some("API Reference")),
                file("README.md", Some("mido")),
                file("CHANGELOG.md", None),
            ],
        }
    }

    #[test]
    fn empty_query_lists_files_in_tree_order() {
        let project = project();
        let mut finder = Finder::new();
        finder.open(&project);
        let labels: Vec<&str> = finder.hits.iter().map(|h| h.label.as_str()).collect();
        assert_eq!(
            labels,
            [
                "docs/getting-started.md  Getting Started",
                "docs/reference.md  API Reference",
                "README.md  mido",
                "CHANGELOG.md"
            ]
        );
    }

    #[test]
    fn fuzzy_query_ranks_and_marks_matches() {
        let project = project();
        let mut finder = Finder::new();
        finder.open(&project);
        finder.query = "apiref".into();
        finder.search(&project);
        assert_eq!(finder.chosen(), Some(2));
        assert!(!finder.hits[0].indices.is_empty());
        finder.query = "gs".into();
        finder.search(&project);
        assert_eq!(finder.hits[0].entry, 1);
        finder.query = "zzzz".into();
        finder.search(&project);
        assert!(finder.hits.is_empty() && finder.chosen().is_none());
    }
}
