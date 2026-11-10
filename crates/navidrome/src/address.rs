//! A server's address, as the user types it, and the one it is reached at
//! ([ADR-0044](../../../docs/adr/0044-navidromes-password-as-it-is-used.md)).

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

use opium_core::{Error, Result};

/// The address `typed` is reached at, without its trailing slash.
///
/// A scheme typed at the start (`http://` or `https://`, in any case) is
/// always followed. An address without one is reached over HTTPS, unless it
/// is on the local network (see [`is_local`]), which is reached over HTTP, as
/// a server at home is. The host is a name (ASCII letters, digits, `.`, `-`
/// and `_`), an IPv4 address, or an IPv6 one (in brackets, or bare, without a
/// zone), with a port. The address built is read back by the URL parser,
/// which must find the scheme and the host that were classified.
///
/// # Errors
///
/// [`Error::Protocol`] for an address holding a user or a password
/// (see [`refuse_user_information`]); one with a space, a query (`?`) or a
/// fragment (`#`); a scheme other than `http` and `https`; and, with a scheme
/// typed or not, one with no host, a host or a port that is not as above, or
/// that the URL parser reads as another (`evil.com\.local`, `010.0.0.1`). The
/// message never repeats what was typed, which may be a password put in the
/// wrong field.
pub(crate) fn server_url(typed: &str) -> Result<String> {
    let typed = typed.trim().trim_end_matches('/');
    refuse_user_information(typed)?;
    if typed.chars().any(|c| c.is_whitespace() || c.is_control()) || typed.contains(['?', '#']) {
        return Err(malformed());
    }
    // A scheme typed is followed; the rest is checked as one typed without.
    let (typed_scheme, typed) = match typed.split_once("://").filter(|(s, _)| is_scheme(s)) {
        Some((s, rest)) if s.eq_ignore_ascii_case("http") => (Some("http"), rest),
        Some((s, rest)) if s.eq_ignore_ascii_case("https") => (Some("https"), rest),
        Some(_) => return Err(malformed()),
        None => (None, typed),
    };
    let authority = typed.split('/').next().unwrap_or_default();
    let path = &typed[authority.len()..];
    let (host, port) = split_host(authority).ok_or_else(malformed)?;
    let port_ok = port.is_none_or(|p| {
        p.bytes().all(|b| b.is_ascii_digit()) && p.parse::<u16>().is_ok_and(|n| n != 0)
    });
    if host.is_empty() || !port_ok || !is_host(host) {
        return Err(malformed());
    }
    let scheme = typed_scheme.unwrap_or(if is_local(host) { "http" } else { "https" });
    // An IPv6 address is bracketed, so that a port can follow it.
    let shown = if host.contains(':') {
        format!("[{host}]")
    } else {
        host.to_owned()
    };
    let port = port.map_or_else(String::new, |p| format!(":{p}"));
    let address = format!("{scheme}://{shown}{port}{path}");
    if !read_back_as(&address, scheme, host) {
        return Err(malformed());
    }
    Ok(address)
}

/// Whether `text` can be a scheme: a letter, then letters, digits, `+`, `-`
/// or `.`. What comes before a `://` that is not one is part of the address.
fn is_scheme(text: &str) -> bool {
    text.starts_with(|c: char| c.is_ascii_alphabetic())
        && text
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'+' | b'-' | b'.'))
}

/// Whether `host` is a name made of ASCII letters, digits, `.`, `-` and `_`,
/// or an IPv6 address (without a zone).
fn is_host(host: &str) -> bool {
    host.parse::<Ipv6Addr>().is_ok()
        || host
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'-' | b'_'))
}

/// Whether the URL parser (the one requests go through) finds `scheme` and
/// `host` in `address`: it reads a backslash as a slash, and a number as
/// another address, so what was classified might not be what is reached.
fn read_back_as(address: &str, scheme: &str, host: &str) -> bool {
    let Ok(url) = reqwest::Url::parse(address) else {
        return false;
    };
    let same_host = match (url.host_str(), host.parse::<Ipv6Addr>()) {
        (Some(found), Ok(ip)) => {
            found
                .strip_prefix('[')
                .and_then(|f| f.strip_suffix(']'))
                .and_then(|f| f.parse::<Ipv6Addr>().ok())
                == Some(ip)
        }
        (Some(found), Err(_)) => found.eq_ignore_ascii_case(host),
        (None, _) => false,
    };
    url.scheme() == scheme && same_host
}

/// Whether `url` holds user information: a user, with or without a
/// password, whatever its scheme (`https://me:pass@host`). Any `@` counts,
/// wherever it falls, so that a password holding a `/`, a `?` or a `#`
/// cannot slip into the path.
///
/// # Errors
///
/// [`Error::Protocol`], which names nothing that was typed: the password
/// would otherwise reach the screen and the logs. It is kept in no address
/// (every cover's address starts with the server's, and is kept and shared).
pub(crate) fn refuse_user_information(url: &str) -> Result<()> {
    if url.contains('@') {
        return Err(Error::Protocol(
            "the server's address holds a user or a password: give the address alone, \
             and the user and the password in their own fields"
                .into(),
        ));
    }
    Ok(())
}

fn malformed() -> Error {
    Error::Protocol(
        "the server's address is not valid: give it as a name or an address, \
         with a port if it has one, like nas.local:4533"
            .into(),
    )
}

/// `authority` split into its host (brackets off) and its port, if any.
/// `None` for an authority with more than one colon that is not an IPv6
/// address, a bracket left open, or brackets around anything but an IPv6
/// address.
fn split_host(authority: &str) -> Option<(&str, Option<&str>)> {
    if let Some(inner) = authority.strip_prefix('[') {
        let (host, after) = inner.split_once(']')?;
        host.parse::<Ipv6Addr>().ok()?;
        return match after {
            "" => Some((host, None)),
            _ => after.strip_prefix(':').map(|port| (host, Some(port))),
        };
    }
    if authority.parse::<Ipv6Addr>().is_ok() {
        return Some((authority, None));
    }
    match authority.split_once(':') {
        None => Some((authority, None)),
        Some((host, port)) if !port.contains(':') => Some((host, Some(port))),
        Some(_) => None,
    }
}

/// Whether `host` is on the local network, or the machine itself: a private
/// IPv4 address (10/8, 172.16/12, 192.168/16), a loopback one (127/8, `::1`),
/// the unspecified one (`0.0.0.0`, `::`: they name this machine, as loopback
/// does), an IPv6 link-local (fe80::/10) or unique local (fc00::/7) address,
/// `localhost`, or a name ending in `.local`. A trailing dot and the case are
/// ignored.
fn is_local(host: &str) -> bool {
    let host = host.to_ascii_lowercase();
    let host = host.strip_suffix('.').unwrap_or(&host);
    if let Ok(ip) = host.parse::<Ipv4Addr>() {
        return ip.is_private() || ip.is_loopback() || ip.is_unspecified();
    }
    if let Ok(ip) = host.parse::<Ipv6Addr>() {
        return IpAddr::V6(ip).is_loopback()
            || ip.is_unspecified()
            || ip.is_unicast_link_local()
            || ip.is_unique_local();
    }
    host == "localhost" || host.ends_with(".local")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reached(typed: &str) -> String {
        server_url(typed).unwrap_or_else(|e| panic!("{typed:?} refused: {e}"))
    }

    #[test]
    fn a_scheme_typed_is_followed_whatever_the_host() {
        assert_eq!(reached("http://example.org"), "http://example.org");
        assert_eq!(
            reached("https://192.168.1.5:4533"),
            "https://192.168.1.5:4533"
        );
        assert_eq!(reached("HTTP://nas.local/"), "http://nas.local");
        assert_eq!(reached("  http://[::1]:4533/ "), "http://[::1]:4533");
        assert_eq!(reached("https://nas/navidrome/"), "https://nas/navidrome");
    }

    #[test]
    fn a_scheme_counts_only_at_the_start() {
        assert_eq!(reached("nas.local/x://y"), "http://nas.local/x://y");
        assert_eq!(
            reached("music.example.org/x://y"),
            "https://music.example.org/x://y"
        );
        assert_eq!(reached("HtTpS://nas"), "https://nas");
    }

    #[test]
    fn a_name_alone_is_reached_over_https() {
        assert_eq!(reached("music.example.org"), "https://music.example.org");
        assert_eq!(
            reached("music.example.org:4533/"),
            "https://music.example.org:4533"
        );
        assert_eq!(
            reached(" example.org/navidrome "),
            "https://example.org/navidrome"
        );
        // Near the local names, not on them.
        assert_eq!(reached("locallocal"), "https://locallocal");
        assert_eq!(
            reached("nas.local.example.org"),
            "https://nas.local.example.org"
        );
        assert_eq!(reached("notlocalhost"), "https://notlocalhost");
        assert_eq!(
            reached("localhost.example.org"),
            "https://localhost.example.org"
        );
        assert_eq!(reached("local"), "https://local");
    }

    #[test]
    fn local_names_are_reached_over_http() {
        assert_eq!(reached("localhost"), "http://localhost");
        assert_eq!(reached("LocalHost:4533"), "http://LocalHost:4533");
        assert_eq!(reached("localhost./x"), "http://localhost./x");
        assert_eq!(reached("nas.local"), "http://nas.local");
        assert_eq!(reached("Nas.LOCAL:4533"), "http://Nas.LOCAL:4533");
        assert_eq!(reached("nas.local."), "http://nas.local.");
    }

    #[test]
    fn a_private_ipv4_address_is_local_at_each_edge_of_its_range() {
        for local in [
            "10.0.0.0",
            "10.255.255.255",
            "172.16.0.0",
            "172.31.255.255",
            "192.168.0.0",
            "192.168.255.255",
            "127.0.0.1",
            "127.255.255.254",
            "0.0.0.0",
        ] {
            assert_eq!(
                reached(&format!("{local}:4533")),
                format!("http://{local}:4533")
            );
        }
        for public in [
            "9.255.255.255",
            "11.0.0.0",
            "172.15.255.255",
            "172.32.0.0",
            "192.167.255.255",
            "192.169.0.0",
            "126.255.255.255",
            "128.0.0.0",
            "169.254.1.1",
            "100.64.0.1",
            "8.8.8.8",
        ] {
            assert_eq!(reached(public), format!("https://{public}"));
        }
    }

    #[test]
    fn an_ipv6_address_is_local_at_each_edge_of_its_range() {
        for local in [
            "::1",
            "::",
            "fe80::1",
            "febf:ffff::1",
            "fc00::1",
            "fdff:ffff::1",
        ] {
            assert_eq!(reached(&format!("[{local}]")), format!("http://[{local}]"));
            assert_eq!(
                reached(&format!("[{local}]:4533")),
                format!("http://[{local}]:4533")
            );
            // Bare, it is bracketed so that the address can be reached.
            assert_eq!(reached(local), format!("http://[{local}]"));
        }
        for public in [
            "fe7f:ffff::1",
            "fec0::1",
            "fbff:ffff::1",
            "fe00::1",
            "2001:db8::1",
            "::2",
        ] {
            assert_eq!(
                reached(&format!("[{public}]:4533")),
                format!("https://[{public}]:4533")
            );
            assert_eq!(reached(public), format!("https://[{public}]"));
        }
        assert_eq!(reached("[FE80::1]/x"), "http://[FE80::1]/x");
    }

    #[test]
    fn a_path_follows_the_host() {
        assert_eq!(reached("[::1]/music"), "http://[::1]/music");
        assert_eq!(reached("::1/music"), "http://[::1]/music");
        assert_eq!(
            reached("192.168.1.5:4533/navidrome/"),
            "http://192.168.1.5:4533/navidrome"
        );
    }

    #[test]
    fn user_information_is_refused_whatever_the_scheme_and_names_nothing_typed() {
        for typed in [
            "https://me:hunter2@music.example.org",
            "http://me:hunter2@192.168.1.5:4533",
            "me:hunter2@music.example.org",
            "me:hunter2@192.168.1.5",
            "https://me@music.example.org",
            "me@music.example.org",
            "https://:hunter2@music.example.org",
            "https://@music.example.org",
            "ftp://me:hunter2@nas",
            "https://me:p@ss@music.example.org",
            // A password holding a separator still ends up refused.
            "https://me:12/hunter2@music.example.org",
            "https://me:12?hunter2@music.example.org",
            "https://me:12#hunter2@music.example.org",
            "https://music.example.org/@hunter2",
            "https://me%40x:hunter2@music.example.org",
        ] {
            let said = match server_url(typed) {
                Err(Error::Protocol(said)) => said,
                other => panic!("{typed:?} was not refused: {other:?}"),
            };
            assert!(!said.contains("hunter2"), "{said}");
            assert!(!said.contains("me:"), "{said}");
        }
    }

    #[test]
    fn a_malformed_address_is_refused_without_repeating_it() {
        for typed in [
            "",
            "/",
            ":4533",
            "nas:",
            "nas:0",
            "nas:65536",
            "nas:+80",
            "nas:80a",
            "nas:80:90",
            "me:hunter2",
            "[::1",
            "[::1]x",
            "[::1]:",
            "[::1]:99999",
            "my nas",
            "nas/ x",
            "nas.local?x=1",
            "https://nas#x",
            "https://na\ns",
            "https://na s",
            // A backslash would end the host for the URL parser, not for us.
            "evil.com\\.local",
            "evil.com\\x.local:4533",
            "[evil.com\\.local]",
            "[nas.local]",
            // A zone cannot be reached.
            "[fe80::1%eth0]:4533",
            "fe80::1%eth0",
            "nas%2e.local",
            "na$s",
            "nas.local\\",
            // The URL parser would read these as other addresses.
            "010.0.0.1",
            "192.168.1",
            // Only http and https are schemes of a server.
            "ftp://nas",
            "javascript://x",
            "file:///etc",
            "://nas",
        ] {
            let said = match server_url(typed) {
                Err(Error::Protocol(said)) => said,
                other => panic!("{typed:?} was not refused: {other:?}"),
            };
            assert!(!said.contains("hunter2"), "{said}");
        }
    }

    #[test]
    fn a_kept_address_holding_user_information_is_refused() {
        assert!(refuse_user_information("https://music.example.org:4533/x").is_ok());
        assert!(refuse_user_information("https://me:hunter2@music.example.org").is_err());
        assert!(refuse_user_information("https://me@music.example.org").is_err());
    }

    #[test]
    fn a_scheme_typed_is_followed_and_its_host_checked_all_the_same() {
        assert_eq!(
            reached("HTTP://Music.example.org/"),
            "http://Music.example.org"
        );
        assert_eq!(
            reached("https://192.168.1.10:4533/nd"),
            "https://192.168.1.10:4533/nd"
        );
        assert_eq!(reached("http://[fe80::1]:4533"), "http://[fe80::1]:4533");
        for typed in [
            "http://evil.com\\.local",
            "https://[nas.local]",
            "http://",
            "https://:4533",
        ] {
            assert!(server_url(typed).is_err(), "{typed:?} accepted");
        }
    }
}
