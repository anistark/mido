use std::collections::HashMap;

pub fn slug(text: &str) -> String {
    let mut out = String::new();
    for c in text.trim().chars() {
        if c.is_alphanumeric() || c == '_' || c == '-' {
            out.extend(c.to_lowercase());
        } else if c == ' ' {
            out.push('-');
        }
    }
    out
}

#[derive(Debug, Default)]
pub struct Slugger {
    seen: HashMap<String, usize>,
}

impl Slugger {
    pub fn unique(&mut self, text: &str) -> String {
        let base = slug(text);
        let count = self.seen.entry(base.clone()).or_insert(0);
        let out = if *count == 0 {
            base.clone()
        } else {
            format!("{base}-{count}")
        };
        *count += 1;
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn github_style_slugs() {
        assert_eq!(slug("Hello, World!"), "hello-world");
        assert_eq!(slug("  Trim  me "), "trim--me");
        assert_eq!(slug("C++ & Rust_2"), "c--rust_2");
        assert_eq!(slug("Ünïcode Ok"), "ünïcode-ok");
    }

    #[test]
    fn duplicates_get_numbered() {
        let mut s = Slugger::default();
        assert_eq!(s.unique("Setup"), "setup");
        assert_eq!(s.unique("Setup"), "setup-1");
        assert_eq!(s.unique("setup"), "setup-2");
    }
}
