//! Signing in through a real browser to obtain the `media-user-token`: Apple's
//! sign-in page is a web flow (with 2FA), gone through in a browser of its own
//! (see `browser-signin`).

use std::path::Path;
use std::time::Duration;

pub use browser_signin::SignInError;

const SITE: browser_signin::Site = browser_signin::Site {
    url: "https://music.apple.com/signin",
    domain: "music.apple.com",
    token: "media-user-token",
    also: &[],
};

/// Open `browser` on Apple's sign-in page and wait for the user to finish.
///
/// `profile` is the browser profile dedicated to this; it is created if missing.
pub async fn sign_in(
    browser: &Path,
    profile: &Path,
    timeout: Duration,
) -> Result<String, SignInError> {
    let mut cookies = browser_signin::sign_in(browser, profile, SITE, timeout).await?;
    Ok(cookies.remove(SITE.token).unwrap_or_default())
}
