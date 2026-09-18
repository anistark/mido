use std::collections::HashSet;

use crate::render::layout::Heading;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row {
    pub index: usize,
    pub prefix: String,
    pub marker: &'static str,
    pub has_children: bool,
    pub collapsed: bool,
}

#[derive(Debug, Default, Clone)]
pub struct Tree {
    depths: Vec<usize>,
    parents: Vec<Option<usize>>,
}

impl Tree {
    pub fn new(headings: &[Heading]) -> Self {
        let mut stack: Vec<usize> = Vec::new();
        let mut depths = Vec::with_capacity(headings.len());
        let mut parents = Vec::with_capacity(headings.len());
        for (i, h) in headings.iter().enumerate() {
            while stack.last().is_some_and(|&j| headings[j].level >= h.level) {
                stack.pop();
            }
            parents.push(stack.last().copied());
            depths.push(stack.len());
            stack.push(i);
        }
        Self { depths, parents }
    }

    pub fn len(&self) -> usize {
        self.depths.len()
    }

    pub fn parent(&self, i: usize) -> Option<usize> {
        self.parents.get(i).copied().flatten()
    }

    pub fn has_children(&self, i: usize) -> bool {
        self.parents.get(i + 1) == Some(&Some(i))
    }

    pub fn visible_ancestor(&self, i: usize, collapsed: &HashSet<usize>) -> usize {
        let mut shown = i;
        let mut cursor = self.parent(i);
        while let Some(p) = cursor {
            if collapsed.contains(&p) {
                shown = p;
            }
            cursor = self.parent(p);
        }
        shown
    }

    pub fn rows(&self, collapsed: &HashSet<usize>) -> Vec<Row> {
        (0..self.len())
            .filter(|&i| self.visible_ancestor(i, collapsed) == i)
            .map(|i| {
                let depth = self.depths[i];
                let mut prefix = String::new();
                for level in 1..depth {
                    let ancestor = (0..i).rev().find(|&j| self.depths[j] == level);
                    let open = ancestor.is_some_and(|a| self.has_later_sibling(a));
                    prefix.push_str(if open { "│ " } else { "  " });
                }
                if depth > 0 {
                    prefix.push_str(if self.has_later_sibling(i) {
                        "├ "
                    } else {
                        "└ "
                    });
                }
                let has_children = self.has_children(i);
                let is_collapsed = has_children && collapsed.contains(&i);
                let marker = match (has_children, is_collapsed) {
                    (false, _) => "  ",
                    (true, false) => "▾ ",
                    (true, true) => "▸ ",
                };
                Row {
                    index: i,
                    prefix,
                    marker,
                    has_children,
                    collapsed: is_collapsed,
                }
            })
            .collect()
    }

    fn has_later_sibling(&self, i: usize) -> bool {
        self.depths[i + 1..]
            .iter()
            .find(|&&d| d <= self.depths[i])
            .is_some_and(|&d| d == self.depths[i])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn heading(level: u8, text: &str) -> Heading {
        Heading {
            level,
            text: text.to_string(),
            line: 0,
        }
    }

    fn render(headings: &[Heading], collapsed: &HashSet<usize>) -> Vec<String> {
        Tree::new(headings)
            .rows(collapsed)
            .iter()
            .map(|r| format!("{}{}{}", r.prefix, r.marker, headings[r.index].text))
            .collect()
    }

    fn sample() -> Vec<Heading> {
        vec![
            heading(1, "Title"),
            heading(2, "A"),
            heading(3, "A1"),
            heading(3, "A2"),
            heading(2, "B"),
            heading(3, "B1"),
        ]
    }

    #[test]
    fn nested_tree_with_connectors_and_markers() {
        assert_eq!(
            render(&sample(), &HashSet::new()),
            [
                "▾ Title",
                "├ ▾ A",
                "│ ├   A1",
                "│ └   A2",
                "└ ▾ B",
                "  └   B1"
            ]
        );
    }

    #[test]
    fn collapsed_nodes_hide_descendants() {
        let collapsed = HashSet::from([1]);
        assert_eq!(
            render(&sample(), &collapsed),
            ["▾ Title", "├ ▸ A", "└ ▾ B", "  └   B1"]
        );
        let tree = Tree::new(&sample());
        assert_eq!(tree.visible_ancestor(2, &collapsed), 1);
        assert_eq!(tree.visible_ancestor(5, &collapsed), 5);
        assert_eq!(render(&sample(), &HashSet::from([0])), ["▸ Title"]);
    }

    #[test]
    fn skipped_levels_and_multiple_roots() {
        let h = [
            heading(2, "Intro"),
            heading(4, "Deep"),
            heading(2, "Next"),
            heading(1, "Root"),
        ];
        assert_eq!(
            render(&h, &HashSet::new()),
            ["▾ Intro", "└   Deep", "  Next", "  Root"]
        );
        let tree = Tree::new(&h);
        assert_eq!(tree.parent(1), Some(0));
        assert!(!tree.has_children(3));
    }
}
