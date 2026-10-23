use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use crate::text::{self, Encoding};

const BOM: &str = "\u{feff}";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Eol {
    Lf,
    CrLf,
}

impl Eol {
    fn as_str(self) -> &'static str {
        match self {
            Eol::Lf => "\n",
            Eol::CrLf => "\r\n",
        }
    }
}

/// How a file's bytes were laid out, so saving an unedited buffer writes them back unchanged.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Format {
    pub eol: Eol,
    pub bom: bool,
    pub trailing_newline: bool,
    /// Both line endings were found. The file is saved with the more common one.
    pub mixed: bool,
    pub encoding: Encoding,
}

impl Default for Format {
    fn default() -> Self {
        Self {
            eol: Eol::Lf,
            bom: false,
            trailing_newline: true,
            mixed: false,
            encoding: Encoding::Utf8,
        }
    }
}

pub fn decode(text: &str) -> (Format, Vec<String>) {
    let (bom, text) = match text.strip_prefix(BOM) {
        Some(rest) => (true, rest),
        None => (false, text),
    };
    let trailing_newline = text.ends_with('\n');
    let body = text.strip_suffix('\n').unwrap_or(text);
    let mut crlf = 0;
    let mut lf = 0;
    let mut lines: Vec<String> = body.split('\n').map(str::to_string).collect();
    let breaks = if trailing_newline {
        lines.len()
    } else {
        lines.len() - 1
    };
    for line in &lines[..breaks] {
        if line.ends_with('\r') {
            crlf += 1;
        } else {
            lf += 1;
        }
    }
    for line in &mut lines[..breaks] {
        if line.ends_with('\r') {
            line.pop();
        }
    }
    let format = Format {
        eol: if crlf > lf { Eol::CrLf } else { Eol::Lf },
        bom,
        trailing_newline,
        mixed: crlf > 0 && lf > 0,
        encoding: Encoding::Utf8,
    };
    (format, lines)
}

pub fn encode(format: Format, lines: &[String]) -> String {
    let mut out = String::new();
    if format.bom {
        out.push_str(BOM);
    }
    out.push_str(&lines.join(format.eol.as_str()));
    if format.trailing_newline {
        out.push_str(format.eol.as_str());
    }
    out
}

/// The file behind an editor buffer: where it lives, how it was laid out, and what was on
/// disk the last time mido read or wrote it.
#[derive(Debug, Clone)]
pub struct EditFile {
    pub path: PathBuf,
    pub format: Format,
    disk: Option<Vec<u8>>,
}

impl EditFile {
    /// Reads the file, or starts an empty one when the path does not exist yet. A file that
    /// does not decode cleanly is refused, since saving it would change bytes nobody edited.
    pub fn open(path: &Path) -> io::Result<(Self, Vec<String>)> {
        match fs::read(path) {
            Ok(bytes) => {
                let text = text::decode(&bytes)?;
                if text.lossy {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "not valid UTF-8, so saving could lose bytes",
                    ));
                }
                let (mut format, lines) = decode(&text);
                format.encoding = text.encoding;
                let file = Self {
                    path: path.to_path_buf(),
                    format,
                    disk: Some(bytes),
                };
                Ok((file, lines))
            }
            Err(err) if err.kind() == io::ErrorKind::NotFound => Ok((
                Self {
                    path: path.to_path_buf(),
                    format: Format::default(),
                    disk: None,
                },
                vec![String::new()],
            )),
            Err(err) => Err(err),
        }
    }

    pub fn exists(&self) -> bool {
        self.disk.is_some()
    }

    /// Whether the file on disk differs from what mido last read or wrote.
    pub fn changed_on_disk(&self) -> bool {
        match fs::read(&self.path) {
            Ok(bytes) => self.disk.as_deref() != Some(bytes.as_slice()),
            Err(_) => self.disk.is_some(),
        }
    }

    pub fn save(&mut self, lines: &[String]) -> io::Result<()> {
        let bytes = text::encode(&encode(self.format, lines), self.format.encoding);
        let target = fs::canonicalize(&self.path).unwrap_or_else(|_| self.path.clone());
        write_atomic(&target, &bytes)?;
        self.format.mixed = false;
        self.disk = Some(bytes);
        Ok(())
    }

    /// Where unsaved work goes if mido panics while editing.
    pub fn recovery_path(&self) -> PathBuf {
        sibling(&self.path, "mido-recover")
    }
}

fn sibling(path: &Path, suffix: &str) -> PathBuf {
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    path.with_file_name(format!(".{name}.{suffix}"))
}

/// Writes to a temporary file beside the target, flushes it, and renames it over the target,
/// so a crash leaves either the old file or the new one, never half of each.
pub fn write_atomic(path: &Path, bytes: &[u8]) -> io::Result<()> {
    if let Some(dir) = path.parent().filter(|d| !d.as_os_str().is_empty()) {
        fs::create_dir_all(dir)?;
    }
    let tmp = sibling(path, &format!("mido-{}.tmp", std::process::id()));
    let _ = fs::remove_file(&tmp);
    let result = (|| {
        let mut file = OpenOptions::new().write(true).create_new(true).open(&tmp)?;
        file.write_all(bytes)?;
        file.sync_all()?;
        if let Ok(meta) = fs::metadata(path) {
            fs::set_permissions(&tmp, meta.permissions())?;
        }
        fs::rename(&tmp, path)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&tmp);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn round_trip(text: &str) {
        let (format, lines) = decode(text);
        assert_eq!(encode(format, &lines), text, "{text:?}");
    }

    #[test]
    fn decoding_and_encoding_round_trips() {
        for text in [
            "",
            "\n",
            "\n\n",
            "one",
            "one\n",
            "one\ntwo",
            "one\ntwo\n",
            "one\n\n",
            "one\r\ntwo\r\n",
            "one\r\ntwo",
            "\u{feff}# Title\n",
            "\u{feff}",
            "tab\there\n",
            "trailing spaces  \n",
        ] {
            round_trip(text);
        }
    }

    #[test]
    fn line_endings_are_detected() {
        let (format, lines) = decode("a\r\nb\r\n");
        assert_eq!(format.eol, Eol::CrLf);
        assert_eq!(lines, ["a", "b"]);
        assert!(!format.mixed);
        let (format, lines) = decode("a\r\nb\nc\r\n");
        assert!(format.mixed);
        assert_eq!(format.eol, Eol::CrLf);
        assert_eq!(lines, ["a", "b", "c"]);
        assert_eq!(encode(format, &lines), "a\r\nb\r\nc\r\n");
        let (format, lines) = decode("a\nb\nc\r\n");
        assert_eq!(format.eol, Eol::Lf);
        assert_eq!(lines, ["a", "b", "c"]);
    }

    #[test]
    fn saving_is_atomic_and_keeps_the_layout() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("note.md");
        fs::write(&path, "one\r\ntwo").unwrap();
        let (mut file, mut lines) = EditFile::open(&path).unwrap();
        assert!(file.exists());
        lines.push("three".to_string());
        file.save(&lines).unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), "one\r\ntwo\r\nthree");
        let leftovers: Vec<_> = fs::read_dir(dir.path())
            .unwrap()
            .map(|e| e.unwrap().file_name())
            .collect();
        assert_eq!(leftovers, ["note.md"]);
    }

    #[test]
    fn a_missing_file_is_created_on_save() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("sub/new.md");
        let (mut file, lines) = EditFile::open(&path).unwrap();
        assert!(!file.exists());
        assert_eq!(lines, [""]);
        assert!(!file.changed_on_disk());
        file.save(&["# New".to_string()]).unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), "# New\n");
        assert!(file.exists());
    }

    #[test]
    fn changes_on_disk_are_noticed() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("note.md");
        fs::write(&path, "one\n").unwrap();
        let (mut file, lines) = EditFile::open(&path).unwrap();
        assert!(!file.changed_on_disk());
        fs::write(&path, "changed\n").unwrap();
        assert!(file.changed_on_disk());
        file.save(&lines).unwrap();
        assert!(!file.changed_on_disk());
    }

    #[cfg(unix)]
    #[test]
    fn saving_through_a_symlink_writes_the_target() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("real.md");
        let link = dir.path().join("link.md");
        fs::write(&target, "a\n").unwrap();
        std::os::unix::fs::symlink(&target, &link).unwrap();
        let (mut file, _) = EditFile::open(&link).unwrap();
        file.save(&["b".to_string()]).unwrap();
        assert!(
            fs::symlink_metadata(&link)
                .unwrap()
                .file_type()
                .is_symlink()
        );
        assert_eq!(fs::read_to_string(&target).unwrap(), "b\n");
    }

    #[test]
    fn utf16_saves_back_byte_for_byte_and_invalid_utf8_is_not_edited() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("wide.md");
        let bytes = text::encode("\u{feff}# Wide\r\n\r\nCafé\r\n", Encoding::Utf16Le);
        fs::write(&path, &bytes).unwrap();
        let (mut file, lines) = EditFile::open(&path).unwrap();
        assert_eq!(file.format.encoding, Encoding::Utf16Le);
        assert_eq!(lines, ["# Wide", "", "Café"]);
        file.save(&lines).unwrap();
        assert_eq!(fs::read(&path).unwrap(), bytes);
        assert!(!file.changed_on_disk());

        let latin1 = dir.path().join("latin1.md");
        fs::write(&latin1, b"caf\xe9\n").unwrap();
        let err = EditFile::open(&latin1).unwrap_err();
        assert!(err.to_string().contains("not valid UTF-8"));
    }
}
