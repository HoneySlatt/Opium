//! What goes over a connection: frames in the clear until the two devices
//! have proved they were paired, then Noise messages only they can read.

use std::io;

use tokio::io::{AsyncReadExt, AsyncWriteExt};

/// The handshake: both know the key shared at pairing (psk), neither has a
/// lasting key pair; each connection's keys are new, so what was said
/// stays unreadable even should the shared key leak later.
const PATTERN: &str = "Noise_NNpsk0_25519_ChaChaPoly_SHA256";

/// The longest frame in the clear: a greeting, a pairing step.
const MAX_PLAIN: usize = 64 * 1024;

/// The longest message sealed: a whole queue handed over.
const MAX_SEALED: usize = 32 * 1024 * 1024;

const TAG: usize = 16;
const NOISE_MAX: usize = 65535;
/// A sealed message goes in pieces this long, each with a byte saying
/// whether more follow.
const PIECE: usize = NOISE_MAX - TAG - 1;

fn invalid(what: impl std::fmt::Display) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, what.to_string())
}

pub async fn write_plain<W: AsyncWriteExt + Unpin>(w: &mut W, bytes: &[u8]) -> io::Result<()> {
    if bytes.len() > MAX_PLAIN {
        return Err(invalid("frame too long"));
    }
    w.write_all(&(bytes.len() as u32).to_be_bytes()).await?;
    w.write_all(bytes).await?;
    w.flush().await
}

pub async fn read_plain<R: AsyncReadExt + Unpin>(r: &mut R) -> io::Result<Vec<u8>> {
    let len = r.read_u32().await? as usize;
    if len > MAX_PLAIN {
        return Err(invalid("frame too long"));
    }
    let mut bytes = vec![0; len];
    r.read_exact(&mut bytes).await?;
    Ok(bytes)
}

pub async fn write_json<W: AsyncWriteExt + Unpin>(
    w: &mut W,
    value: &impl serde::Serialize,
) -> io::Result<()> {
    write_plain(w, &serde_json::to_vec(value).map_err(invalid)?).await
}

pub async fn read_json<R: AsyncReadExt + Unpin, T: serde::de::DeserializeOwned>(
    r: &mut R,
) -> io::Result<T> {
    serde_json::from_slice(&read_plain(r).await?).map_err(invalid)
}

/// The two ends' keys once the handshake is through; each direction counts
/// its own messages.
pub type Keys = std::sync::Arc<snow::StatelessTransportState>;

/// Prove to each other the key from pairing, and agree on this connection's
/// keys. `prologue` names both ends, so that a message meant for one pair
/// of devices can't pass for another's.
pub async fn handshake<S: AsyncReadExt + AsyncWriteExt + Unpin>(
    stream: &mut S,
    initiator: bool,
    key: &[u8; 32],
    prologue: &[u8],
) -> io::Result<Keys> {
    let params = PATTERN.parse().map_err(invalid)?;
    let builder = snow::Builder::new(params)
        .psk(0, key)
        .and_then(|b| b.prologue(prologue))
        .map_err(invalid)?;
    let mut state = if initiator {
        builder.build_initiator()
    } else {
        builder.build_responder()
    }
    .map_err(invalid)?;
    let mut buf = vec![0; NOISE_MAX];
    let mut scratch = vec![0; NOISE_MAX];
    for turn in 0..2 {
        if (turn == 0) == initiator {
            let n = state.write_message(&[], &mut buf).map_err(invalid)?;
            write_plain(stream, &buf[..n]).await?;
        } else {
            let message = read_plain(stream).await?;
            // A wrong key fails here: the other end wasn't paired with us.
            state
                .read_message(&message, &mut scratch)
                .map_err(|_| invalid("not paired"))?;
        }
    }
    Ok(std::sync::Arc::new(
        state.into_stateless_transport_mode().map_err(invalid)?,
    ))
}

pub async fn send<W: AsyncWriteExt + Unpin>(
    w: &mut W,
    keys: &Keys,
    nonce: &mut u64,
    message: &[u8],
) -> io::Result<()> {
    let mut sealed = vec![0; NOISE_MAX];
    let mut plain = Vec::with_capacity(PIECE + 1);
    let pieces = message.len().div_ceil(PIECE).max(1);
    for i in 0..pieces {
        let piece = &message[i * PIECE..((i + 1) * PIECE).min(message.len())];
        plain.clear();
        plain.push(u8::from(i + 1 < pieces));
        plain.extend_from_slice(piece);
        let n = keys
            .write_message(*nonce, &plain, &mut sealed)
            .map_err(invalid)?;
        *nonce += 1;
        w.write_all(&(n as u16).to_be_bytes()).await?;
        w.write_all(&sealed[..n]).await?;
    }
    w.flush().await
}

pub async fn receive<R: AsyncReadExt + Unpin>(
    r: &mut R,
    keys: &Keys,
    nonce: &mut u64,
) -> io::Result<Vec<u8>> {
    let mut message = Vec::new();
    let mut sealed = vec![0; NOISE_MAX];
    let mut plain = vec![0; NOISE_MAX];
    loop {
        let len = r.read_u16().await? as usize;
        r.read_exact(&mut sealed[..len]).await?;
        let n = keys
            .read_message(*nonce, &sealed[..len], &mut plain)
            .map_err(|_| invalid("a message was tampered with"))?;
        *nonce += 1;
        let (&more, piece) = plain[..n]
            .split_first()
            .ok_or_else(|| invalid("empty piece"))?;
        if message.len() + piece.len() > MAX_SEALED {
            return Err(invalid("message too long"));
        }
        message.extend_from_slice(piece);
        if more == 0 {
            return Ok(message);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn the_same_key_talks_and_a_long_message_comes_whole() {
        let (mut a, mut b) = tokio::io::duplex(1 << 20);
        let key = [7; 32];
        let (ka, kb) = tokio::join!(
            handshake(&mut a, true, &key, b"x"),
            handshake(&mut b, false, &key, b"x"),
        );
        let (ka, kb) = (ka.unwrap(), kb.unwrap());
        let long: Vec<u8> = (0..200_000u32).map(|i| i as u8).collect();
        let (mut na, mut nb) = (0, 0);
        let (sent, got) = tokio::join!(send(&mut a, &ka, &mut na, &long), async {
            receive(&mut b, &kb, &mut nb).await
        });
        sent.unwrap();
        assert_eq!(got.unwrap(), long);
        send(&mut b, &kb, &mut 0, b"").await.unwrap();
        assert_eq!(receive(&mut a, &ka, &mut 0).await.unwrap(), b"");
    }

    #[tokio::test]
    async fn another_key_is_turned_away() {
        let (mut a, mut b) = tokio::io::duplex(1 << 16);
        let (ka, kb) = tokio::join!(handshake(&mut a, true, &[1; 32], b"x"), async move {
            // The responder hangs up: the initiator learns it so.
            handshake(&mut b, false, &[2; 32], b"x").await
        });
        assert!(kb.is_err());
        assert!(ka.is_err());
    }
}
