//! Signing in at soundcloud.com in a browser of its own (see
//! `browser-signin`): the web player keeps the user's token in the
//! `oauth_token` cookie, short-lived since 2026, and the one that renews it in
//! `oauth_refresh_token`.

use std::path::Path;
use std::time::Duration;

pub use browser_signin::SignInError;

use crate::Tokens;

const SITE: browser_signin::Site = browser_signin::Site {
    url: "https://soundcloud.com/signin",
    domain: "soundcloud.com",
    token: "oauth_token",
    also: &["oauth_refresh_token"],
};

/// Open `browser` on SoundCloud's sign-in page and wait for the user to
/// finish. `profile` is the browser profile dedicated to this.
pub async fn sign_in(
    browser: &Path,
    profile: &Path,
    timeout: Duration,
) -> Result<Tokens, SignInError> {
    let mut cookies = browser_signin::sign_in(browser, profile, SITE, timeout).await?;
    Ok(Tokens {
        oauth: cookies.remove(SITE.token).unwrap_or_default(),
        refresh: cookies.remove("oauth_refresh_token"),
    })
}
