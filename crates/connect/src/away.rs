//! Away from home: paired devices not on the same network reach each other
//! through iroh, which finds a way between them (straight through the
//! routers when it can, through a relay otherwise: n0's, or the one set)
//! and carries the same connection as at home, which only paired devices
//! can read. What it publishes of a device is the relay it uses, not its
//! address; and only paired devices know its id there.

use std::io;
use std::pin::Pin;
use std::task::{Context, Poll};

use iroh::endpoint::{Connection, Incoming, RecvStream, SendStream};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};

/// What Opium Connect speaks on iroh.
const ALPN: &[u8] = b"opium-connect/1";

/// This device on iroh, as `secret`, through `relay` (n0's when `None`).
pub async fn endpoint(secret: [u8; 32], relay: Option<&str>) -> Option<iroh::Endpoint> {
    let mut builder = iroh::Endpoint::builder(iroh::endpoint::presets::N0)
        .secret_key(iroh::SecretKey::from_bytes(&secret))
        .alpns(vec![ALPN.to_vec()]);
    if let Some(relay) = relay {
        match relay.parse::<iroh::RelayUrl>() {
            Ok(url) => builder = builder.relay_mode(iroh::RelayMode::custom([url])),
            Err(e) => eprintln!("opium connect: the relay {relay} isn't an address: {e}"),
        }
    }
    builder
        .bind()
        .await
        .map_err(|e| eprintln!("opium connect: can't go on iroh: {e}"))
        .ok()
}

/// A stream of an iroh connection, which stays open as long as it.
pub struct Quic {
    _connection: Connection,
    send: SendStream,
    recv: RecvStream,
}

/// Reach the device that is `to` on iroh.
pub async fn dial(endpoint: &iroh::Endpoint, to: &str) -> io::Result<Quic> {
    let id: iroh::EndpointId = to.parse().map_err(io::Error::other)?;
    let connection = endpoint.connect(id, ALPN).await.map_err(io::Error::other)?;
    let (send, recv) = connection.open_bi().await.map_err(io::Error::other)?;
    Ok(Quic {
        _connection: connection,
        send,
        recv,
    })
}

/// A device reaching this one on iroh, if `paired` knows it.
pub async fn answer(incoming: Incoming, paired: impl Fn(&str) -> bool) -> io::Result<Quic> {
    let connection = incoming.await.map_err(io::Error::other)?;
    if !paired(&connection.remote_id().to_string()) {
        connection.close(0u8.into(), b"not paired");
        return Err(io::Error::other("not paired"));
    }
    let (send, recv) = connection.accept_bi().await.map_err(io::Error::other)?;
    Ok(Quic {
        _connection: connection,
        send,
        recv,
    })
}

impl AsyncRead for Quic {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        Pin::new(&mut self.recv).poll_read(cx, buf)
    }
}

impl AsyncWrite for Quic {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        AsyncWrite::poll_write(Pin::new(&mut self.send), cx, buf)
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.send).poll_flush(cx)
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.send).poll_shutdown(cx)
    }
}
