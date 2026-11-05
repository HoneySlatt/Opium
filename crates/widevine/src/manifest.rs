//! Reading the `version` of a Chromium component's `manifest.json`, by hand.
//!
//! Only the top-level object is walked; nested values are skipped without recursion, so that no
//! input can make the reader panic, recurse deeply or loop: every step moves forward by at least
//! one byte or ends the reading.

/// The `version` string of a manifest's top-level object, as it is written between its quotes,
/// or `None` when the text is not such an object or has no such string.
///
/// The value is not unescaped: a version holding a backslash does not read as numbers anyway.
pub(crate) fn manifest_version(text: &[u8]) -> Option<&str> {
    let mut reader = Reader { text, at: 0 };
    reader.skip_whitespace();
    reader.expect(b'{')?;
    reader.skip_whitespace();
    if reader.peek()? == b'}' {
        return None;
    }
    loop {
        let key = reader.string()?;
        reader.skip_whitespace();
        reader.expect(b':')?;
        reader.skip_whitespace();
        if key == b"version" && reader.peek()? == b'"' {
            return std::str::from_utf8(reader.string()?).ok();
        }
        reader.skip_value()?;
        reader.skip_whitespace();
        match reader.next()? {
            b',' => reader.skip_whitespace(),
            _ => return None,
        }
    }
}

struct Reader<'a> {
    text: &'a [u8],
    at: usize,
}

impl<'a> Reader<'a> {
    fn peek(&self) -> Option<u8> {
        self.text.get(self.at).copied()
    }

    fn next(&mut self) -> Option<u8> {
        let byte = self.peek()?;
        self.at += 1;
        Some(byte)
    }

    fn expect(&mut self, byte: u8) -> Option<()> {
        (self.next()? == byte).then_some(())
    }

    fn skip_whitespace(&mut self) {
        while matches!(self.peek(), Some(b' ' | b'\t' | b'\n' | b'\r')) {
            self.at += 1;
        }
    }

    /// A string's raw contents, the reader standing on its opening quote.
    fn string(&mut self) -> Option<&'a [u8]> {
        self.expect(b'"')?;
        let start = self.at;
        loop {
            match self.next()? {
                b'"' => return self.text.get(start..self.at - 1),
                b'\\' => {
                    self.next()?;
                }
                _ => {}
            }
        }
    }

    /// Skips one value: a string, an object or array with everything nested in it, or a
    /// literal (number, `true`, `false`, `null`).
    fn skip_value(&mut self) -> Option<()> {
        match self.peek()? {
            b'"' => self.string().map(|_| ()),
            b'{' | b'[' => self.skip_nested(),
            _ => {
                let start = self.at;
                while let Some(byte) = self.peek() {
                    if matches!(byte, b',' | b'}' | b']' | b' ' | b'\t' | b'\n' | b'\r') {
                        break;
                    }
                    self.at += 1;
                }
                (self.at > start).then_some(())
            }
        }
    }

    /// Skips an object or array, counting the depth instead of recursing.
    fn skip_nested(&mut self) -> Option<()> {
        let mut depth: usize = 0;
        loop {
            match self.peek()? {
                b'"' => {
                    self.string()?;
                }
                b'{' | b'[' => {
                    depth += 1;
                    self.at += 1;
                }
                b'}' | b']' => {
                    depth = depth.checked_sub(1)?;
                    self.at += 1;
                    if depth == 0 {
                        return Some(());
                    }
                }
                _ => self.at += 1,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_version_is_read_from_the_top_level_object() {
        let text = br#"{
            "manifest_version": 2,
            "name": "WidevineCdm",
            "platforms": [{"os": "linux", "arch": "x64", "sub_package_path": "_platform_specific/linux_x64/"}],
            "x-cdm": {"version": "1.2.3"},
            "version": "4.10.2830.0"
        }"#;
        assert_eq!(manifest_version(text), Some("4.10.2830.0"));
    }

    #[test]
    fn a_nested_version_is_not_the_manifests() {
        assert_eq!(manifest_version(br#"{"a": {"version": "1.0"}}"#), None);
        assert_eq!(manifest_version(br#"{"a": ["version", "1.0"]}"#), None);
        assert_eq!(manifest_version(br#"{"a": "\"version\": \"1.0\""}"#), None);
    }

    #[test]
    fn malformed_manifests_have_no_version() {
        for text in [
            &b""[..],
            b"   ",
            b"[]",
            b"{",
            b"{}",
            b"{\"version\"",
            b"{\"version\":",
            b"{\"version\": 4}",
            b"{\"version\": \"4.1",
            b"{\"a\": }",
            b"{\"a\": 1 \"version\": \"1\"}",
            b"{\"a\": {]}",
            b"{\"a\": ]]], \"version\": \"1\"}",
            b"{\"version\": \"\xff\xfe\"}",
            b"{\"a\\",
        ] {
            assert_eq!(
                manifest_version(text),
                None,
                "{:?}",
                String::from_utf8_lossy(text)
            );
        }
    }
}
