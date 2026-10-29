//! Signing in to a service through a real browser, to obtain the cookies its
//! web player keeps the user's tokens in.
//!
//! Sign-in pages are web flows (with 2FA, captchas), so the user goes through
//! them in a Chromium-family browser running on its own profile. The browser is
//! started with the DevTools protocol enabled and asked for its cookies until
//! the token shows up — reading them live avoids decrypting Chromium's on-disk
//! cookie store, whose format and key storage keep changing.
//!
//! The profile is kept between sign-ins: when a token expires, a browser that
//! is still signed in hands over a fresh one without any typing.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use futures_util::{SinkExt, StreamExt};
use tokio::process::Command;
use tokio_tungstenite::tungstenite::Message;

/// Where to sign in, and which cookies to bring back.
#[derive(Debug, Clone, Copy)]
pub struct Site<'a> {
    /// The page the browser opens.
    pub url: &'a str,
    /// The cookies' domain, or a parent of it: `music.apple.com`.
    pub domain: &'a str,
    /// The cookie whose arrival means the user is signed in.
    pub token: &'a str,
    /// Other cookies of the domain to bring back with it, when set.
    pub also: &'a [&'a str],
}
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
    #[error("the browser was closed without signing in")]
    NotSignedIn,
}

/// Open `browser` on the site's sign-in page and wait for the user to finish;
/// returns the token cookie and those of `also` that were set, by name.
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
    site: Site<'_>,
    timeout: Duration,
) -> Result<HashMap<String, String>, SignInError> {
    let port_file = profile.join("DevToolsActivePort");
    let mut child = Command::new(browser)
        .arg(format!("--user-data-dir={}", profile.display()))
        .args([
            "--no-first-run",
            "--no-default-browser-check",
            "--remote-debugging-port=0",
        ])
        .arg(format!("--app={}", site.url))
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
            if let Some(cookies) = poll_cookies(&mut ws, site, deadline, timeout).await? {
                let close = serde_json::json!({ "id": 0, "method": "Browser.close" });
                let _ = ws.send(Message::text(close.to_string())).await;
                return Ok(cookies);
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

/// The same, for sites that may turn a browser away when its DevTools are
/// open (SoundCloud's bot guard has): the browser opens as any other, and once
/// the user has signed in and closed it, it is started again unseen, on no
/// page, only to hand the cookies over.
///
/// The process we spawn is not a reliable signal (see [`sign_in`]): the
/// browser counts as closed once our process is gone and nothing holds the
/// profile's lock any more.
pub async fn sign_in_unwatched(
    browser: &Path,
    profile: &Path,
    site: Site<'_>,
    timeout: Duration,
) -> Result<HashMap<String, String>, SignInError> {
    let mut child = Command::new(browser)
        .arg(format!("--user-data-dir={}", profile.display()))
        .args(["--no-first-run", "--no-default-browser-check"])
        .arg(format!("--app={}", site.url))
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .kill_on_drop(true)
        .spawn()
        .map_err(|e| SignInError::Launch(browser.to_owned(), e))?;
    let deadline = Instant::now() + timeout;
    let lock = profile.join("SingletonLock");
    loop {
        tokio::time::sleep(POLL).await;
        let exited = matches!(child.try_wait(), Ok(Some(_)));
        if exited && tokio::fs::symlink_metadata(&lock).await.is_err() {
            break;
        }
        if Instant::now() > deadline {
            return Err(SignInError::TimedOut(timeout));
        }
    }
    let cookies = read_cookies(browser, profile, site).await?;
    if cookies.contains_key(site.token) {
        Ok(cookies)
    } else {
        Err(SignInError::NotSignedIn)
    }
}

/// The site's cookies in `profile`, from the browser started headless on no
/// page.
async fn read_cookies(
    browser: &Path,
    profile: &Path,
    site: Site<'_>,
) -> Result<HashMap<String, String>, SignInError> {
    let port_file = profile.join("DevToolsActivePort");
    // One left from an earlier run would name a port nothing answers on.
    let _ = tokio::fs::remove_file(&port_file).await;
    let _child = Command::new(browser)
        .arg(format!("--user-data-dir={}", profile.display()))
        .args([
            "--headless=new",
            "--no-first-run",
            "--remote-debugging-port=0",
            "about:blank",
        ])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .kill_on_drop(true)
        .spawn()
        .map_err(|e| SignInError::Launch(browser.to_owned(), e))?;
    let deadline = Instant::now() + Duration::from_secs(20);
    loop {
        if let Some(endpoint) = read_port_file(&port_file).await
            && let Ok((mut ws, _)) = tokio_tungstenite::connect_async(endpoint.as_str()).await
        {
            let request = serde_json::json!({ "id": 1, "method": "Storage.getCookies" });
            ws.send(Message::text(request.to_string()))
                .await
                .map_err(|e| SignInError::DevTools(e.to_string()))?;
            while let Some(Ok(msg)) = ws.next().await {
                let Message::Text(text) = msg else { continue };
                let value: serde_json::Value = serde_json::from_str(&text)
                    .map_err(|e| SignInError::DevTools(e.to_string()))?;
                if value["id"] == 1 {
                    let close = serde_json::json!({ "id": 2, "method": "Browser.close" });
                    let _ = ws.send(Message::text(close.to_string())).await;
                    return Ok(find_cookies(&value, site));
                }
            }
        }
        if Instant::now() > deadline {
            return Err(SignInError::DevTools(
                "the browser didn't open its DevTools".into(),
            ));
        }
        tokio::time::sleep(POLL).await;
    }
}

/// Ask for cookies until the token appears. `Ok(None)` when the connection drops,
/// which is the browser restarting or closing.
async fn poll_cookies<S>(
    ws: &mut S,
    site: Site<'_>,
    deadline: Instant,
    timeout: Duration,
) -> Result<Option<HashMap<String, String>>, SignInError>
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
            let value: serde_json::Value =
                serde_json::from_str(&text).map_err(|e| SignInError::DevTools(e.to_string()))?;
            // Events and stale replies share the socket; only ours matters.
            if value["id"] == request_id {
                break value;
            }
        };
        let cookies = find_cookies(&reply, site);
        if cookies.contains_key(site.token) {
            return Ok(Some(cookies));
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

/// The site's cookies among those of the reply, by name: the token and those
/// of `also`, the empty ones left out.
fn find_cookies(reply: &serde_json::Value, site: Site<'_>) -> HashMap<String, String> {
    let Some(cookies) = reply["result"]["cookies"].as_array() else {
        return HashMap::new();
    };
    cookies
        .iter()
        .filter(|c| {
            c["domain"]
                .as_str()
                .is_some_and(|d| d.trim_start_matches('.').ends_with(site.domain))
        })
        .filter_map(|c| Some((c["name"].as_str()?, c["value"].as_str()?)))
        .filter(|(name, value)| {
            !value.is_empty() && (*name == site.token || site.also.contains(name))
        })
        .map(|(name, value)| (name.to_owned(), value.to_owned()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const SITE: Site = Site {
        url: "https://music.apple.com/signin",
        domain: "music.apple.com",
        token: "media-user-token",
        also: &["refresh"],
    };

    #[test]
    fn the_cookies_are_picked_from_the_site_only() {
        let reply = serde_json::json!({ "id": 3, "result": { "cookies": [
            { "name": "media-user-token", "value": "elsewhere", "domain": ".example.com" },
            { "name": "itspod", "value": "47", "domain": ".music.apple.com" },
            { "name": "media-user-token", "value": "the-token", "domain": ".music.apple.com" },
            { "name": "refresh", "value": "again", "domain": "music.apple.com" },
        ]}});
        let cookies = find_cookies(&reply, SITE);
        assert_eq!(cookies.len(), 2);
        assert_eq!(cookies["media-user-token"], "the-token");
        assert_eq!(cookies["refresh"], "again");
    }

    #[test]
    fn an_empty_token_is_not_a_sign_in() {
        let reply = serde_json::json!({ "id": 1, "result": { "cookies": [
            { "name": "media-user-token", "value": "", "domain": ".music.apple.com" },
        ]}});
        assert!(!find_cookies(&reply, SITE).contains_key("media-user-token"));
    }
}
