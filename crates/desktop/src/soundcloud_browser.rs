//! SoundCloud's changes (likes, follows, playlists) sent as the browser the
//! user signed in with: its bot guard, DataDome, turns away any other
//! client's. They go out as Chrome sends them, with the guard's `datadome`
//! cookie of the sign-in profile, kept as the guard renews it. When the guard
//! asks for a check anyway, that browser opens for the user to pass it.

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use opium_core::{Error, Result};
use soundcloud::{Answer, Change};
use wreq_util::Emulation;

const SITE: browser_signin::Site<'static> = browser_signin::Site {
    url: "https://soundcloud.com/",
    domain: "soundcloud.com",
    token: "datadome",
    also: &[],
};

/// How long the user has to pass the check.
const CHECK_WITHIN: Duration = Duration::from_secs(600);

pub struct ChromeLike {
    http: wreq::Client,
    browser: OsString,
    profile: PathBuf,
    /// Where the cookie is kept between runs, readable by the user only.
    kept: PathBuf,
    /// The cookie, once read; held while a change is sent, one at a time.
    cookie: tokio::sync::Mutex<Option<String>>,
    /// Tells the user what to do in the browser.
    notify: Box<dyn Fn(String) + Send + Sync>,
}

impl ChromeLike {
    pub fn new(
        browser: OsString,
        profile: PathBuf,
        kept: PathBuf,
        notify: Box<dyn Fn(String) + Send + Sync>,
    ) -> Result<Self> {
        let http = wreq::Client::builder()
            .emulation(Emulation::Chrome147)
            .connect_timeout(Duration::from_secs(10))
            .read_timeout(Duration::from_secs(30))
            .build()
            .map_err(|e| Error::Network(e.to_string()))?;
        Ok(Self {
            http,
            browser,
            profile,
            kept,
            cookie: tokio::sync::Mutex::new(None),
            notify,
        })
    }

    /// The profile's cookie, as its browser last had it.
    async fn profile_cookie(&self) -> Option<String> {
        let mut cookies = browser_signin::read_cookies(self.browser.as_ref(), &self.profile, SITE)
            .await
            .ok()?;
        cookies.remove("datadome")
    }

    /// The cookie kept, or the profile's when its browser was used since.
    async fn first(&self) -> Option<String> {
        let changed = |p: &Path| std::fs::metadata(p).and_then(|m| m.modified()).ok();
        let kept = changed(&self.kept);
        let used = changed(&self.profile.join("Default/Cookies"));
        let from_kept = std::fs::read_to_string(&self.kept).ok().filter(|_| {
            kept.unwrap_or(SystemTime::UNIX_EPOCH) >= used.unwrap_or(SystemTime::UNIX_EPOCH)
        });
        match from_kept {
            Some(cookie) => Some(cookie.trim().to_owned()),
            None => self.profile_cookie().await,
        }
    }

    fn keep(&self, cookie: &str) {
        crate::backend::write_private(&self.kept, cookie.as_bytes()).ok();
    }

    async fn once(&self, change: &Change, cookie: &mut Option<String>) -> Result<Answer> {
        let method = wreq::Method::from_bytes(change.method.as_bytes())
            .map_err(|e| Error::Protocol(e.to_string()))?;
        let mut request = self
            .http
            .request(method, &change.url)
            .header("Origin", "https://soundcloud.com")
            .header("Referer", "https://soundcloud.com/");
        for (name, value) in &change.headers {
            request = request.header(name, value);
        }
        if let Some(cookie) = cookie.as_deref() {
            request = request.header("Cookie", format!("datadome={cookie}"));
        }
        if let Some(body) = &change.body {
            request = request.body(body.clone());
        }
        let resp = request
            .send()
            .await
            .map_err(|e| Error::Network(format!("SoundCloud: {e}")))?;
        // The guard renews its cookie with its answers: the next change
        // carries the new one.
        let renewed = resp.headers().get_all("set-cookie").iter().find_map(|v| {
            let rest = v.to_str().ok()?.strip_prefix("datadome=")?;
            Some(rest.split(';').next().unwrap_or("").to_owned())
        });
        if let Some(renewed) = renewed.filter(|c| !c.is_empty()) {
            self.keep(&renewed);
            *cookie = Some(renewed);
        }
        let status = resp.status().as_u16();
        let guard_header = resp.headers().contains_key("x-dd-b");
        let body = resp
            .text()
            .await
            .map_err(|e| Error::Network(format!("SoundCloud: {e}")))?;
        Ok(Answer {
            status,
            guard_header,
            body,
        })
    }
}

#[async_trait::async_trait]
impl soundcloud::Browser for ChromeLike {
    async fn send(&self, change: Change) -> Result<Answer> {
        let mut cookie = self.cookie.lock().await;
        if cookie.is_none() {
            *cookie = self.first().await;
        }
        let answer = self.once(&change, &mut cookie).await?;
        if !(answer.status == 403 && answer.guard_header) {
            return Ok(answer);
        }
        // The guard wants a check: in the browser, which it shows when a
        // song is liked there; then the change is sent again.
        (self.notify)(
            "SoundCloud wants a bot check: like a song in the window opened, pass the check, \
             then close it"
                .into(),
        );
        browser_signin::open_until_closed(
            self.browser.as_ref(),
            &self.profile,
            SITE.url,
            CHECK_WITHIN,
        )
        .await
        .map_err(|e| Error::Protocol(format!("SoundCloud's check: {e}")))?;
        *cookie = self.profile_cookie().await;
        if let Some(fresh) = cookie.as_deref() {
            self.keep(fresh);
        }
        self.once(&change, &mut cookie).await
    }
}
