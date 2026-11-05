//! The developer token, read from Apple's web player as it carries it.
//!
//! The web player's page names its main script; that script holds the token, a JWT
//! (RFC 7519) signed ES256, among the rest of its code. Both are searched in one pass each,
//! in time proportional to their length: the script is about 3.3 MB and the token lies
//! about 0.9 MB into it.

use std::sync::LazyLock;
use std::time::{Duration, SystemTime};

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use opium_core::{Error, Result};
use regex::Regex;

/// The most of the page or the script that is read: both are a few megabytes, and a larger
/// answer is not the web player's.
const LARGEST_FILE: usize = 64 * 1024 * 1024;

/// Where a `<script` tag starts. The regex crate looks for the literal with SIMD, which keeps
/// the search of a page of megabytes well under a millisecond.
static SCRIPT_TAG: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)<script").expect("a constant, valid pattern"));

/// Text shaped like a JWT whose header is a JSON object: `{"` in base64url, then three
/// base64url segments joined by dots. The regex crate finds it with a SIMD search for `eyJ`
/// and a DFA, in one pass whatever comes between.
static JWT_SHAPED: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"eyJ[A-Za-z0-9_-]*\.[A-Za-z0-9_-]+\.[A-Za-z0-9_-]+")
        .expect("a constant, valid pattern")
});

/// Reads the developer token from Apple's web player at `https://music.apple.com`: its page,
/// then the main script the page names, then the first ES256 JWT in that script.
///
/// `http` is the crate's client: its redirect policy follows Apple's 301 from `/` to a
/// storefront's page, and nothing off Apple's hosts.
///
/// # Errors
///
/// Each step that fails says which it is: the page out of reach ([`Error::Network`], with
/// its status when it answered one), a page naming no main script, the script out of reach
/// ([`Error::Network`]), a script holding no token, or a token whose payload does not decode
/// ([`Error::Protocol`] for these three). No message carries the token.
pub async fn developer_token(http: &reqwest::Client) -> Result<String> {
    developer_token_at(http, crate::WEB_PLAYER).await
}

/// [`developer_token`], from the web player at `web_player` (a scheme and a host, without a
/// final `/`): Apple's, but for the tests' local server.
pub(crate) async fn developer_token_at(http: &reqwest::Client, web_player: &str) -> Result<String> {
    let page = read(http, &format!("{web_player}/"), "the web player's page").await?;
    let script = main_script(&page)
        .ok_or_else(|| Error::Protocol("the web player's page names no main script".into()))?;
    let script = read(
        http,
        &format!("{web_player}{script}"),
        "the web player's main script",
    )
    .await?;
    let token = first_token(&script).ok_or_else(|| {
        Error::Protocol("the web player's main script holds no developer token".into())
    })?;
    if claims(token).is_none() {
        return Err(Error::Protocol(
            "the developer token's payload does not decode".into(),
        ));
    }
    Ok(token.to_owned())
}

/// When `token`, a JWT, expires: its payload's `exp`, in seconds since 1970 (RFC 7519
/// §4.1.4), whole or not.
///
/// `None` when `token` is not three dot-separated segments, when its payload is not
/// base64url-encoded JSON (RFC 7519 §3), or when its `exp` is missing, not a number, or
/// outside what [`SystemTime`] holds. Never panics.
pub fn expires_at(token: &str) -> Option<SystemTime> {
    let exp = claims(token)?.get("exp")?.clone();
    if let Some(seconds) = exp.as_u64() {
        return SystemTime::UNIX_EPOCH.checked_add(Duration::from_secs(seconds));
    }
    if let Some(seconds) = exp.as_i64() {
        return SystemTime::UNIX_EPOCH.checked_sub(Duration::from_secs(seconds.unsigned_abs()));
    }
    let seconds = exp.as_f64()?;
    let since = Duration::try_from_secs_f64(seconds.abs()).ok()?;
    if seconds >= 0.0 {
        SystemTime::UNIX_EPOCH.checked_add(since)
    } else {
        SystemTime::UNIX_EPOCH.checked_sub(since)
    }
}

/// `token`'s payload, the JWT's claims: a JSON object, base64url-encoded without padding.
/// `None` for anything else.
fn claims(token: &str) -> Option<serde_json::Map<String, serde_json::Value>> {
    let mut segments = token.split('.');
    let (Some(_), Some(payload), Some(_), None) = (
        segments.next(),
        segments.next(),
        segments.next(),
        segments.next(),
    ) else {
        return None;
    };
    match decoded_json(payload)? {
        serde_json::Value::Object(claims) => Some(claims),
        _ => None,
    }
}

/// `segment` as base64url without padding, then as JSON.
fn decoded_json(segment: &str) -> Option<serde_json::Value> {
    let bytes = URL_SAFE_NO_PAD.decode(segment).ok()?;
    serde_json::from_slice(&bytes).ok()
}

/// The body of `url`, a page or a script of the web player, as text. `what` names it in
/// errors.
async fn read(http: &reqwest::Client, url: &str, what: &str) -> Result<String> {
    let out_of_reach =
        |e: reqwest::Error| Error::Network(format!("{what} is out of reach: {}", e.without_url()));
    let mut resp = http.get(url).send().await.map_err(out_of_reach)?;
    let status = resp.status();
    if !status.is_success() {
        return Err(Error::Network(format!(
            "{what} is out of reach: HTTP {status}"
        )));
    }
    let mut body = Vec::new();
    while let Some(chunk) = resp.chunk().await.map_err(out_of_reach)? {
        if body.len() + chunk.len() > LARGEST_FILE {
            return Err(Error::Protocol(format!(
                "{what} is larger than {} MiB",
                LARGEST_FILE / (1024 * 1024)
            )));
        }
        body.extend_from_slice(&chunk);
    }
    Ok(String::from_utf8_lossy(&body).into_owned())
}

/// The path of the main script `page` names: the `src` of its first
/// `<script type="module">` that is `/assets/index~<hash>.js` or `/assets/index-<hash>.js`.
/// The path is on the web player's host.
fn main_script(page: &str) -> Option<&str> {
    main_script_counting(page, &mut 0)
}

/// [`main_script`], adding to `steps` how many bytes of `page` its own loops looked at:
/// tests bound it by the page's length.
fn main_script_counting<'a>(page: &'a str, steps: &mut usize) -> Option<&'a str> {
    let mut from = 0;
    while let Some(tag) = SCRIPT_TAG.find_at(page, from) {
        from = tag.end();
        // `<scripts`, say, is another tag.
        let name_ends = page
            .as_bytes()
            .get(tag.end())
            .is_none_or(|&b| b.is_ascii_whitespace() || b == b'>' || b == b'/');
        if !name_ends {
            continue;
        }
        let (attributes, end) = tag_attributes(page, tag.end(), steps);
        from = end;
        // A tag the page cuts short names nothing.
        let Some(attributes) = attributes else {
            continue;
        };
        let module = attributes.iter().any(|(name, value)| {
            name.eq_ignore_ascii_case("type") && value.eq_ignore_ascii_case("module")
        });
        let src = attributes
            .iter()
            .find(|(name, _)| name.eq_ignore_ascii_case("src"))
            .map(|(_, value)| *value);
        if module && let Some(path) = src.filter(|src| is_main_script(src)) {
            return Some(path);
        }
    }
    None
}

/// The attributes of the tag whose name ends at `from` in `page`, and where the tag ends:
/// just past its `>`. A tag that never closes has none, and ends with `page`. A value is
/// quoted with `"` or `'`, and may then hold a `>`, or runs to the next space or `>`. Adds the
/// bytes looked at to `steps`.
fn tag_attributes<'a>(
    page: &'a str,
    from: usize,
    steps: &mut usize,
) -> (Option<Vec<(&'a str, &'a str)>>, usize) {
    let bytes = page.as_bytes();
    let mut attributes = Vec::new();
    let mut at = from;
    // Every turn of each loop moves `at` forward, so the tag is read once.
    loop {
        while at < bytes.len() && (bytes[at].is_ascii_whitespace() || bytes[at] == b'/') {
            at += 1;
            *steps += 1;
        }
        if at >= bytes.len() {
            return (None, bytes.len());
        }
        if bytes[at] == b'>' {
            return (Some(attributes), at + 1);
        }
        let name_start = at;
        while at < bytes.len()
            && !matches!(bytes[at], b'=' | b'>' | b'/')
            && !bytes[at].is_ascii_whitespace()
        {
            at += 1;
            *steps += 1;
        }
        let name = &page[name_start..at];
        if at >= bytes.len() || bytes[at] != b'=' {
            // An attribute without a value (`async`, `crossorigin`), or a lone `/`.
            if at < bytes.len() && bytes[at] == b'/' {
                at += 1;
            }
            attributes.push((name, ""));
            continue;
        }
        at += 1;
        let value = match bytes.get(at) {
            Some(&quote @ (b'"' | b'\'')) => {
                let start = at + 1;
                let end = bytes[start..]
                    .iter()
                    .position(|&b| b == quote)
                    .map_or(bytes.len(), |n| start + n);
                *steps += end - at;
                at = (end + 1).min(bytes.len());
                &page[start..end]
            }
            _ => {
                let start = at;
                while at < bytes.len() && bytes[at] != b'>' && !bytes[at].is_ascii_whitespace() {
                    at += 1;
                    *steps += 1;
                }
                &page[start..at]
            }
        };
        attributes.push((name, value));
    }
}

/// Whether `src` is `/assets/index~<hash>.js` or `/assets/index-<hash>.js`, the hash one or
/// more ASCII letters, digits, `_` or `-`.
fn is_main_script(src: &str) -> bool {
    let Some(hash) = src
        .strip_prefix("/assets/index")
        .and_then(|rest| rest.strip_prefix('~').or_else(|| rest.strip_prefix('-')))
        .and_then(|rest| rest.strip_suffix(".js"))
    else {
        return false;
    };
    !hash.is_empty() && hash.bytes().all(is_base64url)
}

/// The first JWT of `script` whose header names `"alg": "ES256"`: three base64url segments
/// joined by dots, the first not part of a longer run of base64url characters. A JWT whose
/// header does not decode, or names another algorithm, is passed over.
fn first_token(script: &str) -> Option<&str> {
    first_token_counting(script, &mut 0)
}

/// [`first_token`], adding to `steps` how many bytes of the candidates its own loop looked
/// at: tests bound it by the script's length.
fn first_token_counting<'a>(script: &'a str, steps: &mut usize) -> Option<&'a str> {
    let bytes = script.as_bytes();
    let mut from = 0;
    while let Some(found) = JWT_SHAPED.find_at(script, from) {
        let start = found.start();
        let header_end = start + found.as_str().find('.').unwrap_or(found.len());
        *steps += found.len();
        // The next JWT may start after the header's dot, as its next segment: nowhere
        // inside the header, whose characters all follow a base64url one.
        from = header_end;
        let inside_a_run = start > 0 && is_base64url(bytes[start - 1]);
        if !inside_a_run && names_es256(&script[start..header_end]) {
            return Some(found.as_str());
        }
    }
    None
}

/// Whether `header`, a JWT's first segment, is a JSON object naming `"alg": "ES256"`.
fn names_es256(header: &str) -> bool {
    decoded_json(header).is_some_and(|header| header["alg"] == "ES256")
}

/// Whether `b` is of the base64url alphabet (RFC 4648 §5).
fn is_base64url(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'-' || b == b'_'
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{Reply, TestServer, fake_token, reply, test_client};

    fn segment(json: &str) -> String {
        URL_SAFE_NO_PAD.encode(json)
    }

    #[test]
    fn the_main_script_is_the_module_one_in_either_form() {
        let page = r#"<html><head><script src="/assets/index~old.js"></script>
<script crossorigin type="module" src="/assets/index~a1B2-c_3.js"></script></head></html>"#;
        assert_eq!(main_script(page), Some("/assets/index~a1B2-c_3.js"));
        let dashed = r#"<SCRIPT TYPE='module' defer src=/assets/index-Zx9.js>"#;
        assert_eq!(main_script(dashed), Some("/assets/index-Zx9.js"));
    }

    #[test]
    fn other_scripts_and_malformed_tags_name_none() {
        for page in [
            "",
            "<script",
            "<script type=\"module\" src=\"/assets/vendor~abc.js\">",
            "<script type=\"module\" src=\"/assets/index~.js\">",
            "<script type=\"module\" src=\"/assets/index~ab/c.js\">",
            "<script type=\"module\" src=\"https://elsewhere.example/assets/index~ab.js\">",
            "<script type=\"text/javascript\" src=\"/assets/index~ab.js\">",
            "<script type=\"module\" src=\"/assets/index~ab.js",
            "<scripts type=\"module\" src=\"/assets/index~ab.js\"",
            "<script type=\"module\" data-x=\"a>b\" src=\"/assets/index~ab.jsx\">",
        ] {
            assert_eq!(main_script(page), None, "{page}");
        }
        // A quoted `>` does not end the tag.
        let quoted = "<script data-x=\"a>b\" type=module src=\"/assets/index~ab.js\">";
        assert_eq!(main_script(quoted), Some("/assets/index~ab.js"));
    }

    #[test]
    fn the_first_es256_token_is_found_past_the_others() {
        let es256 = segment(r#"{"alg":"ES256","kid":"K"}"#);
        let hs256 = segment(r#"{"alg":"HS256"}"#);
        let payload = segment(r#"{"exp":1}"#);
        let script = format!(
            "var a=\"{hs256}.{payload}.c2ln\";var b=\"eyJnotbase64!.x.y\";\
             var c=x{es256}.{payload}.c2ln;var d=\"{es256}.{payload}.Zmlyc3Q\";\
             var e=\"{es256}.{payload}.c2Vjb25k\";"
        );
        let found = first_token(&script).unwrap();
        assert_eq!(found, format!("{es256}.{payload}.Zmlyc3Q"));
    }

    #[test]
    fn a_token_needs_three_segments() {
        let es256 = segment(r#"{"alg":"ES256"}"#);
        for script in [
            es256.clone(),
            format!("{es256}."),
            format!("{es256}.abc"),
            format!("{es256}.abc."),
            format!("{es256}..abc"),
        ] {
            assert_eq!(first_token(&script), None, "{script}");
        }
        // A dot before the header is not part of a longer run.
        let after_dot = format!("a.{es256}.abc.def");
        assert_eq!(first_token(&after_dot), Some(&after_dot[2..]));
    }

    #[test]
    fn its_expiry_is_read_from_the_payload() {
        let token = |payload: &str| {
            format!(
                "{}.{}.c2ln",
                segment(r#"{"alg":"ES256"}"#),
                segment(payload)
            )
        };
        let at = |seconds: u64| Some(SystemTime::UNIX_EPOCH + Duration::from_secs(seconds));
        assert_eq!(
            expires_at(&token(r#"{"exp":1800000000}"#)),
            at(1_800_000_000)
        );
        assert_eq!(
            expires_at(&token(r#"{"exp":1800000000.5}"#)),
            Some(SystemTime::UNIX_EPOCH + Duration::from_millis(1_800_000_000_500))
        );
        assert_eq!(expires_at(&token(r#"{"exp":0}"#)), at(0));
        for unreadable in [
            r#"{"iat":1}"#,
            r#"{"exp":"1800000000"}"#,
            r#"{"exp":null}"#,
            r#"{"exp":1e300}"#,
            r#"{"exp":18446744073709551615}"#,
            "[1]",
            "not json",
        ] {
            assert_eq!(expires_at(&token(unreadable)), None, "{unreadable}");
        }
        for malformed in ["", ".", "a.b", "a.b.c.d", "a.!!.c", "a..c"] {
            assert_eq!(expires_at(malformed), None, "{malformed}");
        }
    }

    /// The web player's page and script as the local server serves them, `/` redirected to a
    /// storefront's page as Apple does.
    fn web_player(token: String) -> TestServer {
        TestServer::start(move |request| match request.path.as_str() {
            "/" => reply(301, "").header("Location", "/xx/new"),
            "/xx/new" => reply(
                200,
                "<html><script type=\"module\" crossorigin src=\"/assets/index~t35t.js\"></script>",
            ),
            "/assets/index~t35t.js" => reply(200, &format!("let k=\"{token}\";")),
            _ => reply(404, ""),
        })
    }

    #[tokio::test]
    async fn the_token_is_read_from_the_page_and_its_script() {
        let token = fake_token(1_900_000_000);
        let server = web_player(token.clone());
        let read = developer_token_at(&test_client(server.port()), &server.web_player())
            .await
            .unwrap();
        assert_eq!(read, token);
        let paths: Vec<String> = server.requests().into_iter().map(|r| r.path).collect();
        assert_eq!(paths, ["/", "/xx/new", "/assets/index~t35t.js"]);
    }

    #[tokio::test]
    async fn each_failing_step_names_itself() {
        let es256 = segment(r#"{"alg":"ES256"}"#);
        let cases: [(&str, u16, &str, u16, String); 5] = [
            ("page", 503, "", 200, String::new()),
            (
                "names no main script",
                200,
                "<script src=\"/assets/index~t.js\">",
                200,
                String::new(),
            ),
            (
                "main script is out of reach: HTTP 404",
                200,
                "<script type=module src=/assets/index~t.js>",
                404,
                String::new(),
            ),
            (
                "holds no developer token",
                200,
                "<script type=module src=/assets/index~t.js>",
                200,
                "var x = 1;".into(),
            ),
            (
                "payload does not decode",
                200,
                "<script type=module src=/assets/index~t.js>",
                200,
                format!("\"{es256}.bm90IGpzb24.c2ln\""),
            ),
        ];
        for (expected, page_status, page, script_status, script) in cases {
            let page = page.to_owned();
            let script = script.clone();
            let server = TestServer::start(move |request| match request.path.as_str() {
                "/" => reply(page_status, &page),
                _ => reply(script_status, &script),
            });
            let error = developer_token_at(&test_client(server.port()), &server.web_player())
                .await
                .unwrap_err()
                .to_string();
            assert!(error.contains(expected), "{error} names {expected}");
        }
    }

    #[tokio::test]
    async fn a_page_out_of_reach_is_a_network_error() {
        // The connection closes without an answer.
        let server = TestServer::start(|_| Reply::hang_up());
        let error = developer_token_at(&test_client(server.port()), &server.web_player())
            .await
            .unwrap_err();
        assert!(
            matches!(&error, Error::Network(m) if m.starts_with("the web player's page is out of reach")),
            "{error}"
        );
    }

    #[tokio::test]
    async fn a_redirect_off_apples_hosts_is_not_followed() {
        let server = TestServer::start(|request| {
            if request.host.starts_with("elsewhere.example") {
                reply(200, "<script type=module src=/assets/index~t.js>")
            } else {
                let port = request.host.rsplit(':').next().unwrap_or("80").to_owned();
                reply(302, "").header("Location", &format!("http://elsewhere.example:{port}/"))
            }
        });
        let error = developer_token_at(&test_client(server.port()), &server.web_player())
            .await
            .unwrap_err()
            .to_string();
        assert!(error.contains("HTTP 302"), "{error}");
        assert!(
            server
                .requests()
                .iter()
                .all(|r| r.host.starts_with("music.apple.com")),
            "nothing asked of another host"
        );
    }

    #[test]
    fn the_searches_look_at_each_byte_a_bounded_number_of_times() {
        let es256 = segment(r#"{"alg":"ES256"}"#);
        // Runs of headers that each fail a little further on, and tags that never close.
        let script = format!("{}{es256}.a.b", "eyJeyJ.eyJ.".repeat(20_000));
        let mut steps = 0;
        assert!(first_token_counting(&script, &mut steps).is_some());
        assert!(steps <= 3 * script.len(), "{steps} for {}", script.len());
        let page = "<script a=\"".to_owned() + &"<script ".repeat(20_000);
        let mut steps = 0;
        assert_eq!(main_script_counting(&page, &mut steps), None);
        assert!(steps <= 2 * page.len(), "{steps} for {}", page.len());
    }
    /// A synthetic text of `len` bytes, read like minified JavaScript; with `decoys`, two
    /// strings in every 140 bytes that start like a JWT but are none.
    fn filler(len: usize, decoys: bool) -> String {
        let chunk = if decoys {
            "function q(e,t){return e.map(n=>n.id).filter(Boolean).concat(t)}\
             var s={eyJ:\"eyJub3RoaW5n\",k:\"a.b.c\"};const w=document.createElement(\"div\");"
        } else {
            "function q(e,t){return e.map(n=>n.id).filter(Boolean).concat(t)}\
             var s={key:\"value\",k:\"a.b.c\"};const w=document.createElement(\"div\");"
        };
        chunk.repeat(len / chunk.len() + 1)[..len].to_owned()
    }

    /// The median of 30 runs of `search`, after one to warm up.
    fn median(mut search: impl FnMut()) -> Duration {
        search();
        let mut times: Vec<Duration> = (0..30)
            .map(|_| {
                let start = std::time::Instant::now();
                search();
                start.elapsed()
            })
            .collect();
        times.sort();
        times[times.len() / 2]
    }

    /// The searches timed on files the size of Apple's on 2026-11-05: a page of 2.4 MB, and a
    /// script of 3.3 MB whose first token is about 0.9 MB in, with and without text before it
    /// that looks like a JWT's start. Printed, never asserted: run it in a release build
    /// (`cargo test --release -p apple-music the_searches_are_timed -- --nocapture`) for
    /// figures to set beside the earlier code's.
    #[test]
    fn the_searches_are_timed_on_files_the_size_of_apples() {
        let tag = "<script type=\"module\" crossorigin src=\"/assets/index~0f1e2d3c.js\"></script>";
        let head = format!(
            "<!DOCTYPE html><html lang=\"en\"><head><meta charset=\"utf-8\">{tag}</head><body>"
        );
        let page = format!("{head}{}", filler(2_400_000 - head.len(), false));
        let late_page = format!("<html>{}{tag}", filler(2_400_000, false));
        let token = fake_token(1_900_000_000);
        let script = |decoys: bool| {
            format!(
                "{}const a=\"{token}\";{}const b=\"{token}\";{}",
                filler(900_000, decoys),
                filler(1_200_000, decoys),
                filler(1_200_000, decoys)
            )
        };
        let (plain, decoyed) = (script(false), script(true));
        assert!(plain.len() > 3_300_000);
        let page_time = median(|| assert!(main_script(std::hint::black_box(&page)).is_some()));
        let late_time = median(|| assert!(main_script(std::hint::black_box(&late_page)).is_some()));
        let plain_time = median(|| {
            assert_eq!(
                first_token(std::hint::black_box(&plain)),
                Some(token.as_str())
            )
        });
        let decoyed_time = median(|| {
            assert_eq!(
                first_token(std::hint::black_box(&decoyed)),
                Some(token.as_str())
            )
        });
        println!(
            "token search, median of 30: page of {} bytes, its script named in its head \
             {page_time:?}, at its end {late_time:?}; script of {} bytes, the token {} bytes in, \
             {plain_time:?}, with JWT-like text every 70 bytes {decoyed_time:?}",
            page.len(),
            plain.len(),
            plain.find(&token).unwrap_or(0)
        );
    }
}

/// The page and the script fed damaged inputs: see `crate::mutation_tests`.
#[cfg(test)]
mod mutation_tests {
    use super::*;
    use crate::mutation_tests::damaged_texts;

    #[test]
    fn web_player_pages_and_scripts() {
        let token = crate::test_support::fake_token(1_900_000_000);
        let pages = [
            "<!doctype html><html><head><meta charset=utf-8><script type=\"module\" crossorigin src=\"/assets/index~a1b2c3.js\"></script></head></html>".to_owned(),
            "<script src=x></script><SCRIPT TYPE=module SRC='/assets/index-Zz_9.js'>".to_owned(),
        ];
        let mut named = 0;
        for page in damaged_texts(0x9A6E, &pages) {
            let mut steps = 0;
            if let Some(path) = main_script_counting(&page, &mut steps) {
                named += 1;
                assert!(is_main_script(path), "{path}");
            }
            assert!(
                steps <= 2 * page.len() + 16,
                "{steps} steps for {} bytes",
                page.len()
            );
        }
        assert!(named > 0, "some pages still name their script");
        let scripts = [
            format!(
                "var a=1,b=\"eyJub3QiOiJhIHRva2VuIn0.x.y\";const t=\"{token}\";var c=\"{token}\";"
            ),
            token.clone(),
        ];
        let mut found_any = 0;
        for script in damaged_texts(0x5C21, &scripts) {
            let mut steps = 0;
            if let Some(found) = first_token_counting(&script, &mut steps) {
                found_any += 1;
                let segments: Vec<&str> = found.split('.').collect();
                assert_eq!(segments.len(), 3, "{found}");
                assert!(segments.iter().all(|s| {
                    !s.is_empty()
                        && s.bytes()
                            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
                }));
                assert!(names_es256(segments[0]));
                let _ = expires_at(found);
            }
            assert!(
                steps <= 3 * script.len() + 16,
                "{steps} steps for {} bytes",
                script.len()
            );
            let _ = expires_at(&script);
        }
        assert!(found_any > 0, "some scripts still hold a token");
    }
}
