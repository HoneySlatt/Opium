//! A song one device downloads for another: the phone, which can't read
//! Apple Music's and SoundCloud's encrypted songs to keep them, asks the
//! computer, which can, over a connection of their own, paired and
//! encrypted as the others are. The phone keeps what comes sealed, as it
//! keeps its own downloads; nothing else sees it on the way.
//!
//! The borrower asks for one song; the lender answers what it is, then its
//! bytes in pieces, then an empty piece. Hanging up calls it off.

use std::io::{self, Read};

use music_core::model::Track;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

use crate::{Conn, failed, wire};

/// The bytes of a song come in pieces this long.
const PIECE: usize = 256 * 1024;
/// Pieces read ahead of the connection, on either side.
const AHEAD: usize = 4;

/// What a song lent is.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Lent {
    pub len: u64,
    /// `m4a`, `flac`…
    pub extension: String,
    /// Frames the encoder put before the music, which the file doesn't say.
    pub priming: u32,
    /// It came through the service's DRM: it plays while the subscription does.
    pub protected: bool,
    pub kbps: Option<u32>,
    pub spatial: bool,
}

/// What lends songs: the computer's downloads.
#[async_trait::async_trait]
pub trait Lender: Send + Sync {
    /// `track` as downloading it would get it; `original`: a SoundCloud
    /// uploader's own file, where they allow it.
    async fn lend(&self, track: Track, original: bool) -> Result<(Lent, Box<dyn Read + Send>), String>;
}

#[derive(Serialize, Deserialize)]
struct Ask {
    track: Track,
    original: bool,
}

#[derive(Serialize, Deserialize)]
enum Answer {
    Coming(Lent),
    Refused(String),
}

/// A song coming from another device, read as it comes; the connection
/// ends once it is dropped.
pub struct Borrowed {
    pieces: mpsc::Receiver<io::Result<Vec<u8>>>,
    piece: Vec<u8>,
    at: usize,
}

impl Read for Borrowed {
    /// Blocking: not from within an async task.
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        while self.at == self.piece.len() {
            match self.pieces.blocking_recv() {
                Some(piece) => {
                    self.piece = piece?;
                    self.at = 0;
                }
                None => return Ok(0),
            }
        }
        let n = buf.len().min(self.piece.len() - self.at);
        buf[..n].copy_from_slice(&self.piece[self.at..self.at + n]);
        self.at += n;
        Ok(n)
    }
}

/// Ask for `track` over `stream`, its handshake made: what it is, and its
/// bytes as they come.
pub(crate) async fn borrow(
    mut stream: Conn,
    keys: wire::Keys,
    track: Track,
    original: bool,
) -> io::Result<(Lent, Borrowed)> {
    let ask = serde_json::to_vec(&Ask { track, original }).map_err(io::Error::other)?;
    let (mut sent, mut heard) = (0, 0);
    wire::send(&mut stream, &keys, &mut sent, &ask).await?;
    let answer: Answer = serde_json::from_slice(&wire::receive(&mut stream, &keys, &mut heard).await?)
        .map_err(io::Error::other)?;
    let lent = match answer {
        Answer::Coming(lent) => lent,
        Answer::Refused(reason) => return Err(failed(&reason)),
    };
    let (tx, pieces) = mpsc::channel(AHEAD);
    let len = lent.len;
    tokio::spawn(async move {
        let mut got = 0u64;
        loop {
            let piece = match wire::receive(&mut stream, &keys, &mut heard).await {
                Ok(piece) => piece,
                Err(e) => {
                    let _ = tx.send(Err(e)).await;
                    return;
                }
            };
            if piece.is_empty() {
                if got != len {
                    let _ = tx.send(Err(failed("the song came short"))).await;
                }
                return;
            }
            got += piece.len() as u64;
            // Dropped: called off, and so is the connection.
            if tx.send(Ok(piece)).await.is_err() {
                return;
            }
        }
    });
    Ok((
        lent,
        Borrowed {
            pieces,
            piece: Vec::new(),
            at: 0,
        },
    ))
}

/// Answer a device asking for a song over `stream`, its handshake made.
pub(crate) async fn lend(mut stream: Conn, keys: wire::Keys, lender: Option<&dyn Lender>) -> io::Result<()> {
    let (mut sent, mut heard) = (0, 0);
    let ask: Ask = serde_json::from_slice(&wire::receive(&mut stream, &keys, &mut heard).await?)
        .map_err(io::Error::other)?;
    let lent = match lender {
        Some(lender) => lender.lend(ask.track, ask.original).await,
        None => Err("this device doesn't lend songs".into()),
    };
    let (lent, reader) = match lent {
        Ok(lent) => lent,
        Err(reason) => {
            let answer = serde_json::to_vec(&Answer::Refused(reason)).map_err(io::Error::other)?;
            return wire::send(&mut stream, &keys, &mut sent, &answer).await;
        }
    };
    let answer = serde_json::to_vec(&Answer::Coming(lent)).map_err(io::Error::other)?;
    wire::send(&mut stream, &keys, &mut sent, &answer).await?;
    // Read off the runtime (the computer's decrypts as it reads), a few
    // pieces ahead of the connection.
    let (tx, mut pieces) = mpsc::channel::<io::Result<Vec<u8>>>(AHEAD);
    tokio::task::spawn_blocking(move || {
        let mut reader = reader;
        loop {
            let mut piece = vec![0; PIECE];
            let mut n = 0;
            while n < PIECE {
                match reader.read(&mut piece[n..]) {
                    Ok(0) => break,
                    Ok(k) => n += k,
                    Err(e) if e.kind() == io::ErrorKind::Interrupted => {}
                    Err(e) => {
                        let _ = tx.blocking_send(Err(e));
                        return;
                    }
                }
            }
            piece.truncate(n);
            let last = n < PIECE;
            // The connection gone: no more is read.
            if (n > 0 && tx.blocking_send(Ok(piece)).is_err()) || last {
                return;
            }
        }
    });
    while let Some(piece) = pieces.recv().await {
        wire::send(&mut stream, &keys, &mut sent, &piece?).await?;
    }
    wire::send(&mut stream, &keys, &mut sent, &[]).await
}
