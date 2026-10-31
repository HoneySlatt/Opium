//! A download carried on where it broke off: a connection that stalls or
//! drops midway (a Wi-Fi blip, a server closing it) is asked again for the
//! rest, from where it stopped, rather than the song ending there.

use std::future::Future;
use std::time::Duration;

/// How many times in a row a broken download is asked again.
const TRIES: u64 = 3;

/// Push the rest of `resp`'s body, `received` bytes in already, through
/// `push` (false once nobody reads); broken off, `reopen` asks from where it
/// stopped. Why it ended short, if it did.
pub(crate) async fn carry_on<F, Fut>(
    mut resp: reqwest::Response,
    mut received: u64,
    reopen: F,
    mut push: impl FnMut(&[u8]) -> bool,
) -> Result<(), String>
where
    F: Fn(u64) -> Fut,
    Fut: Future<Output = Option<reqwest::Response>>,
{
    loop {
        let e = match resp.chunk().await {
            Ok(Some(bytes)) => {
                if !push(&bytes) {
                    return Ok(());
                }
                received += bytes.len() as u64;
                continue;
            }
            Ok(None) => return Ok(()),
            Err(e) => e.without_url().to_string(),
        };
        let mut again = None;
        for k in 1..=TRIES {
            tokio::time::sleep(Duration::from_secs(k)).await;
            // Only the rest, as asked: anything else would be pushed twice.
            if let Some(r) = reopen(received).await
                && r.status() == reqwest::StatusCode::PARTIAL_CONTENT
            {
                again = Some(r);
                break;
            }
        }
        resp = again.ok_or(e)?;
    }
}

/// `request` again, from byte `from` on.
pub(crate) fn rest_of(
    request: &reqwest::RequestBuilder,
    from: u64,
) -> impl Future<Output = Option<reqwest::Response>> + use<> {
    let request = request
        .try_clone()
        .map(|r| r.header("Range", format!("bytes={from}-")));
    async move { request?.send().await.ok() }
}

#[cfg(test)]
mod tests {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    /// A server that drops the first connection halfway, then answers a
    /// range with the rest.
    async fn flaky_server(body: &'static [u8]) -> String {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            for first in [true, false] {
                let (mut socket, _) = listener.accept().await.unwrap();
                let mut asked = vec![0; 2048];
                let n = socket.read(&mut asked).await.unwrap();
                let asked = String::from_utf8_lossy(&asked[..n]).to_lowercase();
                if first {
                    let head = format!("HTTP/1.1 200 OK\r\ncontent-length: {}\r\n\r\n", body.len());
                    socket.write_all(head.as_bytes()).await.unwrap();
                    socket.write_all(&body[..body.len() / 2]).await.unwrap();
                    // Dropped midway.
                } else {
                    let from: usize = asked
                        .split("range: bytes=")
                        .nth(1)
                        .and_then(|r| r.split('-').next())
                        .and_then(|n| n.trim().parse().ok())
                        .unwrap();
                    let rest = &body[from..];
                    let head = format!(
                        "HTTP/1.1 206 Partial Content\r\ncontent-length: {}\r\ncontent-range: bytes {from}-{}/{}\r\n\r\n",
                        rest.len(),
                        body.len() - 1,
                        body.len()
                    );
                    socket.write_all(head.as_bytes()).await.unwrap();
                    socket.write_all(rest).await.unwrap();
                }
            }
        });
        format!("http://{addr}/file")
    }

    #[tokio::test]
    async fn a_download_dropped_midway_goes_on_from_where_it_stopped() {
        let body: &'static [u8] = b"0123456789abcdefghijklmnopqrstuvwxyz";
        let url = flaky_server(body).await;
        let http = reqwest::Client::new();
        let resp = http.get(&url).send().await.unwrap();
        let again = http.get(&url);
        let mut got = Vec::new();
        let ended = super::carry_on(
            resp,
            0,
            |from| super::rest_of(&again, from),
            |bytes| {
                got.extend_from_slice(bytes);
                true
            },
        )
        .await;
        assert_eq!(ended, Ok(()));
        assert_eq!(got, body);
    }
}
