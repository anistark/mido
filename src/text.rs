//! Reading a file as text. Markdown is nearly always UTF-8, but Windows tools write UTF-16
//! with a byte order mark, and an old file may be in a legacy encoding. UTF-16 is decoded
//! and written back the same way, invalid UTF-8 is shown with replacement characters and
//! never saved, and a file with NUL bytes is taken for binary and refused.

use std::io;
use std::ops::Deref;
use std::path::Path;

const BINARY_SCAN: usize = 8192;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Encoding {
    #[default]
    Utf8,
    Utf16Le,
    Utf16Be,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Text {
    pub text: String,
    pub encoding: Encoding,
    /// Some bytes did not decode and were replaced, so the text cannot be saved back.
    pub lossy: bool,
}

impl Deref for Text {
    type Target = str;

    fn deref(&self) -> &str {
        &self.text
    }
}

impl Text {
    /// What to tell the reader about how the file was read, if anything is unusual.
    pub fn notice(&self, name: &str) -> Option<String> {
        if self.lossy {
            Some(format!(
                "{name} is not valid UTF-8, bad bytes show as \u{fffd} and editing is off"
            ))
        } else if self.encoding != Encoding::Utf8 {
            Some(format!("{name} is UTF-16, it is saved back that way"))
        } else {
            None
        }
    }
}

pub fn decode(bytes: &[u8]) -> io::Result<Text> {
    let utf16 = match bytes {
        [0xff, 0xfe, ..] => Some(Encoding::Utf16Le),
        [0xfe, 0xff, ..] => Some(Encoding::Utf16Be),
        _ => None,
    };
    if let Some(encoding) = utf16 {
        let units = bytes.chunks(2).map(|pair| match (pair, encoding) {
            ([a, b], Encoding::Utf16Le) => Ok(u16::from_le_bytes([*a, *b])),
            ([a, b], _) => Ok(u16::from_be_bytes([*a, *b])),
            _ => Err(()),
        });
        let mut lossy = false;
        let mut text = String::with_capacity(bytes.len() / 2);
        for unit in char::decode_utf16(units.map_while(Result::ok)) {
            text.push(unit.unwrap_or_else(|_| {
                lossy = true;
                char::REPLACEMENT_CHARACTER
            }));
        }
        if bytes.len() % 2 == 1 {
            lossy = true;
            text.push(char::REPLACEMENT_CHARACTER);
        }
        return Ok(Text {
            text,
            encoding,
            lossy,
        });
    }
    if bytes[..bytes.len().min(BINARY_SCAN)].contains(&0) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "looks like a binary file, not text",
        ));
    }
    Ok(match String::from_utf8(bytes.to_vec()) {
        Ok(text) => Text {
            text,
            ..Text::default()
        },
        Err(err) => Text {
            text: String::from_utf8_lossy(err.as_bytes()).into_owned(),
            lossy: true,
            ..Text::default()
        },
    })
}

pub fn encode(text: &str, encoding: Encoding) -> Vec<u8> {
    match encoding {
        Encoding::Utf8 => text.as_bytes().to_vec(),
        Encoding::Utf16Le => text.encode_utf16().flat_map(u16::to_le_bytes).collect(),
        Encoding::Utf16Be => text.encode_utf16().flat_map(u16::to_be_bytes).collect(),
    }
}

pub fn read(path: &Path) -> io::Result<Text> {
    decode(&std::fs::read(path)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn utf16(text: &str, encoding: Encoding) -> Vec<u8> {
        encode(&format!("\u{feff}{text}"), encoding)
    }

    #[test]
    fn utf16_with_a_byte_order_mark_round_trips() {
        for encoding in [Encoding::Utf16Le, Encoding::Utf16Be] {
            let bytes = utf16("# Café 日本\r\n\r\nText 👋\r\n", encoding);
            let text = decode(&bytes).unwrap();
            assert_eq!(text.encoding, encoding);
            assert!(!text.lossy);
            assert_eq!(&*text, "\u{feff}# Café 日本\r\n\r\nText 👋\r\n");
            assert_eq!(encode(&text, encoding), bytes);
            assert!(text.notice("a.md").unwrap().contains("UTF-16"));
        }
        let mut odd = utf16("ab", Encoding::Utf16Le);
        odd.push(b'c');
        assert!(decode(&odd).unwrap().lossy);
    }

    #[test]
    fn invalid_utf8_is_replaced_and_binary_is_refused() {
        let latin1 = decode(b"# T\n\ncaf\xe9\n").unwrap();
        assert!(latin1.lossy);
        assert_eq!(&*latin1, "# T\n\ncaf\u{fffd}\n");
        assert!(latin1.notice("a.md").unwrap().contains("not valid UTF-8"));

        let plain = decode("# Fine ✓\n".as_bytes()).unwrap();
        assert!(!plain.lossy && plain.notice("a.md").is_none());

        let err = decode(b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR").unwrap_err();
        assert!(err.to_string().contains("binary"));
    }
}
