//! The web player's `client_id`, which every request to `api-v2` carries. It
//! is in the page itself (`window.__sc_hydration`, the one the web app uses)
//! and in one of its scripts; SoundCloud changes it every few months.

use music_core::{Error, Result};
use regex::Regex;

const HOME: &str = "https://soundcloud.com/";

/// Read the client id from soundcloud.com: the page's own first, else the
/// scripts', the last ones first (it is in one of the largest, at the end).
pub(crate) async fn scrape(http: &reqwest::Client) -> Result<String> {
    let page = text(http, HOME).await?;
    if let Some(id) = in_page(&page) {
        return Ok(id);
    }
    for script in scripts(&page).into_iter().rev() {
        if let Ok(js) = text(http, &script).await
            && let Some(id) = in_script(&js)
        {
            return Ok(id);
        }
    }
    Err(Error::Protocol(
        "SoundCloud: no client id on soundcloud.com".into(),
    ))
}

async fn text(http: &reqwest::Client, url: &str) -> Result<String> {
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

/// `{"hydratable":"apiClient","data":{"id":"…","isExpiring":…}}`.
fn in_page(page: &str) -> Option<String> {
    let re = Regex::new(r#""hydratable":"apiClient","data":\{"id":"([0-9A-Za-z]{32})""#).ok()?;
    Some(re.captures(page)?[1].to_owned())
}

/// `client_id:"…"`.
fn in_script(js: &str) -> Option<String> {
    let re = Regex::new(r#"client_id\s*:\s*"([0-9A-Za-z]{32})""#).ok()?;
    Some(re.captures(js)?[1].to_owned())
}

/// The page's scripts on SoundCloud's asset host, in page order.
fn scripts(page: &str) -> Vec<String> {
    let Ok(re) = Regex::new(r#"<script[^>]+src="(https://a-v2\.sndcdn\.com/assets/[^"]+\.js)""#)
    else {
        return Vec::new();
    };
    re.captures_iter(page).map(|c| c[1].to_owned()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const ID: &str = "ZeISTkUY1Q4so1FsHJKZaJo898iWk0d3";

    #[test]
    fn the_page_names_the_id_the_web_app_uses() {
        let page = format!(
            r#"window.__sc_hydration = [{{"hydratable":"apiClient","data":{{"id":"{ID}","isExpiring":true}}}}];"#
        );
        assert_eq!(in_page(&page).as_deref(), Some(ID));
    }

    #[test]
    fn a_script_names_one_too() {
        let js = format!(r#"a.set({{client_id:"{ID}",client_is_expiring:!1}})"#);
        assert_eq!(in_script(&js).as_deref(), Some(ID));
    }

    #[test]
    fn only_the_asset_host_scripts_are_read() {
        let page = r#"<script crossorigin src="https://a-v2.sndcdn.com/assets/0-4336d5f1.js"></script>
<script src="https://example.com/x.js"></script>
<script crossorigin src="https://a-v2.sndcdn.com/assets/55-4f297086.js"></script>"#;
        assert_eq!(
            scripts(page),
            [
                "https://a-v2.sndcdn.com/assets/0-4336d5f1.js",
                "https://a-v2.sndcdn.com/assets/55-4f297086.js"
            ]
        );
    }
}
