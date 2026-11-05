//! Box headers and bodies (ISO/IEC 14496-12 §4.2), read with checked arithmetic.
//!
//! Offsets are absolute: counted from the first byte of the data given. A box's *body* starts
//! after its type, its 64-bit size when it has one, and its extended type when it is a `uuid`.

use crate::Error;

/// A four-character box type.
pub(crate) type Kind = [u8; 4];

/// How deep boxes may nest: a top-level box is at depth 1, a child of its child at 3.
pub(crate) const MAX_DEPTH: u8 = 16;

/// A box located in the data.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Header {
    /// Where the box starts: its size field.
    pub at: usize,
    pub kind: Kind,
    /// Where its body starts.
    pub body: usize,
    /// Where it ends: the first byte after it.
    pub end: usize,
}

/// What the bytes at an offset say of the box starting there.
pub(crate) enum Found {
    /// Fewer bytes than the header needs have arrived.
    Cut,
    /// A box of `size == 0`, which runs to the end of its container.
    Open { kind: Kind, body: usize },
    /// A box with a declared end, which may lie past the data known.
    Sized(Header),
}

/// Reads the box header at `at`.
///
/// The header is complete only once its size, its type, its 64-bit size when `size == 1` and its
/// extended type when the type is `uuid` have all arrived; else [`Found::Cut`]. A size below
/// the header's own length, or one this machine cannot address, is an error.
pub(crate) fn find(data: &[u8], at: usize) -> Result<Found, Error> {
    let Some(head) = slice(data, at, 8) else {
        return Ok(Found::Cut);
    };
    let size = u32::from_be_bytes([head[0], head[1], head[2], head[3]]);
    let kind: Kind = [head[4], head[5], head[6], head[7]];
    let mut body = at + 8;
    let size = match size {
        0 => None,
        1 => {
            let Some(large) = slice(data, body, 8) else {
                return Ok(Found::Cut);
            };
            body += 8;
            Some(u64::from_be_bytes([
                large[0], large[1], large[2], large[3], large[4], large[5], large[6], large[7],
            ]))
        }
        n => Some(u64::from(n)),
    };
    if kind == *b"uuid" {
        if slice(data, body, 16).is_none() {
            return Ok(Found::Cut);
        }
        body += 16;
    }
    let Some(size) = size else {
        return Ok(Found::Open { kind, body });
    };
    let header_len = body - at;
    if size < header_len as u64 {
        return Err(Error::new(
            at,
            kind,
            format!("has a size of {size} bytes, less than its {header_len}-byte header"),
        ));
    }
    let end = usize::try_from(size)
        .ok()
        .and_then(|size| at.checked_add(size))
        .ok_or_else(|| {
            Error::new(
                at,
                kind,
                format!("has a size of {size} bytes, more than this machine can address"),
            )
        })?;
    Ok(Found::Sized(Header {
        at,
        kind,
        body,
        end,
    }))
}

/// `n` bytes at `at`, when they are all there.
pub(crate) fn slice(data: &[u8], at: usize, n: usize) -> Option<&[u8]> {
    data.get(at..at.checked_add(n)?)
}

/// A box type, quoted in backticks for an error message.
fn shown(kind: Kind) -> String {
    format!("`{}`", kind.escape_ascii())
}

/// The children of a container whose bytes have all arrived, one by one, each checked to lie
/// inside it.
pub(crate) struct Children<'a> {
    data: &'a [u8],
    parent: Header,
    at: usize,
    /// The depth of the children.
    pub depth: u8,
}

impl<'a> Children<'a> {
    /// The children of `parent`, which is at `parent_depth`; nesting past [`MAX_DEPTH`] is an
    /// error.
    pub fn of(data: &'a [u8], parent: Header, parent_depth: u8) -> Result<Self, Error> {
        if parent_depth >= MAX_DEPTH {
            return Err(Error::new(
                parent.at,
                parent.kind,
                format!("nests boxes deeper than {MAX_DEPTH} levels"),
            ));
        }
        Ok(Self {
            data,
            parent,
            at: parent.body,
            depth: parent_depth + 1,
        })
    }

    /// The next child; `steps` counts one per box read.
    pub fn next(&mut self, steps: &mut usize) -> Result<Option<Header>, Error> {
        if self.at >= self.parent.end {
            return Ok(None);
        }
        *steps += 1;
        let left = self.parent.end - self.at;
        let past = |header: Header| {
            Error::new(
                header.at,
                header.kind,
                format!("runs past its {}", shown(self.parent.kind)),
            )
        };
        // Read within the container only, so that what follows it never changes what is found:
        // a header that does not fit is stray bytes, whatever comes after.
        let inside = &self.data[..self.parent.end];
        let header = match find(inside, self.at)? {
            Found::Cut => {
                return Err(Error::new(
                    self.parent.at,
                    self.parent.kind,
                    format!("ends in {left} stray bytes"),
                ));
            }
            Found::Open { kind, body } => Header {
                at: self.at,
                kind,
                body,
                end: self.parent.end,
            },
            Found::Sized(header) => header,
        };
        if header.body > self.parent.end || header.end > self.parent.end {
            return Err(past(header));
        }
        self.at = header.end;
        Ok(Some(header))
    }
}

/// A box's body read field by field, each one checked to lie inside the box.
pub(crate) struct Body<'a> {
    data: &'a [u8],
    header: Header,
    at: usize,
}

impl<'a> Body<'a> {
    pub fn of(data: &'a [u8], header: Header) -> Self {
        Self {
            data,
            header,
            at: header.body,
        }
    }

    /// Where the next field starts.
    pub fn at(&self) -> usize {
        self.at
    }

    /// How many bytes of the body are left.
    pub fn left(&self) -> usize {
        self.header.end.saturating_sub(self.at)
    }

    /// The error for a body that ends before `what`.
    pub fn short(&self, what: &str) -> Error {
        Error::new(
            self.header.at,
            self.header.kind,
            format!("ends before its {what}"),
        )
    }

    /// An error about this box.
    pub fn fault(&self, what: impl Into<String>) -> Error {
        Error::new(self.header.at, self.header.kind, what)
    }

    /// The next `n` bytes, named `what` in the error when they are not all there.
    pub fn bytes(&mut self, n: usize, what: &str) -> Result<&'a [u8], Error> {
        if n > self.left() {
            return Err(self.short(what));
        }
        let bytes = self
            .data
            .get(self.at..self.at + n)
            .ok_or_else(|| self.short(what))?;
        self.at += n;
        Ok(bytes)
    }

    pub fn skip(&mut self, n: usize, what: &str) -> Result<(), Error> {
        self.bytes(n, what).map(|_| ())
    }

    pub fn array<const N: usize>(&mut self, what: &str) -> Result<[u8; N], Error> {
        let bytes = self.bytes(N, what)?;
        let mut array = [0; N];
        array.copy_from_slice(bytes);
        Ok(array)
    }

    pub fn u8(&mut self, what: &str) -> Result<u8, Error> {
        Ok(self.array::<1>(what)?[0])
    }

    pub fn u16(&mut self, what: &str) -> Result<u16, Error> {
        self.array(what).map(u16::from_be_bytes)
    }

    pub fn u32(&mut self, what: &str) -> Result<u32, Error> {
        self.array(what).map(u32::from_be_bytes)
    }

    pub fn i32(&mut self, what: &str) -> Result<i32, Error> {
        self.array(what).map(i32::from_be_bytes)
    }

    pub fn u64(&mut self, what: &str) -> Result<u64, Error> {
        self.array(what).map(u64::from_be_bytes)
    }

    /// A full box's version; its flags are read with [`Self::flags`] or skipped.
    pub fn version(&mut self) -> Result<u8, Error> {
        self.u8("version")
    }

    /// A full box's 24-bit flags.
    pub fn flags(&mut self) -> Result<u32, Error> {
        let [a, b, c] = self.array("flags")?;
        Ok(u32::from_be_bytes([0, a, b, c]))
    }

    /// The rest of the body, as the boxes it holds; `depth` is this box's depth.
    pub fn children(&self, depth: u8) -> Result<Children<'a>, Error> {
        let rest = Header {
            body: self.at.min(self.header.end),
            ..self.header
        };
        Children::of(self.data, rest, depth)
    }
}

/// The boxes of a range, read leniently for a rewrite: the walk stops at the first box it
/// cannot read whole, without an error. It holds no borrow, so that the caller may write between
/// two steps.
pub(crate) struct Lenient {
    at: usize,
    end: usize,
}

impl Lenient {
    pub fn new(at: usize, end: usize) -> Self {
        Self { at, end }
    }

    /// The boxes inside `parent`.
    pub fn inside(parent: Header) -> Self {
        Self::new(parent.body, parent.end)
    }

    pub fn next(&mut self, data: &[u8]) -> Option<Header> {
        let end = self.end.min(data.len());
        if self.at >= end {
            return None;
        }
        let header = match find(data, self.at) {
            Ok(Found::Sized(header)) if header.end <= end => header,
            Ok(Found::Open { kind, body }) if body <= end => Header {
                at: self.at,
                kind,
                body,
                end,
            },
            _ => return None,
        };
        self.at = header.end;
        Some(header)
    }
}

/// The box at `at`, read leniently: `None` unless it is whole and of `kind`.
pub(crate) fn lenient_at(data: &[u8], at: usize, kind: Kind) -> Option<Header> {
    match find(data, at) {
        Ok(Found::Sized(header)) if header.kind == kind && header.end <= data.len() => Some(header),
        Ok(Found::Open { kind: found, body }) if found == kind => Some(Header {
            at,
            kind,
            body,
            end: data.len(),
        }),
        _ => None,
    }
}

/// Reads a big-endian 32-bit value at `at`, leniently.
pub(crate) fn lenient_u32(data: &[u8], at: usize) -> Option<u32> {
    slice(data, at, 4).map(|b| u32::from_be_bytes([b[0], b[1], b[2], b[3]]))
}

/// Reads a box type at `at`, leniently.
pub(crate) fn lenient_kind(data: &[u8], at: usize) -> Option<Kind> {
    slice(data, at, 4).map(|b| [b[0], b[1], b[2], b[3]])
}

/// Writes `value` at `at` when the bytes are there; a rewrite never fails loudly.
pub(crate) fn write_u32(data: &mut [u8], at: usize, value: u32) {
    if let Some(bytes) = at.checked_add(4).and_then(|end| data.get_mut(at..end)) {
        bytes.copy_from_slice(&value.to_be_bytes());
    }
}

/// Changes a box's type.
pub(crate) fn retype(data: &mut [u8], header: Header, kind: Kind) {
    if let Some(bytes) = data.get_mut(header.at + 4..header.at + 8) {
        bytes.copy_from_slice(&kind);
    }
}

/// Pure containers of 14496-12, whose bodies are boxes wherever they appear. A misplaced one is
/// walked, so that nesting stays bounded, and its contents ignored.
pub(crate) fn is_container(kind: Kind) -> bool {
    matches!(
        &kind,
        b"moov"
            | b"trak"
            | b"edts"
            | b"mdia"
            | b"minf"
            | b"dinf"
            | b"stbl"
            | b"mvex"
            | b"moof"
            | b"traf"
            | b"sinf"
            | b"schi"
    )
}

/// Walks a box that has nothing to give where it was found, so that its nesting is checked.
pub(crate) fn skip(data: &[u8], header: Header, depth: u8, steps: &mut usize) -> Result<(), Error> {
    if !is_container(header.kind) {
        return Ok(());
    }
    let mut children = Children::of(data, header, depth)?;
    while let Some(child) = children.next(steps)? {
        skip(data, child, children.depth, steps)?;
    }
    Ok(())
}
