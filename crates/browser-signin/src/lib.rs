//! Signing in to a service through a real browser, to obtain the cookies its
//! web player keeps the user's tokens in.
//!
//! Sign-in pages are web flows (with 2FA, captchas), so the user goes through
//! them in a Chromium-family browser running on its own profile. The browser's
//! DevTools are asked for its cookies until the token shows up — reading them
//! live avoids decrypting Chromium's on-disk cookie store, whose format and key
//! storage keep changing. They are reached over two pipes only the browser and
//! we hold (`--remote-debugging-pipe`), not a port any program on the machine
//! could connect to while the sign-in lasts.
//!
//! The profile is kept between sign-ins: when a token expires, a browser that
//! is still signed in hands over a fresh one without any typing.

use std::collections::HashMap;
use std::os::fd::{AsRawFd, OwnedFd};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::unix::pipe;
use tokio::process::{Child, Command};

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

/// How often the browser is asked for its cookies, and looked at.
const POLL: Duration = Duration::from_millis(500);

#[derive(Debug, thiserror::Error)]
pub enum SignInError {
    #[error("could not start the browser `{0}`: {1}")]
    Launch(PathBuf, std::io::Error),
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
/// The browser closes itself once the token is there. Chromium may also carry
/// on in another process (it restarts on a fresh profile, and hands off to an
/// instance already running on the profile), which ends our pipes: then the
/// cookies are read once that browser is closed.
pub async fn sign_in(
    browser: &Path,
    profile: &Path,
    site: Site<'_>,
    timeout: Duration,
) -> Result<HashMap<String, String>, SignInError> {
    let deadline = Instant::now() + timeout;
    let mut devtools = DevTools::launch(
        browser,
        profile,
        &[
            "--no-first-run",
            "--no-default-browser-check",
            &format!("--app={}", site.url),
        ],
    )?;
    while let Ok(reply) = devtools.call("Storage.getCookies", None).await {
        let cookies = find_cookies(&reply, site);
        if cookies.contains_key(site.token) {
            devtools.close().await;
            return Ok(cookies);
        }
        if Instant::now() > deadline {
            return Err(SignInError::TimedOut(timeout));
        }
        tokio::time::sleep(POLL).await;
    }
    // Closed, or carried on elsewhere: its cookies say once it is done.
    wait_until_closed(&mut devtools.child, profile, deadline, timeout).await?;
    signed_in(read_cookies(browser, profile, site).await?, site)
}

/// The same, for sites that may turn a browser away when its DevTools are
/// open (SoundCloud's bot guard has): the browser opens as any other, and once
/// the user has signed in and closed it, it is started again unseen, on no
/// page, only to hand the cookies over.
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
    wait_until_closed(&mut child, profile, Instant::now() + timeout, timeout).await?;
    signed_in(read_cookies(browser, profile, site).await?, site)
}

fn signed_in(
    cookies: HashMap<String, String>,
    site: Site<'_>,
) -> Result<HashMap<String, String>, SignInError> {
    if cookies.contains_key(site.token) {
        Ok(cookies)
    } else {
        Err(SignInError::NotSignedIn)
    }
}

/// Wait for the browser on `profile` to be closed. The process we spawn is
/// not a reliable signal (see [`sign_in`]): it counts as closed once our
/// process is gone and nothing holds the profile's lock any more.
async fn wait_until_closed(
    child: &mut Child,
    profile: &Path,
    deadline: Instant,
    timeout: Duration,
) -> Result<(), SignInError> {
    let lock = profile.join("SingletonLock");
    loop {
        let exited = matches!(child.try_wait(), Ok(Some(_)));
        if exited && tokio::fs::symlink_metadata(&lock).await.is_err() {
            return Ok(());
        }
        if Instant::now() > deadline {
            return Err(SignInError::TimedOut(timeout));
        }
        tokio::time::sleep(POLL).await;
    }
}

/// The site's cookies in `profile`, from the browser started headless on no
/// page.
async fn read_cookies(
    browser: &Path,
    profile: &Path,
    site: Site<'_>,
) -> Result<HashMap<String, String>, SignInError> {
    let mut devtools = DevTools::launch(
        browser,
        profile,
        &["--headless=new", "--no-first-run", "about:blank"],
    )?;
    let reply = devtools.call("Storage.getCookies", None).await?;
    devtools.close().await;
    Ok(find_cookies(&reply, site))
}

/// How long the browser may take to answer one call.
const ANSWER_WITHIN: Duration = Duration::from_secs(20);

/// A browser and its DevTools, over `--remote-debugging-pipe`: it reads
/// calls on its descriptor 3 and writes answers on 4, each a JSON message
/// ended by a NUL.
struct DevTools {
    child: Child,
    calls: pipe::Sender,
    answers: BufReader<pipe::Receiver>,
    next_id: u64,
}

impl DevTools {
    fn launch(browser: &Path, profile: &Path, args: &[&str]) -> Result<Self, SignInError> {
        let launch = |e: std::io::Error| SignInError::Launch(browser.to_owned(), e);
        let (their_calls, calls) = std::io::pipe().map_err(launch)?;
        let (answers, their_answers) = std::io::pipe().map_err(launch)?;
        let (read_fd, write_fd) = (their_calls.as_raw_fd(), their_answers.as_raw_fd());
        let mut command = Command::new(browser);
        command
            .arg(format!("--user-data-dir={}", profile.display()))
            .arg("--remote-debugging-pipe")
            .args(args)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .kill_on_drop(true);
        // SAFETY: only async-signal-safe calls between fork and exec. The
        // pipes go to 3 and 4 by way of copies above both, so that neither
        // lands on the other's place, and without close-on-exec.
        unsafe {
            command.pre_exec(move || {
                let read = libc::fcntl(read_fd, libc::F_DUPFD, 10);
                let write = libc::fcntl(write_fd, libc::F_DUPFD, 10);
                if read < 0 || write < 0 || libc::dup2(read, 3) < 0 || libc::dup2(write, 4) < 0 {
                    return Err(std::io::Error::last_os_error());
                }
                libc::close(read);
                libc::close(write);
                Ok(())
            });
        }
        let child = command.spawn().map_err(launch)?;
        // The browser's ends are its own now.
        drop((their_calls, their_answers));
        let devtools = |e: std::io::Error| SignInError::DevTools(e.to_string());
        Ok(Self {
            child,
            calls: pipe::Sender::from_owned_fd(OwnedFd::from(calls)).map_err(devtools)?,
            answers: BufReader::new(
                pipe::Receiver::from_owned_fd(OwnedFd::from(answers)).map_err(devtools)?,
            ),
            next_id: 1,
        })
    }

    /// Call `method`, and wait for its answer; an error once the browser
    /// has gone.
    async fn call(
        &mut self,
        method: &str,
        params: Option<serde_json::Value>,
    ) -> Result<serde_json::Value, SignInError> {
        let id = self.next_id;
        self.next_id += 1;
        let mut call = serde_json::json!({ "id": id, "method": method });
        if let Some(params) = params {
            call["params"] = params;
        }
        let mut message = call.to_string().into_bytes();
        message.push(0);
        let gone = |e: std::io::Error| SignInError::DevTools(e.to_string());
        self.calls.write_all(&message).await.map_err(gone)?;
        let answer = async {
            let mut buf = Vec::new();
            loop {
                buf.clear();
                if self.answers.read_until(0, &mut buf).await.map_err(gone)? == 0 {
                    return Err(SignInError::DevTools("the browser closed".into()));
                }
                buf.pop_if(|b| *b == 0);
                let value: serde_json::Value = serde_json::from_slice(&buf)
                    .map_err(|e| SignInError::DevTools(e.to_string()))?;
                // Events share the pipe; only our answer matters.
                if value["id"] == id {
                    return Ok(value);
                }
            }
        };
        tokio::time::timeout(ANSWER_WITHIN, answer)
            .await
            .map_err(|_| SignInError::DevTools("the browser didn't answer".into()))?
    }

    async fn close(&mut self) {
        let _ = self.call("Browser.close", None).await;
        let _ = tokio::time::timeout(ANSWER_WITHIN, self.child.wait()).await;
    }
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
                .map(|d| d.trim_start_matches('.'))
                .is_some_and(|d| {
                    d == site.domain
                        || d.strip_suffix(site.domain)
                            .is_some_and(|sub| sub.ends_with('.'))
                })
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
