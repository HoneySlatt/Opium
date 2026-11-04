//! The developer token, borrowed from Apple's own web player.
//!
//! Apple's official route is a JWT signed with a MusicKit key from a paid
//! developer account. The web player ships one in its main JS bundle instead,
//! and this reads it from there — the approach Kopuz takes. It breaks whenever
//! Apple renames the bundle or moves the token, so failures say which step broke.

use std::sync::LazyLock;

use opium_core::{Error, Result};
use regex::Regex;

const WEB_PLAYER: &str = "https://music.apple.com";

static BUNDLE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"/assets/index~[^/]+\.js").expect("bundle regex"));
static JWT: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"eyJ[A-Za-z0-9\-_]+\.[A-Za-z0-9\-_]+\.[A-Za-z0-9\-_]+").expect("jwt regex")
});

pub async fn developer_token(http: &reqwest::Client) -> Result<String> {
    let html = fetch_text(http, WEB_PLAYER).await?;
    let bundle = BUNDLE
        .find(&html)
        .ok_or_else(|| Error::Protocol("no index~*.js bundle on music.apple.com".into()))?;
    let js = fetch_text(http, &format!("{WEB_PLAYER}{}", bundle.as_str())).await?;
    JWT.find(&js)
        .map(|m| m.as_str().to_owned())
        .ok_or_else(|| Error::Protocol("no developer token in the web player bundle".into()))
}

async fn fetch_text(http: &reqwest::Client, url: &str) -> Result<String> {
    let resp = http
        .get(url)
        .send()
        .await
        .map_err(|e| Error::Network(format!("{url}: {e}")))?;
    if !resp.status().is_success() {
        return Err(Error::Protocol(format!("{url}: HTTP {}", resp.status())));
    }
    resp.text()
        .await
        .map_err(|e| Error::Network(format!("{url}: {e}")))
}
