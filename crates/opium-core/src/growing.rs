//! A file downloading in the background, read as it comes: a read past what
//! has come waits for it, so a track plays from its first bytes. The
//! download pushes into a [`Feed`]; dropping the [`Growing`] reader tells it
//! to stop.

use std::io::{Read, Seek, SeekFrom};
use std::sync::{Arc, Condvar, Mutex, MutexGuard, Weak};

/// The reading end, `len` bytes long in all.
pub struct Growing {
    shared: Arc<Shared>,
    pos: u64,
    len: u64,
}

/// The downloading end.
pub struct Feed {
    shared: Weak<Shared>,
}

struct Shared {
    state: Mutex<State>,
    more: Condvar,
}

#[derive(Default)]
struct State {
    data: Vec<u8>,
    done: bool,
    error: Option<String>,
}

/// A file of `len` bytes to come.
pub fn growing(len: u64) -> (Growing, Feed) {
    let shared = Arc::new(Shared {
        state: Mutex::new(State::default()),
        more: Condvar::new(),
    });
    let feed = Feed {
        shared: Arc::downgrade(&shared),
    };
    (
        Growing {
            shared,
            pos: 0,
            len,
        },
        feed,
    )
}

impl Shared {
    fn lock(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(|e| e.into_inner())
    }
}

impl Feed {
    /// Add what came; `false` once the reader is gone, to stop.
    pub fn push(&self, bytes: &[u8]) -> bool {
        let Some(shared) = self.shared.upgrade() else {
            return false;
        };
        shared.lock().data.extend_from_slice(bytes);
        shared.more.notify_all();
        true
    }

    /// The download ended, or failed with `error`.
    pub fn end(self, error: Option<String>) {
        let Some(shared) = self.shared.upgrade() else {
            return;
        };
        let mut state = shared.lock();
        state.done = true;
        state.error = error;
        drop(state);
        shared.more.notify_all();
    }
}

impl Read for Growing {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        if self.pos >= self.len || buf.is_empty() {
            return Ok(0);
        }
        let mut state = self.shared.lock();
        loop {
            let have = state.data.len() as u64;
            if have > self.pos {
                let start = self.pos as usize;
                let n = buf.len().min(state.data.len() - start);
                buf[..n].copy_from_slice(&state.data[start..start + n]);
                self.pos += n as u64;
                return Ok(n);
            }
            if state.done {
                return match &state.error {
                    Some(e) => Err(std::io::Error::other(e.clone())),
                    None => Ok(0),
                };
            }
            state = self
                .shared
                .more
                .wait(state)
                .unwrap_or_else(|e| e.into_inner());
        }
    }
}

impl Seek for Growing {
    fn seek(&mut self, to: SeekFrom) -> std::io::Result<u64> {
        let pos = match to {
            SeekFrom::Start(p) => p as i64,
            SeekFrom::Current(d) => self.pos as i64 + d,
            SeekFrom::End(d) => self.len as i64 + d,
        };
        if pos < 0 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "seek before the start",
            ));
        }
        self.pos = pos as u64;
        Ok(self.pos)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_reads_what_has_come_and_waits_for_the_rest() {
        let (mut reader, feed) = growing(6);
        feed.push(b"abc");
        let later = std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(50));
            feed.push(b"def");
            feed.end(None);
        });
        let mut all = Vec::new();
        reader.read_to_end(&mut all).unwrap();
        later.join().unwrap();
        assert_eq!(all, b"abcdef");
        reader.seek(SeekFrom::End(-2)).unwrap();
        let mut tail = Vec::new();
        reader.read_to_end(&mut tail).unwrap();
        assert_eq!(tail, b"ef");
    }

    #[test]
    fn the_feed_stops_once_the_reader_is_gone() {
        let (reader, feed) = growing(6);
        assert!(feed.push(b"abc"));
        drop(reader);
        assert!(!feed.push(b"def"));
    }
}
