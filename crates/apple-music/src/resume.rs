//! A download carried on where it broke off: a connection that stalls or
//! drops midway (a Wi-Fi blip, a server closing it) is asked again for the
//! rest, from where it stopped, rather than the song ending there.

use std::future::Future;
use std::time::Duration;

/// How many times in a row a broken download is asked again.
const TRIES: u64 = 3;

/// Whether `resp` begins at byte `from`, as the Range asked: a `Content-Range`
/// saying so, or none at all (a server that answers 206 without one).
fn starts_at(resp: &reqwest::Response, from: u64) -> bool {
    let Some(range) = resp.headers().get(reqwest::header::CONTENT_RANGE) else {
        return true;
    };
    range
        .to_str()
        .ok()
        .and_then(|r| r.trim().strip_prefix("bytes "))
        .and_then(|r| r.split('-').next())
        .and_then(|start| start.trim().parse::<u64>().ok())
        .is_some_and(|start| start == from)
}

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
    let mut carried_nothing = 0u64;
    loop {
        let before = received;
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
            // Only the rest, as asked, and from the byte asked for: a
            // server that answers 206 from somewhere else — another
            // encode, another node of a CDN — would have its bytes put
            // after the ones already in, and the song would read as whole
            // while playing as noise from there on.
            if let Some(r) = reopen(received).await
                && r.status() == reqwest::StatusCode::PARTIAL_CONTENT
                && starts_at(&r, received)
            {
                again = Some(r);
                break;
            }
        }
        // Each break has its own tries, but a reopen that brings nothing
        // mustn't be tried for ever: once they are spent on a body that
        // gave no byte, it ends there.
        if again.is_some() && received == before {
            carried_nothing += 1;
            if carried_nothing > TRIES {
                return Err(e);
            }
        } else {
            carried_nothing = 0;
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
