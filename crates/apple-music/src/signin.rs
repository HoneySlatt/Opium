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
use tokio::process::{Child, Command};
use tokio_tungstenite::tungstenite::Message;

const SIGNIN_URL: &str = "https://music.apple.com/signin";
const TOKEN_COOKIE: &str = "media-user-token";
const POLL: Duration = Duration::from_millis(500);

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
pub async fn sign_in(
    browser: &Path,
    profile: &Path,
    timeout: Duration,
) -> Result<String, SignInError> {
    let port_file = profile.join("DevToolsActivePort");
    // A stale file from an earlier run would point at a dead port.
    let _ = tokio::fs::remove_file(&port_file).await;

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
    let endpoint = loop {
        if let Some(endpoint) = read_port_file(&port_file).await {
            break endpoint;
        }
        wait_a_bit(&mut child, deadline, timeout).await?;
    };

    let (mut ws, _) = tokio_tungstenite::connect_async(endpoint.as_str())
        .await
        .map_err(|e| SignInError::DevTools(e.to_string()))?;

    let mut request_id = 0u64;
    loop {
        request_id += 1;
        let request = serde_json::json!({ "id": request_id, "method": "Storage.getCookies" });
        ws.send(Message::text(request.to_string()))
            .await
            .map_err(|e| SignInError::DevTools(e.to_string()))?;

        let reply = loop {
            let msg = ws
                .next()
                .await
                .ok_or(SignInError::BrowserClosed)?
                .map_err(|_| SignInError::BrowserClosed)?;
            let Message::Text(text) = msg else { continue };
            let value: serde_json::Value = serde_json::from_str(&text)
                .map_err(|e| SignInError::DevTools(e.to_string()))?;
            // Events and stale replies share the socket; only ours matters.
            if value["id"] == request_id {
                break value;
            }
        };

        if let Some(token) = find_token(&reply) {
            let close = serde_json::json!({ "id": request_id + 1, "method": "Browser.close" });
            let _ = ws.send(Message::text(close.to_string())).await;
            let _ = tokio::time::timeout(Duration::from_secs(5), child.wait()).await;
            return Ok(token);
        }
        wait_a_bit(&mut child, deadline, timeout).await?;
    }
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

async fn wait_a_bit(
    child: &mut Child,
    deadline: Instant,
    timeout: Duration,
) -> Result<(), SignInError> {
    if Instant::now() > deadline {
        return Err(SignInError::TimedOut(timeout));
    }
    if let Ok(Some(_)) = child.try_wait() {
        return Err(SignInError::BrowserClosed);
    }
    tokio::time::sleep(POLL).await;
    Ok(())
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
