use std::process::{Command, Output};

fn mido(args: &[&str]) -> Output {
    let config = tempfile::tempdir().unwrap();
    Command::new(env!("CARGO_BIN_EXE_mido"))
        .args(args)
        .env("MIDO_CONFIG_DIR", config.path())
        .env("COLORTERM", "truecolor")
        .env_remove("NO_COLOR")
        .output()
        .unwrap()
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

#[test]
fn completions_print_a_script_for_each_shell() {
    for (shell, marker) in [
        ("bash", "_mido()"),
        ("zsh", "#compdef mido"),
        ("fish", "complete -c mido"),
        ("elvish", "edit:completion:arg-completer[mido]"),
        ("powershell", "Register-ArgumentCompleter"),
    ] {
        let out = mido(&["completions", shell]);
        assert!(out.status.success(), "{shell}");
        let script = stdout(&out);
        assert!(script.contains(marker), "{shell} script lacks {marker}");
        assert!(
            script.contains("no-color"),
            "{shell} script lacks the flags"
        );
    }
}

#[test]
fn completions_name_the_shells_they_know() {
    let out = mido(&["completions"]);
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("bash, zsh, fish, elvish or powershell"));
    let out = mido(&["completions", "tcsh"]);
    assert!(String::from_utf8_lossy(&out.stderr).contains("unknown shell `tcsh`"));
}

#[test]
fn no_color_prints_plain_text() {
    let page = "tests/fixtures/basic.md";
    let colored = stdout(&mido(&["-p", page]));
    assert!(colored.contains('\x1b'), "print mode is styled by default");
    let plain = stdout(&mido(&["-p", "--no-color", page]));
    assert!(!plain.contains('\x1b'));
    assert!(plain.contains("Basic"), "{plain}");
}
