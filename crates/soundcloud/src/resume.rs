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
