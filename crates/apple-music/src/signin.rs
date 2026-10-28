//! Signing in through a real browser to obtain the `media-user-token`.
//!
//! Apple's sign-in page is a web flow (with 2FA), so the user goes through it in a
//! Chromium-family browser running on its own profile. The browser is started with
//! the DevTools protocol enabled and asked for its cookies until the token shows
//! up — reading it live avoids decrypting Chromium's on-disk cookie store, whose
//! format and key storage keep changing.
//!
//! The profile is kept between sign-ins: when the token expires, a browser that is
//! still signed in to music.apple.com hands over a fresh one without any typing.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use futures_util::{SinkExt, StreamExt};
use tokio::process::Command;
use tokio_tungstenite::tungstenite::Message;

const SIGNIN_URL: &str = "https://music.apple.com/signin";
const TOKEN_COOKIE: &str = "media-user-token";
const POLL: Duration = Duration::from_millis(500);
/// Polls (of [`POLL`]) without an answering browser before giving up on it.
const GIVE_UP_AFTER: u32 = 10;

#[derive(Debug, thiserror::Error)]
pub enum SignInError {
    #[error("could not start the browser `{0}`: {1}")]
    Launch(PathBuf, std::io::Error),
    #[error("the browser was closed before sign-in finished")]
    BrowserClosed,
    #[error("sign-in did not finish within {0:?}")]
    TimedOut(Duration),
    #[error("DevTools connection failed: {0}")]
    DevTools(String),
}

/// Open `browser` on Apple's sign-in page and wait for the user to finish.
///
/// `profile` is the browser profile dedicated to this; it is created if missing.
///
/// The process we spawn is not a reliable signal: Chromium restarts itself on a
/// fresh profile, and hands off to an instance already running on the same
/// profile. So the browser is followed through `DevToolsActivePort`, reconnecting
/// whenever it moves, and only counts as closed once our process is gone and
/// nothing answers there any more.
pub async fn sign_in(
    browser: &Path,
    profile: &Path,
    timeout: Duration,
) -> Result<String, SignInError> {
    let port_file = profile.join("DevToolsActivePort");
    let mut child = Command::new(browser)
        .arg(format!("--user-data-dir={}", profile.display()))
        .args([
            "--no-first-run",
            "--no-default-browser-check",
            "--remote-debugging-port=0",
        ])
        .arg(format!("--app={SIGNIN_URL}"))
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .kill_on_drop(true)
        .spawn()
        .map_err(|e| SignInError::Launch(browser.to_owned(), e))?;

    let deadline = Instant::now() + timeout;
    let mut unreachable_after_exit = 0;
    loop {
        if let Some(endpoint) = read_port_file(&port_file).await
            && let Ok((mut ws, _)) = tokio_tungstenite::connect_async(endpoint.as_str()).await
        {
            unreachable_after_exit = 0;
            if let Some(token) = poll_token(&mut ws, deadline, timeout).await? {
                let close = serde_json::json!({ "id": 0, "method": "Browser.close" });
                let _ = ws.send(Message::text(close.to_string())).await;
                return Ok(token);
            }
        }
        if Instant::now() > deadline {
            return Err(SignInError::TimedOut(timeout));
        }
        if let Ok(Some(_)) = child.try_wait() {
            unreachable_after_exit += 1;
            if unreachable_after_exit > GIVE_UP_AFTER {
                return Err(SignInError::BrowserClosed);
            }
        }
        tokio::time::sleep(POLL).await;
    }
}

/// Ask for cookies until the token appears. `Ok(None)` when the connection drops,
/// which is the browser restarting or closing.
async fn poll_token<S>(
    ws: &mut S,
    deadline: Instant,
    timeout: Duration,
) -> Result<Option<String>, SignInError>
where
    S: futures_util::Sink<Message>
        + futures_util::Stream<Item = Result<Message, tokio_tungstenite::tungstenite::Error>>
        + Unpin,
{
    for request_id in 1u64.. {
        let request = serde_json::json!({ "id": request_id, "method": "Storage.getCookies" });
        if ws.send(Message::text(request.to_string())).await.is_err() {
            return Ok(None);
        }
        let reply = loop {
            let Some(Ok(msg)) = ws.next().await else {
                return Ok(None);
            };
            let Message::Text(text) = msg else { continue };
            let value: serde_json::Value = serde_json::from_str(&text)
                .map_err(|e| SignInError::DevTools(e.to_string()))?;
            // Events and stale replies share the socket; only ours matters.
            if value["id"] == request_id {
                break value;
            }
        };
        if let Some(token) = find_token(&reply) {
            return Ok(Some(token));
        }
        if Instant::now() > deadline {
            return Err(SignInError::TimedOut(timeout));
        }
        tokio::time::sleep(POLL).await;
    }
    unreachable!("request ids ran out")
}

/// `DevToolsActivePort` holds the port on its first line and the browser target's
/// path on its second.
async fn read_port_file(path: &Path) -> Option<String> {
    let contents = tokio::fs::read_to_string(path).await.ok()?;
    let mut lines = contents.lines();
    let port = lines.next()?.trim();
    let target = lines.next()?.trim();
    (!port.is_empty() && !target.is_empty()).then(|| format!("ws://127.0.0.1:{port}{target}"))
}

fn find_token(reply: &serde_json::Value) -> Option<String> {
    reply["result"]["cookies"]
        .as_array()?
        .iter()
        .find(|c| {
            c["name"] == TOKEN_COOKIE
                && c["domain"]
                    .as_str()
                    .is_some_and(|d| d.ends_with("music.apple.com"))
        })
        .and_then(|c| c["value"].as_str())
        .filter(|v| !v.is_empty())
        .map(str::to_owned)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_token_is_picked_from_apple_music_cookies_only() {
        let reply = serde_json::json!({ "id": 3, "result": { "cookies": [
            { "name": "media-user-token", "value": "elsewhere", "domain": ".example.com" },
            { "name": "itspod", "value": "47", "domain": ".music.apple.com" },
            { "name": "media-user-token", "value": "the-token", "domain": ".music.apple.com" },
        ]}});
        assert_eq!(find_token(&reply).as_deref(), Some("the-token"));
    }

    #[test]
    fn an_empty_token_is_not_a_sign_in() {
        let reply = serde_json::json!({ "id": 1, "result": { "cookies": [
            { "name": "media-user-token", "value": "", "domain": ".music.apple.com" },
        ]}});
        assert_eq!(find_token(&reply), None);
    }
}
