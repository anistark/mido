use std::collections::BTreeMap;
use std::path::{MAIN_SEPARATOR, Path, PathBuf};

use ignore::WalkBuilder;

const EXTENSIONS: [&str; 5] = ["md", "markdown", "mdown", "mkd", "mdx"];
const TITLE_SCAN_BYTES: usize = 8 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub path: PathBuf,
    pub name: String,
    pub depth: usize,
    pub is_dir: bool,
    pub title: Option<String>,
}

#[derive(Debug, Default, Clone)]
pub struct Project {
    pub root: PathBuf,
    pub entries: Vec<Entry>,
}

pub fn is_markdown(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| EXTENSIONS.iter().any(|x| x.eq_ignore_ascii_case(e)))
}

pub fn display_path(path: &Path) -> String {
    let text = path.to_string_lossy();
    if MAIN_SEPARATOR == '/' {
        return text.into_owned();
    }
    text.replace(MAIN_SEPARATOR, "/")
}

impl Project {
    pub fn scan(root: &Path) -> Self {
        let root = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
        let mut files: Vec<PathBuf> = WalkBuilder::new(&root)
            .follow_links(false)
            .require_git(false)
            .build()
            .filter_map(Result::ok)
            .filter(|e| e.file_type().is_some_and(|t| t.is_file()))
            .filter(|e| is_markdown(e.path()))
            .filter_map(|e| e.path().strip_prefix(&root).ok().map(Path::to_path_buf))
            .collect();
        files.sort();
        let mut tree = Node::default();
        for file in &files {
            tree.insert(file);
        }
        let name = root
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| root.display().to_string());
        let mut entries = vec![Entry {
            path: PathBuf::new(),
            name,
            depth: 0,
            is_dir: true,
            title: None,
        }];
        tree.flatten(&root, PathBuf::new(), 1, &mut entries);
        Self { root, entries }
    }

    pub fn files(&self) -> impl Iterator<Item = &Entry> {
        self.entries.iter().filter(|e| !e.is_dir)
    }

    pub fn entry_file(&self) -> Option<PathBuf> {
        let at_root = |name: &str| {
            self.files()
                .find(|e| e.depth == 1 && e.name.eq_ignore_ascii_case(name))
                .map(|e| e.path.clone())
        };
        at_root("README.md")
            .or_else(|| at_root("index.md"))
            .or_else(|| self.files().next().map(|e| e.path.clone()))
    }

    pub fn index_of(&self, path: &Path) -> Option<usize> {
        self.entries.iter().position(|e| e.path == path)
    }

    pub fn absolute(&self, path: &Path) -> PathBuf {
        self.root.join(path)
    }
}

#[derive(Default)]
struct Node {
    dirs: BTreeMap<(String, String), Node>,
    files: BTreeMap<(String, String), PathBuf>,
}

fn key(name: &str) -> (String, String) {
    (name.to_lowercase(), name.to_string())
}

impl Node {
    fn insert(&mut self, path: &Path) {
        let mut node = self;
        let mut components = path.components().peekable();
        while let Some(component) = components.next() {
            let name = component.as_os_str().to_string_lossy().into_owned();
            if components.peek().is_some() {
                node = node.dirs.entry(key(&name)).or_default();
            } else {
                node.files.insert(key(&name), path.to_path_buf());
            }
        }
    }

    fn flatten(&self, root: &Path, prefix: PathBuf, depth: usize, out: &mut Vec<Entry>) {
        for ((_, name), child) in &self.dirs {
            let path = prefix.join(name);
            out.push(Entry {
                path: path.clone(),
                name: name.clone(),
                depth,
                is_dir: true,
                title: None,
            });
            child.flatten(root, path, depth + 1, out);
        }
        for ((_, name), path) in &self.files {
            out.push(Entry {
                path: path.clone(),
                name: name.clone(),
                depth,
                is_dir: false,
                title: first_heading(&root.join(path)),
            });
        }
    }
}

fn first_heading(path: &Path) -> Option<String> {
    let bytes = std::fs::read(path).ok()?;
    let head = &bytes[..bytes.len().min(TITLE_SCAN_BYTES)];
    let text = String::from_utf8_lossy(head);
    let mut lines = text.lines().peekable();
    if lines.peek().is_some_and(|l| l.trim_end() == "---") {
        lines.next();
        for line in lines.by_ref() {
            if line.trim_end() == "---" {
                break;
            }
        }
    }
    let mut previous: Option<&str> = None;
    for line in lines {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix('#') {
            let title = rest
                .trim_start_matches('#')
                .trim()
                .trim_end_matches('#')
                .trim();
            if !title.is_empty() {
                return Some(title.to_string());
            }
        }
        if !trimmed.is_empty()
            && trimmed.chars().all(|c| c == '=')
            && let Some(p) = previous.filter(|p| !p.trim().is_empty())
        {
            return Some(p.trim().to_string());
        }
        previous = Some(line);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        std::fs::create_dir_all(root.join("docs/guide")).unwrap();
        std::fs::create_dir_all(root.join("notes")).unwrap();
        std::fs::create_dir_all(root.join("target")).unwrap();
        std::fs::create_dir_all(root.join(".hidden")).unwrap();
        std::fs::write(
            root.join("README.md"),
            "---\ntitle: x\n---\n\n# Hello World\n",
        )
        .unwrap();
        std::fs::write(root.join("zeta.md"), "Setext Title\n============\n").unwrap();
        std::fs::write(root.join("Alpha.MD"), "no heading here\n").unwrap();
        std::fs::write(root.join("docs/guide/install.md"), "## Install\n").unwrap();
        std::fs::write(root.join("docs/intro.markdown"), "# Intro\n").unwrap();
        std::fs::write(root.join("notes/todo.txt"), "not markdown\n").unwrap();
        std::fs::write(root.join("target/ignored.md"), "# Ignored\n").unwrap();
        std::fs::write(root.join(".hidden/secret.md"), "# Secret\n").unwrap();
        std::fs::write(root.join(".gitignore"), "target/\n").unwrap();
        dir
    }

    #[test]
    fn scans_markdown_tree_dirs_first_and_prunes() {
        let dir = fixture();
        let project = Project::scan(dir.path());
        let root = &project.entries[0];
        assert!(root.is_dir && root.depth == 0 && root.path.as_os_str().is_empty());
        assert_eq!(root.name, dir.path().file_name().unwrap().to_string_lossy());
        let listing: Vec<String> = project
            .entries
            .iter()
            .skip(1)
            .map(|e| {
                format!(
                    "{}{}{}",
                    "  ".repeat(e.depth - 1),
                    e.name,
                    if e.is_dir { "/" } else { "" }
                )
            })
            .collect();
        assert_eq!(
            listing,
            [
                "docs/",
                "  guide/",
                "    install.md",
                "  intro.markdown",
                "Alpha.MD",
                "README.md",
                "zeta.md"
            ]
        );
    }

    #[test]
    fn titles_entry_file_and_lookup() {
        let dir = fixture();
        let project = Project::scan(dir.path());
        let title = |name: &str| {
            project
                .files()
                .find(|e| e.name == name)
                .unwrap()
                .title
                .clone()
        };
        assert_eq!(title("README.md").as_deref(), Some("Hello World"));
        assert_eq!(title("zeta.md").as_deref(), Some("Setext Title"));
        assert_eq!(title("install.md").as_deref(), Some("Install"));
        assert_eq!(title("Alpha.MD"), None);
        assert_eq!(project.entry_file(), Some(PathBuf::from("README.md")));
        assert_eq!(project.index_of(Path::new("docs/intro.markdown")), Some(4));
        assert!(is_markdown(Path::new("x.MD")) && !is_markdown(Path::new("x.txt")));
    }

    #[test]
    fn entry_file_falls_back_to_index_then_first() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("b")).unwrap();
        std::fs::write(dir.path().join("b/z.md"), "").unwrap();
        std::fs::write(dir.path().join("Index.md"), "").unwrap();
        assert_eq!(
            Project::scan(dir.path()).entry_file(),
            Some(PathBuf::from("Index.md"))
        );
        std::fs::remove_file(dir.path().join("Index.md")).unwrap();
        assert_eq!(
            Project::scan(dir.path()).entry_file(),
            Some(PathBuf::from("b/z.md"))
        );
        assert_eq!(Project::scan(&dir.path().join("b")).entries.len(), 2);
    }

    #[test]
    fn display_path_uses_forward_slashes() {
        assert_eq!(
            display_path(&Path::new("guide").join("faq.md")),
            "guide/faq.md"
        );
        assert_eq!(display_path(Path::new("README.md")), "README.md");
    }
}
