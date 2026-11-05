use std::fmt;

/// What was wrong with a file: which box, where it starts, and what was wrong with it.
///
/// Displayed as "the `trun` at 0x1a2b runs past its `traf`": the type, the offset of the box
/// from the start of the data given, then [`what`](Self::what) as a verb phrase.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Error {
    /// Where the box starts, counted from the first byte of the data given.
    pub offset: usize,
    /// The box's four-character type, as found in the file.
    pub kind: [u8; 4],
    /// What was wrong, as a verb phrase that follows "the `kind` at offset".
    pub what: String,
}

impl Error {
    pub(crate) fn new(offset: usize, kind: [u8; 4], what: impl Into<String>) -> Self {
        Self {
            offset,
            kind,
            what: what.into(),
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "the `{}` at {:#x} {}",
            self.kind.escape_ascii(),
            self.offset,
            self.what
        )
    }
}

impl std::error::Error for Error {}
