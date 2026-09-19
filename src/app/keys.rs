pub struct Section {
    pub title: &'static str,
    pub bindings: &'static [(&'static str, &'static str)],
}

pub const SECTIONS: &[Section] = &[
    Section {
        title: "Reading",
        bindings: &[
            ("`j` / `k`, `↓` / `↑`, wheel", "Scroll one line"),
            ("`Ctrl-d` / `Ctrl-u`", "Half page down / up"),
            ("`Ctrl-f` / `Ctrl-b`, `Space`", "Page down / up"),
            ("`g` / `G`", "Top / bottom"),
            ("`t`", "Table of contents"),
            ("`i`", "Show or hide images"),
            ("`m`", "Expand or collapse the front matter"),
            ("`r`", "Reload the file"),
        ],
    },
    Section {
        title: "Panels and focus",
        bindings: &[
            ("`b` / `o`", "Toggle the files / outline panel"),
            ("`f`", "Focus mode: hide the panels, full width"),
            ("`Tab` / `Shift-Tab`", "Cycle focus between the panes"),
            ("`Tab` then `←` / `→`", "Move focus in that direction"),
            ("`l`", "Focus the pane to the right"),
            ("`j` / `k` in a panel", "Move, Enter opens or returns"),
            (
                "`Space`, `←` / `→` in a panel",
                "Fold, collapse / expand a folder or section",
            ),
            ("`-` / `=` in a panel", "Fold all / unfold all"),
            (
                "`T` in the files panel",
                "Show titles instead of file names",
            ),
        ],
    },
    Section {
        title: "Links and history",
        bindings: &[
            ("`]` / `[`", "Select the next / previous link"),
            (
                "`Enter`, click",
                "Follow the selected link or open a footnote",
            ),
            ("`H` / `L`", "Back / forward through visited pages"),
            ("`Ctrl-p`", "Find a file in the folder"),
        ],
    },
    Section {
        title: "Search and editing",
        bindings: &[
            ("`/`", "Search, Enter to keep, Esc to cancel"),
            ("`n` / `N`", "Next / previous match"),
            ("`E`", "Edit the file in $EDITOR, reload on return"),
        ],
    },
    Section {
        title: "Help and quit",
        bindings: &[("`h`, `?`", "This help"), ("`q`", "Quit")],
    },
];

pub fn plain(keys: &str) -> String {
    keys.replace('`', "")
}

pub fn help_rows() -> usize {
    SECTIONS
        .iter()
        .map(|s| s.bindings.len() + 2)
        .sum::<usize>()
        .saturating_sub(1)
}

pub fn markdown() -> String {
    let mut out = String::new();
    for section in SECTIONS {
        out.push_str("## ");
        out.push_str(section.title);
        out.push_str("\n\n| Key | Action |\n| --- | --- |\n");
        for (keys, action) in section.bindings {
            out.push_str(&format!("| {keys} | {action} |\n"));
        }
        out.push('\n');
    }
    out.pop();
    out
}
