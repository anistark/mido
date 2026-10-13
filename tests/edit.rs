use std::fs;
use std::path::{Path, PathBuf};

use mido::edit::file::EditFile;
use mido::edit::{Command, Editor, Motion, TextAreaEditor, assist};
use proptest::prelude::*;

fn corpus() -> Vec<PathBuf> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut files = vec![
        root.join("README.md"),
        root.join("CHANGELOG.md"),
        root.join("AGENTS.md"),
    ];
    for dir in ["docs", "tests/fixtures"] {
        let mut found: Vec<PathBuf> = fs::read_dir(root.join(dir))
            .unwrap()
            .map(|e| e.unwrap().path())
            .filter(|p| p.extension().is_some_and(|e| e == "md"))
            .collect();
        found.sort();
        files.extend(found);
    }
    files
}

#[test]
fn saving_an_unedited_file_writes_the_same_bytes() {
    let dir = tempfile::tempdir().unwrap();
    let files = corpus();
    assert!(files.len() > 10, "the corpus went missing");
    for source in files {
        let original = fs::read(&source).unwrap();
        let copy = dir.path().join(source.file_name().unwrap());
        fs::write(&copy, &original).unwrap();
        let (mut file, lines) = EditFile::open(&copy).unwrap();
        let editor = TextAreaEditor::new(lines);
        file.save(editor.lines()).unwrap();
        assert!(
            fs::read(&copy).unwrap() == original,
            "{} changed on a plain save",
            source.display()
        );
    }
}

#[derive(Debug, Clone)]
enum Op {
    Run(Command),
    Enter,
    Tab(bool),
    Task,
}

fn motion() -> impl Strategy<Value = Motion> {
    prop_oneof![
        Just(Motion::Left),
        Just(Motion::Right),
        Just(Motion::Up),
        Just(Motion::Down),
        Just(Motion::WordLeft),
        Just(Motion::WordRight),
        Just(Motion::LineStart),
        Just(Motion::LineEnd),
        Just(Motion::Top),
        Just(Motion::Bottom),
    ]
}

fn op() -> impl Strategy<Value = Op> {
    let command = prop_oneof![
        4 => prop::sample::select(vec!['a', 'b', ' ', '-', '*', '`', 'é', '界']).prop_map(Command::Insert),
        1 => "[a-c \\n-]{0,6}".prop_map(Command::Text),
        2 => Just(Command::Newline),
        2 => Just(Command::Backspace),
        1 => Just(Command::Delete),
        1 => (0usize..4).prop_map(Command::DeleteForward),
        1 => Just(Command::DeleteWordBack),
        1 => Just(Command::DeleteWordForward),
        3 => (motion(), any::<bool>()).prop_map(|(m, s)| Command::Move(m, s)),
        1 => (0usize..6, 0usize..12).prop_map(|(r, c)| Command::Jump(r, c)),
        1 => Just(Command::SelectAll),
        1 => Just(Command::Cut),
        1 => Just(Command::Copy),
        1 => Just(Command::Paste),
        2 => Just(Command::Undo),
        1 => Just(Command::Redo),
    ];
    prop_oneof![
        8 => command.prop_map(Op::Run),
        1 => Just(Op::Enter),
        1 => any::<bool>().prop_map(Op::Tab),
        1 => Just(Op::Task),
    ]
}

fn perform(editor: &mut TextAreaEditor, op: &Op) {
    let cursor = editor.cursor();
    let assisted = match op {
        Op::Run(command) => return editor.run(command.clone()),
        Op::Enter => assist::enter(editor.lines(), cursor),
        Op::Tab(out) => assist::indent(editor.lines(), cursor, *out),
        Op::Task => Some(assist::toggle_task(editor.lines(), cursor)),
    };
    match assisted {
        Some(commands) => editor.group(commands),
        None => editor.run(Command::Newline),
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(400))]

    #[test]
    fn undo_reaches_the_start_and_redo_the_end(
        start in "[a-c -]{0,8}(\\n[a-c -]{0,8}){0,3}",
        ops in prop::collection::vec(op(), 0..40),
    ) {
        let lines: Vec<String> = start.split('\n').map(str::to_string).collect();
        let mut editor = TextAreaEditor::new(lines);
        for op in &ops {
            perform(&mut editor, op);
        }
        for _ in 0..=ops.len() {
            editor.run(Command::Redo);
        }
        let tip = editor.text();
        for _ in 0..=ops.len() {
            editor.run(Command::Undo);
        }
        prop_assert_eq!(editor.text(), start);
        for _ in 0..=ops.len() {
            editor.run(Command::Redo);
        }
        prop_assert_eq!(editor.text(), tip);
    }

    #[test]
    fn undo_then_redo_changes_nothing(
        ops in prop::collection::vec(op(), 1..30),
    ) {
        let mut editor = TextAreaEditor::new(vec!["- one".to_string(), "two".to_string()]);
        for op in &ops {
            perform(&mut editor, op);
            let here = editor.text();
            let version = editor.version();
            editor.run(Command::Undo);
            if editor.version() != version {
                editor.run(Command::Redo);
            }
            prop_assert_eq!(editor.text(), here);
        }
    }
}

#[test]
fn typing_undoes_a_word_at_a_time() {
    let mut editor = TextAreaEditor::new(vec![String::new()]);
    for c in "hello big world".chars() {
        editor.run(Command::Insert(c));
    }
    editor.run(Command::Undo);
    assert_eq!(editor.text(), "hello big");
    editor.run(Command::Undo);
    assert_eq!(editor.text(), "hello");
    editor.run(Command::Redo);
    assert_eq!(editor.text(), "hello big");
    editor.run(Command::Backspace);
    editor.run(Command::Backspace);
    editor.run(Command::Undo);
    assert_eq!(editor.text(), "hello big");
}

#[test]
fn an_assist_undoes_in_one_step() {
    let mut editor = TextAreaEditor::new(vec!["- one".to_string()]);
    editor.run(Command::Move(Motion::LineEnd, false));
    let commands = assist::enter(editor.lines(), editor.cursor()).unwrap();
    editor.group(commands);
    assert_eq!(editor.text(), "- one\n- ");
    editor.run(Command::Undo);
    assert_eq!(editor.text(), "- one");
    assert_eq!(editor.cursor(), (0, 5));
}
