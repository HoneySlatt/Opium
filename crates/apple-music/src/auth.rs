//! The developer token, read from Apple's web player as it carries it.
//!
//! The web player's page names its main script, anywhere in it, and the script is always
//! asked of the web player's own host; that script holds the token, a JWT (RFC 7519) signed ES256, among the rest of its code. Both are searched in one
//! pass each, in time proportional to their length: the script is about 3.3 MB and the token
//! lies about 0.9 MB into it.

use std::sync::LazyLock;
use std::time::{Duration, SystemTime};

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use opium_core::{Error, Result};
use regex::Regex;

/// The most of the page or the script that is read: both are a few megabytes, and a larger
/// answer is not the web player's.
const LARGEST_FILE: usize = 64 * 1024 * 1024;

/// The main script's path, `/assets/index~<name>.js` or `/assets/index-<name>.js`, wherever
/// it is written and whatever is written before it: `<name>` is one or more characters but
/// those that end an address's path or that address (`/`, `?`, `#`, a quote, `<`, `>`, an
/// ASCII space), dots included, as long as a `.js` can follow (`index~a1.min.js`). The regex
/// crate looks for its literal start with SIMD, which keeps the search of a page of megabytes
/// well under a millisecond.
static MAIN_SCRIPT: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"/assets/index[~-][^/?#"'<>\t\n\x0C\r ]+\.js"#).expect("a constant, valid pattern")
});

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
/// Each step that fails says which it is: the page out of reach, a page naming no main
/// script, the script out of reach, a script holding no token, or a token whose payload does
/// not decode. No connection, or an answer cut short, is an [`Error::Network`]; every other
/// failure, a page or a script answering a failure status (named) included, is an
/// [`Error::Protocol`]. No message carries the token.
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
        &script_address(web_player, script),
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
///
/// # Errors
///
/// [`Error::Network`] when the connection fails or the answer is cut short;
/// [`Error::Protocol`] for a failure status, named, or a body over [`LARGEST_FILE`].
async fn read(http: &reqwest::Client, url: &str, what: &str) -> Result<String> {
    let out_of_reach =
        |e: reqwest::Error| Error::Network(format!("{what} is out of reach: {}", e.without_url()));
    let mut resp = http.get(url).send().await.map_err(out_of_reach)?;
    let status = resp.status();
    // An answer, though a failure: a protocol error, which the renewal on a 401 waits after
    // rather than read the page again at every request.
    if !status.is_success() {
        return Err(Error::Protocol(format!(
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

/// The path and query of the main script `page` names: the first text anywhere in `page`, in
/// a tag or not, of the form `/assets/index~<name>.js` or `/assets/index-<name>.js` (see
/// [`MAIN_SCRIPT`]), with the query right after it, if any: a `?`, then what follows up to a
/// quote, a space, `<` or `>`.
///
/// Whatever comes before the path (a scheme and a host, `//`, `.`, a folder) is not part of
/// it: the script is asked of the web player's own host ([`script_address`]). What comes
/// after `.js`, but a query, is not looked at. One pass over `page`.
fn main_script(page: &str) -> Option<&str> {
    let found = MAIN_SCRIPT.find(page)?;
    let bytes = page.as_bytes();
    let mut end = found.end();
    if bytes.get(end) == Some(&b'?') {
        end += bytes[end..]
            .iter()
            .position(|&b| matches!(b, b'"' | b'\'' | b'<' | b'>') || b.is_ascii_whitespace())
            .unwrap_or(bytes.len() - end);
    }
    Some(&page[found.start()..end])
}

/// The address the main script is asked at: `script`, a path and a query [`main_script`]
/// found, on `web_player`'s host, whatever host the page wrote before it.
///
/// A `\` in the path is written `%5C`: an address's parser takes it for a `/` on `https`,
/// which would let the name step out of `/assets/` (`index~a\..\x.js`). With no `/` in the
/// name, the path is then `/assets/` and one segment, never a dot segment, starting with
/// `index`. The query is kept as the page writes it.
fn script_address(web_player: &str, script: &str) -> String {
    let (path, query) = match script.find('?') {
        Some(at) => script.split_at(at),
        None => (script, ""),
    };
    format!("{web_player}{}{query}", path.replace('\\', "%5C"))
}

/// Whether `path` is `/assets/index~<name>.js` or `/assets/index-<name>.js`, the name one or
/// more characters but `/`, `?`, `#`, a quote, `<`, `>` or an ASCII space.
#[cfg(test)]
fn is_main_script(path: &str) -> bool {
    let Some(name) = path
        .strip_prefix("/assets/index")
        .and_then(|rest| rest.strip_prefix('~').or_else(|| rest.strip_prefix('-')))
        .and_then(|rest| rest.strip_suffix(".js"))
    else {
        return false;
    };
    !name.is_empty()
        && !name.contains(['/', '?', '#', '"', '\'', '<', '>'])
        && !name.contains(|c: char| c.is_ascii_whitespace())
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

/// Entry points to this file's searches for the `cargo fuzz` targets, through
/// [`crate::fuzzing`].
#[cfg(feature = "fuzzing")]
pub(crate) mod fuzzing {
    /// The path and query of the main script `page` names.
    pub fn main_script(page: &str) -> Option<&str> {
        super::main_script(page)
    }

    /// The host, the path and the query, as an address's parser reads them, of the address
    /// the main script `script` names is asked at, on Apple's web player; `None` when that
    /// address does not parse.
    pub fn script_url(script: &str) -> Option<(String, String, Option<String>)> {
        let url = url::Url::parse(&super::script_address(crate::WEB_PLAYER, script)).ok()?;
        Some((
            url.host_str()?.to_owned(),
            url.path().to_owned(),
            url.query().map(str::to_owned),
        ))
    }

    /// The first ES256 JWT of `script`.
    pub fn first_token(script: &str) -> Option<&str> {
        super::first_token(script)
    }

    /// Whether `header`, a JWT's first segment, decodes to a JSON header naming
    /// `"alg": "ES256"`.
    pub fn names_es256(header: &str) -> bool {
        super::names_es256(header)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{Reply, TestServer, fake_token, reply, test_client};

    fn segment(json: &str) -> String {
        URL_SAFE_NO_PAD.encode(json)
    }

    #[test]
    fn the_main_script_is_the_first_named_in_either_form() {
        let page = r#"<html><head><link rel="modulepreload" href="/assets/vendor~v1.js">
<script crossorigin type="module" src="/assets/index~a1B2-c_3.js"></script></head></html>"#;
        assert_eq!(main_script(page), Some("/assets/index~a1B2-c_3.js"));
        let dashed = r#"<SCRIPT TYPE='module' defer src=/assets/index-Zx9.js>"#;
        assert_eq!(main_script(dashed), Some("/assets/index-Zx9.js"));
        // The first one named, whatever names it.
        let two = r#"<script src="/assets/index~first.js"></script>
<script type="module" src="/assets/index~second.js"></script>"#;
        assert_eq!(main_script(two), Some("/assets/index~first.js"));
    }

    #[test]
    fn the_main_script_is_found_anywhere_with_its_query() {
        for (page, expected) in [
            // In no tag at all, or in one cut short.
            ("import(\"/assets/index~ab.js\")", "/assets/index~ab.js"),
            (
                "<script type=\"module\" src=\"/assets/index~ab.js",
                "/assets/index~ab.js",
            ),
            ("<scripts src='/assets/index-ab.js'>", "/assets/index-ab.js"),
            (
                "<script type=\"text/javascript\" src=\"/assets/index~ab.js\">",
                "/assets/index~ab.js",
            ),
            ("/assets/index~ab.js", "/assets/index~ab.js"),
            // Right after the web player's address.
            (
                "<script src=\"https://music.apple.com/assets/index~ab.js\">",
                "/assets/index~ab.js",
            ),
            // Its query, up to a quote, a space, `<` or `>`.
            (
                "src=\"/assets/index~ab.js?v=2&x=%20\"",
                "/assets/index~ab.js?v=2&x=%20",
            ),
            ("src='/assets/index~ab.js?v=2'", "/assets/index~ab.js?v=2"),
            (
                "src=/assets/index~ab.js?v=2 async",
                "/assets/index~ab.js?v=2",
            ),
            ("src=/assets/index~ab.js?v=2>", "/assets/index~ab.js?v=2"),
            ("src=/assets/index~ab.js?v=2<", "/assets/index~ab.js?v=2"),
            ("/assets/index~ab.js?", "/assets/index~ab.js?"),
            ("/assets/index~ab.js#x", "/assets/index~ab.js"),
            // Past text that is not it.
            (
                "/assets/vendor~no.js /assets/index~/assets/index~yes.js",
                "/assets/index~yes.js",
            ),
        ] {
            assert_eq!(main_script(page), Some(expected), "{page}");
        }
    }

    #[test]
    fn other_scripts_name_none() {
        for page in [
            "",
            "<script",
            "<script type=\"module\" src=\"/assets/vendor~abc.js\">",
            "<script type=\"module\" src=\"/assets/index~.js\">",
            "<script type=\"module\" src=\"/assets/index.js\">",
            "<script type=\"module\" src=\"/assets/index_ab.js\">",
            "<script type=\"module\" src=\"/assets/index~ab/c.js\">",
            "<script type=\"module\" src=\"/asset/index~ab.js\">",
            "<script type=\"module\" src=\"/assets/index~a\"b.js\">",
            "<script type=\"module\" src='/assets/index~a'b.js'>",
            "<script type=\"module\" src=\"/assets/index~a b.js\">",
            "<script type=\"module\" src=\"/assets/index~a\tb.js\">",
            "<script type=\"module\" src=\"/assets/index~a?b.js\">",
            "<script type=\"module\" src=\"/assets/index~a#b.js\">",
            "<script type=\"module\" src=\"/assets/index~a<b.js\">",
            "<script type=\"module\" src=\"/assets/index~a>b.js\">",
        ] {
            assert_eq!(main_script(page), None, "{page}");
        }
    }

    #[test]
    fn the_main_script_is_found_whatever_comes_before_it() {
        for (page, expected) in [
            (
                "//music.apple.com/assets/index~ab.js",
                "/assets/index~ab.js",
            ),
            (
                "http://music.apple.com/assets/index~ab.js",
                "/assets/index~ab.js",
            ),
            (
                "https://js-cdn.music.apple.com/assets/index~ab.js?v=1",
                "/assets/index~ab.js?v=1",
            ),
            ("src=\"./assets/index-ab.js\"", "/assets/index-ab.js"),
            ("src=\"/us/assets/index~ab.js\"", "/assets/index~ab.js"),
            ("src=\"x/assets/index~ab.js\"", "/assets/index~ab.js"),
            (
                "src=\"https://elsewhere.example/assets/index~ab.js\"",
                "/assets/index~ab.js",
            ),
            (
                "src=\"//elsewhere.example:8080/cdn/assets/index~ab.js?x=1\"",
                "/assets/index~ab.js?x=1",
            ),
        ] {
            assert_eq!(main_script(page), Some(expected), "{page}");
        }
    }

    #[test]
    fn the_main_script_is_asked_of_the_web_players_host_at_its_path() {
        for (script, expected) in [
            (
                "/assets/index~ab.js",
                "https://music.apple.com/assets/index~ab.js",
            ),
            (
                "/assets/index-a1.min.js?v=1&x=/y",
                "https://music.apple.com/assets/index-a1.min.js?v=1&x=/y",
            ),
            (
                "/assets/index~a\\..\\..\\x.js?a\\b",
                "https://music.apple.com/assets/index~a%5C..%5C..%5Cx.js?a\\b",
            ),
        ] {
            let address = script_address(crate::WEB_PLAYER, script);
            assert_eq!(address, expected);
            let url = url::Url::parse(&address).unwrap();
            assert_eq!(url.host_str(), Some("music.apple.com"));
            assert!(url.path().starts_with("/assets/index"), "{}", url.path());
        }
    }

    #[test]
    fn the_main_scripts_name_may_hold_dots() {
        for (page, expected) in [
            ("src=\"/assets/index~a.b.js\"", "/assets/index~a.b.js"),
            ("src=\"/assets/index~a1.min.js\"", "/assets/index~a1.min.js"),
            (
                "src=/assets/index-a1.min.js?v=2>",
                "/assets/index-a1.min.js?v=2",
            ),
            ("src=\"/assets/index~..js\"", "/assets/index~..js"),
            // Any character but those that end an address's path.
            (
                "src=\"/assets/index~a%2Eb,c;d.js\"",
                "/assets/index~a%2Eb,c;d.js",
            ),
            ("src=\"/assets/index~été.js\"", "/assets/index~été.js"),
        ] {
            assert_eq!(main_script(page), Some(expected), "{page}");
        }
    }

    /// A web player whose page names its script as `named`, and serves the script at
    /// `/assets/index~a1.min.js?v=1` only, on its own host only.
    fn web_player_naming(named: &'static str, token: String) -> TestServer {
        TestServer::start(
            move |request| match (request.host_name(), request.path.as_str()) {
                ("music.apple.com", "/") => reply(
                    200,
                    &format!("<script type=module src=\"{named}\"></script>"),
                ),
                ("music.apple.com", "/assets/index~a1.min.js?v=1") => {
                    reply(200, &format!("t=\"{token}\""))
                }
                _ => reply(404, ""),
            },
        )
    }

    #[tokio::test]
    async fn the_main_script_is_always_asked_of_the_web_player() {
        let token = fake_token(1_900_000_000);
        for named in [
            "/assets/index~a1.min.js?v=1",
            "//music.apple.com/assets/index~a1.min.js?v=1",
            "http://music.apple.com/assets/index~a1.min.js?v=1",
            "https://music.apple.com/assets/index~a1.min.js?v=1",
            "https://js-cdn.music.apple.com/assets/index~a1.min.js?v=1",
            "https://elsewhere.example/assets/index~a1.min.js?v=1",
            "//elsewhere.example/assets/index~a1.min.js?v=1",
            "./assets/index~a1.min.js?v=1",
            "/us/assets/index~a1.min.js?v=1",
        ] {
            let server = web_player_naming(named, token.clone());
            let read = developer_token_at(&test_client(server.port()), &server.web_player())
                .await
                .unwrap_or_else(|e| panic!("{named}: {e}"));
            assert_eq!(read, token, "{named}");
            let asked: Vec<(String, String)> = server
                .requests()
                .into_iter()
                .map(|r| (r.host_name().to_owned(), r.path))
                .collect();
            assert_eq!(
                asked,
                [
                    ("music.apple.com".to_owned(), "/".to_owned()),
                    (
                        "music.apple.com".to_owned(),
                        "/assets/index~a1.min.js?v=1".to_owned()
                    ),
                ],
                "{named}"
            );
        }
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
                "<script type=module src=\"/assets/vendor~t.js\">",
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
                .unwrap_err();
            // A failure status too: the page or the script answered.
            assert!(matches!(error, Error::Protocol(_)), "{error}");
            let error = error.to_string();
            assert!(error.contains(expected), "{error} names {expected}");
        }
    }

    #[tokio::test]
    async fn a_script_out_of_reach_is_a_network_error() {
        let server = TestServer::start(|request| match request.path.as_str() {
            "/" => reply(200, "<script type=module src=/assets/index~t.js>"),
            _ => Reply::hang_up(),
        });
        let error = developer_token_at(&test_client(server.port()), &server.web_player())
            .await
            .unwrap_err();
        assert!(
            matches!(&error, Error::Network(m) if m.starts_with("the web player's main script is out of reach")),
            "{error}"
        );
    }

    #[tokio::test]
    async fn the_script_is_asked_at_the_path_and_query_the_page_names() {
        let token = fake_token(1_900_000_000);
        let script = format!("t=\"{token}\"");
        let server = TestServer::start(move |request| match request.path.as_str() {
            "/" => reply(
                200,
                "<link rel=preload href=\"https://music.apple.com/assets/index~q1.js?v=3&amp=1\">",
            ),
            "/assets/index~q1.js?v=3&amp=1" => reply(200, &script),
            _ => reply(404, ""),
        });
        let read = developer_token_at(&test_client(server.port()), &server.web_player())
            .await
            .unwrap();
        assert_eq!(read, token);
        assert!(
            server
                .requests()
                .iter()
                .all(|r| r.host_name() == "music.apple.com")
        );
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
    fn the_token_search_looks_at_each_byte_a_bounded_number_of_times() {
        let es256 = segment(r#"{"alg":"ES256"}"#);
        // Runs of headers that each fail a little further on.
        let script = format!("{}{es256}.a.b", "eyJeyJ.eyJ.".repeat(20_000));
        let mut steps = 0;
        assert!(first_token_counting(&script, &mut steps).is_some());
        assert!(steps <= 3 * script.len(), "{steps} for {}", script.len());
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
            "<script src=\"https://music.apple.com/assets/index~q1.js?v=3&l=en\" async>".to_owned(),
            "<script src=\"https://js-cdn.music.apple.com/us/assets/index~a1.min.js?v=1\">".to_owned(),
            "<link href=//elsewhere.example/assets/index-x.y.js><script src=./assets/index~a\\b.js>".to_owned(),
        ];
        let mut named = 0;
        for page in damaged_texts(0x9A6E, &pages) {
            if let Some(script) = main_script(&page) {
                named += 1;
                assert!(page.contains(script), "{script}");
                let (path, query) = script.split_once('?').unwrap_or((script, ""));
                assert!(is_main_script(path), "{script}");
                assert!(
                    !query.contains(['"', '\'', '<', '>'])
                        && !query.contains(|c: char| c.is_ascii_whitespace()),
                    "{script}"
                );
                // On the web player's host, at the path the page names, whatever the page
                // writes before it.
                let url = url::Url::parse(&script_address(crate::WEB_PLAYER, script))
                    .expect("an address");
                assert_eq!(url.host_str(), Some("music.apple.com"), "{script}");
                let file = url.path().strip_prefix("/assets/").expect("in /assets/");
                assert!(file.starts_with("index") && !file.contains('/'), "{script}");
            }
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
