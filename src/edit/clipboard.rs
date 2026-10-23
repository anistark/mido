//! Copying to the system clipboard through the terminal with OSC 52, which works over SSH and
//! in tmux without a clipboard library. Base64 is a dozen lines, so it is written out here.
//! Some terminals ignore OSC 52, Apple Terminal among them, so on a local session the text
//! also goes to the platform's clipboard command.

use std::io::Write;
use std::process::{Command, Stdio};

const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

pub fn base64(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let n = (u32::from(chunk[0]) << 16)
            | (u32::from(chunk.get(1).copied().unwrap_or(0)) << 8)
            | u32::from(chunk.get(2).copied().unwrap_or(0));
        for i in 0..4 {
            if i <= chunk.len() {
                out.push(ALPHABET[(n >> (18 - 6 * i) & 63) as usize] as char);
            } else {
                out.push('=');
            }
        }
    }
    out
}

/// The escape sequence that puts `text` on the clipboard.
pub fn osc52(text: &str) -> String {
    format!("\x1b]52;c;{}\x07", base64(text.as_bytes()))
}

/// Hands `text` to `pbcopy`, `wl-copy`, `xclip` or `xsel` on a background thread. Over SSH
/// that would fill the remote machine's clipboard, so it only runs on a local session.
pub fn copy_local(text: &str) {
    let remote = ["SSH_CONNECTION", "SSH_TTY"]
        .iter()
        .any(|v| std::env::var_os(v).is_some());
    if remote {
        return;
    }
    let commands: &[&[&str]] = if cfg!(target_os = "macos") {
        &[&["pbcopy"]]
    } else if cfg!(unix) && std::env::var_os("WAYLAND_DISPLAY").is_some() {
        &[&["wl-copy"]]
    } else if cfg!(unix) && std::env::var_os("DISPLAY").is_some() {
        &[
            &["xclip", "-selection", "clipboard"],
            &["xsel", "--clipboard", "--input"],
        ]
    } else {
        &[]
    };
    let text = text.to_string();
    std::thread::spawn(move || {
        for command in commands {
            let child = Command::new(command[0])
                .args(&command[1..])
                .stdin(Stdio::piped())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn();
            let Ok(mut child) = child else { continue };
            if let Some(mut stdin) = child.stdin.take() {
                let _ = stdin.write_all(text.as_bytes());
            }
            if child.wait().is_ok_and(|s| s.success()) {
                return;
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_matches_the_rfc_vectors() {
        for (plain, encoded) in [
            ("", ""),
            ("f", "Zg=="),
            ("fo", "Zm8="),
            ("foo", "Zm9v"),
            ("foob", "Zm9vYg=="),
            ("fooba", "Zm9vYmE="),
            ("foobar", "Zm9vYmFy"),
        ] {
            assert_eq!(base64(plain.as_bytes()), encoded);
        }
        assert_eq!(osc52("hi"), "\x1b]52;c;aGk=\x07");
    }
}
