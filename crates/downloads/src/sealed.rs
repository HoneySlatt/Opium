//! A downloaded song as it is kept on disk: sealed with the app's key, in
//! chunks of [`CHUNK`] bytes each sealed on its own, so that a reader seeks
//! anywhere without opening the rest. Only Music, which has the key, reads
//! them.
//!
//! The file is a header, then the chunks: [`MAGIC`] and a random nonce
//! prefix, each chunk's nonce that prefix and the chunk's index. The last
//! chunk is sealed as the last, so a file cut short doesn't read as whole.

use std::io::{self, Read, Seek, SeekFrom, Write};

use aes_gcm::Aes256Gcm;
use aes_gcm::aead::consts::U12;
use aes_gcm::aead::generic_array::GenericArray;
use aes_gcm::aead::{AeadInPlace, KeyInit};

const MAGIC: &[u8; 8] = b"MUSICSL1";
const PREFIX: usize = 8;
const HEADER: u64 = (MAGIC.len() + PREFIX) as u64;
/// How much of the song one chunk holds.
pub const CHUNK: usize = 64 * 1024;
const TAG: usize = 16;

pub type Key = [u8; 32];

/// A new random key.
pub fn new_key() -> Key {
    rand::random()
}

fn nonce(prefix: &[u8; PREFIX], index: u64) -> GenericArray<u8, U12> {
    let mut nonce = [0; 12];
    nonce[..PREFIX].copy_from_slice(prefix);
    // A song's chunks fit in 32 bits many times over.
    nonce[PREFIX..].copy_from_slice(&(index as u32).to_be_bytes());
    GenericArray::from(nonce)
}

fn aad(last: bool) -> [u8; 1] {
    [u8::from(last)]
}

fn broken() -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidData,
        "sealed file damaged or not ours",
    )
}

/// Seals what is written to it into `inner`; [`Writer::finish`] seals the
/// last chunk.
pub struct Writer<W: Write> {
    inner: W,
    cipher: Aes256Gcm,
    prefix: [u8; PREFIX],
    index: u64,
    buffer: Vec<u8>,
}

impl<W: Write> Writer<W> {
    pub fn new(mut inner: W, key: &Key) -> io::Result<Self> {
        let prefix: [u8; PREFIX] = rand::random();
        inner.write_all(MAGIC)?;
        inner.write_all(&prefix)?;
        Ok(Self {
            inner,
            cipher: Aes256Gcm::new(key.into()),
            prefix,
            index: 0,
            buffer: Vec::with_capacity(CHUNK + TAG),
        })
    }

    fn seal(&mut self, last: bool) -> io::Result<()> {
        let nonce = nonce(&self.prefix, self.index);
        let tag = self
            .cipher
            .encrypt_in_place_detached(&nonce, &aad(last), &mut self.buffer)
            .map_err(|_| io::Error::other("sealing failed"))?;
        self.inner.write_all(&self.buffer)?;
        self.inner.write_all(&tag)?;
        self.buffer.clear();
        self.index += 1;
        Ok(())
    }

    /// Seal what is left as the last chunk, and hand `inner` back.
    pub fn finish(mut self) -> io::Result<W> {
        self.seal(true)?;
        self.inner.flush()?;
        Ok(self.inner)
    }
}

impl<W: Write> Write for Writer<W> {
    fn write(&mut self, mut buf: &[u8]) -> io::Result<usize> {
        let n = buf.len();
        while !buf.is_empty() {
            // A full chunk waits for more: the last one is sealed apart.
            if self.buffer.len() == CHUNK {
                self.seal(false)?;
            }
            let take = (CHUNK - self.buffer.len()).min(buf.len());
            self.buffer.extend_from_slice(&buf[..take]);
            buf = &buf[take..];
        }
        Ok(n)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}

/// Reads a sealed file back as the song it holds.
pub struct Reader<R: Read + Seek> {
    inner: R,
    cipher: Aes256Gcm,
    prefix: [u8; PREFIX],
    chunks: u64,
    len: u64,
    pos: u64,
    /// The chunk opened last, and its index.
    open: Option<(u64, Vec<u8>)>,
}

impl<R: Read + Seek> Reader<R> {
    pub fn new(mut inner: R, key: &Key) -> io::Result<Self> {
        let size = inner.seek(SeekFrom::End(0))?;
        inner.seek(SeekFrom::Start(0))?;
        let mut header = [0; HEADER as usize];
        inner.read_exact(&mut header).map_err(|_| broken())?;
        if &header[..MAGIC.len()] != MAGIC {
            return Err(broken());
        }
        let body = size - HEADER;
        let sealed = (CHUNK + TAG) as u64;
        let chunks = body.div_ceil(sealed);
        if chunks == 0 || (!body.is_multiple_of(sealed) && body % sealed < TAG as u64) {
            return Err(broken());
        }
        Ok(Self {
            inner,
            cipher: Aes256Gcm::new(key.into()),
            prefix: header[MAGIC.len()..].try_into().expect("8 bytes"),
            chunks,
            len: body - chunks * TAG as u64,
            pos: 0,
            open: None,
        })
    }

    /// How long the song is.
    pub fn len(&self) -> u64 {
        self.len
    }

    /// Whether the key opens it, by reading its first chunk: a song sealed
    /// under another key (the keyring's remade) fails here, where it can
    /// still be played from its service, rather than halfway through.
    pub fn opens(&mut self) -> io::Result<()> {
        self.chunk(0).map(|_| ())
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    fn chunk(&mut self, index: u64) -> io::Result<&[u8]> {
        if self.open.as_ref().is_none_or(|(i, _)| *i != index) {
            let sealed = (CHUNK + TAG) as u64;
            let start = HEADER + index * sealed;
            let size = (HEADER + self.len + self.chunks * TAG as u64 - start).min(sealed);
            let mut data = vec![0; size as usize];
            self.inner.seek(SeekFrom::Start(start))?;
            self.inner.read_exact(&mut data)?;
            let tag = data.split_off(data.len() - TAG);
            let last = index + 1 == self.chunks;
            self.cipher
                .decrypt_in_place_detached(
                    &nonce(&self.prefix, index),
                    &aad(last),
                    &mut data,
                    tag.as_slice().into(),
                )
                .map_err(|_| broken())?;
            self.open = Some((index, data));
        }
        Ok(&self.open.as_ref().expect("just opened").1)
    }
}

impl<R: Read + Seek> Read for Reader<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if self.pos >= self.len || buf.is_empty() {
            return Ok(0);
        }
        let index = self.pos / CHUNK as u64;
        let at = (self.pos % CHUNK as u64) as usize;
        let chunk = self.chunk(index)?;
        let n = (chunk.len() - at).min(buf.len());
        buf[..n].copy_from_slice(&chunk[at..at + n]);
        self.pos += n as u64;
        Ok(n)
    }
}

impl<R: Read + Seek> Seek for Reader<R> {
    fn seek(&mut self, to: SeekFrom) -> io::Result<u64> {
        let pos = match to {
            SeekFrom::Start(p) => Some(p),
            SeekFrom::End(d) => self.len.checked_add_signed(d),
            SeekFrom::Current(d) => self.pos.checked_add_signed(d),
        };
        self.pos =
            pos.ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "seek before start"))?;
        Ok(self.pos)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    fn sealed(data: &[u8], key: &Key) -> Vec<u8> {
        let mut writer = Writer::new(Vec::new(), key).unwrap();
        // In uneven pieces, as a download comes.
        for piece in data.chunks(7_777) {
            writer.write_all(piece).unwrap();
        }
        writer.finish().unwrap()
    }

    fn song(len: usize) -> Vec<u8> {
        (0..len).map(|i| (i * 31 % 251) as u8).collect()
    }

    #[test]
    fn reads_back_what_was_sealed() {
        let key = new_key();
        for len in [0, 1, CHUNK - 1, CHUNK, CHUNK + 1, 3 * CHUNK, 3 * CHUNK + 5] {
            let data = song(len);
            let file = sealed(&data, &key);
            assert_ne!(&file[HEADER as usize..], &data[..], "not sealed");
            let mut reader = Reader::new(Cursor::new(file), &key).unwrap();
            assert_eq!(reader.len(), len as u64);
            let mut back = Vec::new();
            reader.read_to_end(&mut back).unwrap();
            assert_eq!(back, data, "length {len}");
        }
    }

    #[test]
    fn seeks_anywhere() {
        let key = new_key();
        let data = song(5 * CHUNK + 123);
        let mut reader = Reader::new(Cursor::new(sealed(&data, &key)), &key).unwrap();
        for at in [4 * CHUNK + 7, 3, CHUNK - 2, 5 * CHUNK + 100] {
            reader.seek(SeekFrom::Start(at as u64)).unwrap();
            let mut buf = [0; 40];
            let n = reader.read(&mut buf).unwrap();
            assert!(n > 0);
            assert_eq!(&buf[..n], &data[at..at + n]);
        }
        reader.seek(SeekFrom::End(-1)).unwrap();
        let mut last = [0; 1];
        reader.read_exact(&mut last).unwrap();
        assert_eq!(last[0], *data.last().unwrap());
    }

    #[test]
    fn refuses_another_key() {
        let file = sealed(&song(1000), &new_key());
        let mut reader = Reader::new(Cursor::new(file), &new_key()).unwrap();
        assert!(reader.read(&mut [0; 10]).is_err());
    }

    #[test]
    fn refuses_a_file_cut_short() {
        let key = new_key();
        let mut file = sealed(&song(3 * CHUNK), &key);
        // The last chunk gone: the one before isn't sealed as the last.
        file.truncate(HEADER as usize + 2 * (CHUNK + TAG));
        let mut reader = Reader::new(Cursor::new(file), &key).unwrap();
        let mut back = Vec::new();
        assert!(reader.read_to_end(&mut back).is_err());
    }

    #[test]
    fn refuses_a_changed_byte() {
        let key = new_key();
        let mut file = sealed(&song(2 * CHUNK), &key);
        file[HEADER as usize + CHUNK + 40] ^= 1;
        let mut reader = Reader::new(Cursor::new(file), &key).unwrap();
        reader.seek(SeekFrom::Start(CHUNK as u64 + 10)).unwrap();
        assert!(reader.read(&mut [0; 10]).is_err());
    }
}
