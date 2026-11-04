//! Synced lyrics from the sites that share them, for the songs a service
//! has none of: LRCLIB (open, made for this), NetEase Cloud Music and
//! KuGou. A song is matched by its title and artist, and by its length
//! within a few seconds, so that another version's lyrics aren't taken.

pub mod lrc;

use std::time::Duration;

use base64::Engine;
use opium_core::model::Lyrics;
use serde::{Deserialize, Serialize};

/// How much a song's length may differ from the lyrics' and still match.
const SAME_LENGTH: Duration = Duration::from_secs(3);
/// As LRCLIB asks its clients to say who they are.
const USER_AGENT: &str = "Opium (a GPL desktop player)";

/// Where lyrics come from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Source {
    Lrclib,
    NetEase,
    KuGou,
}

impl Source {
    /// Every source, in the order they are tried unless said otherwise.
    pub const ALL: [Source; 3] = [Source::Lrclib, Source::NetEase, Source::KuGou];

    pub fn name(self) -> &'static str {
        match self {
            Source::Lrclib => "LRCLIB",
            Source::NetEase => "NetEase Cloud Music",
            Source::KuGou => "KuGou",
        }
    }
}

/// The song lyrics are looked for.
#[derive(Debug, Clone, PartialEq)]
pub struct Query {
    pub title: String,
    /// The first artist.
    pub artist: String,
    pub album: Option<String>,
    pub duration: Option<Duration>,
}

impl Query {
    /// The title without what sites leave out of it: `(feat. …)`, `[…]`.
    fn bare_title(&self) -> &str {
        let t = self.title.as_str();
        // ASCII lowercasing keeps the byte offsets of the title's own.
        let lower = t.to_ascii_lowercase();
        let cut = ["(feat", "(ft.", "[feat", "(with ", " - feat"]
            .iter()
            .filter_map(|m| lower.find(m))
            .min()
            .unwrap_or(t.len());
        t[..cut].trim()
    }

    /// Whether a length found matches the song's: any, when it has none.
    fn same_length(&self, found: Duration) -> bool {
        self.duration
            .is_none_or(|d| d.abs_diff(found) <= SAME_LENGTH)
    }
}

/// Lowercase letters and digits only: what titles share across sites.
fn norm(s: &str) -> String {
    s.to_lowercase()
        .chars()
        .filter(|c| c.is_alphanumeric())
        .collect()
}

/// A title up to its first `(…)`, `[…]` or ` - …`: what sites agree on
/// when they tag versions and features differently.
fn core(title: &str) -> &str {
    let cut = title
        .find(['(', '['])
        .into_iter()
        .chain(title.find(" - "))
        .min();
    match cut.map(|at| title[..at].trim()) {
        Some(core) if !core.is_empty() => core,
        _ => title,
    }
}

/// Versions whose lyrics or timing differ from the song's.
const VERSIONS: [&str; 7] = [
    "remix",
    "live",
    "acoustic",
    "instrumental",
    "sped up",
    "slowed",
    "a cappella",
];

/// Whether a title found is the song's: the same, or the same up to its
/// tags, as long as neither is a version the other isn't.
fn same_title(query: &Query, found: &str) -> bool {
    let (want, got) = (norm(query.bare_title()), norm(found));
    if want.is_empty() || got.is_empty() {
        return false;
    }
    if got == want {
        return true;
    }
    // Whole words, so that "Alive" isn't live.
    let words = |t: &str| {
        format!(
            " {} ",
            t.to_lowercase()
                .replace(|c: char| !c.is_alphanumeric(), " ")
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" ")
        )
    };
    let (a, b) = (words(&query.title), words(found));
    VERSIONS.iter().all(|v| {
        let v = format!(" {v} ");
        a.contains(&v) == b.contains(&v)
    }) && norm(core(query.bare_title())) == norm(core(found))
}

pub struct Finder {
    http: reqwest::Client,
}

impl Default for Finder {
    fn default() -> Self {
        Self::new()
    }
}

impl Finder {
    pub fn new() -> Self {
        let http = reqwest::Client::builder()
            .user_agent(USER_AGENT)
            .timeout(Duration::from_secs(10))
            .build()
            .unwrap_or_default();
        Self { http }
    }

    /// The song's lyrics from the first of `sources` that has them synced;
    /// failing that, the first plain ones found. Line by line: the sites'
    /// timing of each word is too rough to follow smoothly, as Apple
    /// Music's is.
    pub async fn find(&self, query: &Query, sources: &[Source]) -> Option<(Lyrics, Source)> {
        let mut plain = None;
        for &source in sources {
            let found = match source {
                Source::Lrclib => self.lrclib(query).await,
                Source::NetEase => self.netease(query).await,
                Source::KuGou => self.kugou(query).await,
            }
            .map(line_by_line);
            match found {
                Some(lyrics) if lyrics.synced => return Some((lyrics, source)),
                Some(lyrics) if plain.is_none() => plain = Some((lyrics, source)),
                _ => {}
            }
        }
        plain
    }

    async fn json(&self, request: reqwest::RequestBuilder) -> Option<serde_json::Value> {
        let resp = request.send().await.ok()?;
        if !resp.status().is_success() {
            return None;
        }
        resp.json().await.ok()
    }

    /// LRCLIB's exact match, else its search by title and artist.
    async fn lrclib(&self, query: &Query) -> Option<Lyrics> {
        let mut exact = vec![
            ("track_name", query.title.clone()),
            ("artist_name", query.artist.clone()),
        ];
        if let Some(album) = &query.album {
            exact.push(("album_name", album.clone()));
        }
        if let Some(d) = query.duration {
            exact.push(("duration", d.as_secs().to_string()));
        }
        let got = self
            .json(self.http.get("https://lrclib.net/api/get").query(&exact))
            .await;
        if let Some(lyrics) = got.as_ref().and_then(|j| lrclib_lyrics(j, query)) {
            return Some(lyrics);
        }
        let found = self
            .json(self.http.get("https://lrclib.net/api/search").query(&[
                ("track_name", query.bare_title()),
                ("artist_name", query.artist.as_str()),
            ]))
            .await?;
        let entries = found.as_array()?;
        let fits = |e: &&serde_json::Value| {
            same_title(query, e["trackName"].as_str().unwrap_or_default())
                && e["duration"]
                    .as_f64()
                    .is_none_or(|d| query.same_length(Duration::from_secs_f64(d.max(0.0))))
        };
        // Synced first, else the first that fits.
        entries
            .iter()
            .filter(fits)
            .find(|e| e["syncedLyrics"].is_string())
            .or_else(|| entries.iter().find(fits))
            .and_then(|e| lrclib_lyrics(e, query))
    }

    /// NetEase's search, then the song's LRC.
    async fn netease(&self, query: &Query) -> Option<Lyrics> {
        let words = format!("{} {}", query.artist, query.bare_title());
        let found = self
            .json(
                self.http
                    .get("https://music.163.com/api/search/get")
                    .query(&[("s", words.as_str()), ("type", "1"), ("limit", "10")])
                    .header("Referer", "https://music.163.com"),
            )
            .await?;
        let song = found["result"]["songs"].as_array()?.iter().find(|s| {
            same_title(query, s["name"].as_str().unwrap_or_default())
                && s["duration"]
                    .as_u64()
                    .is_some_and(|ms| query.same_length(Duration::from_millis(ms)))
        })?;
        let id = song["id"].as_u64()?.to_string();
        let lyric = self
            .json(
                self.http
                    .get("https://music.163.com/api/song/lyric")
                    .query(&[("id", id.as_str()), ("lv", "1"), ("kv", "1"), ("tv", "-1")])
                    .header("Referer", "https://music.163.com"),
            )
            .await?;
        lrc::parse(lyric["lrc"]["lyric"].as_str()?, query.bare_title())
    }

    /// KuGou's candidates for the song, then the best one's LRC.
    async fn kugou(&self, query: &Query) -> Option<Lyrics> {
        let keyword = format!("{} - {}", query.artist, query.bare_title());
        let duration = query
            .duration
            .map(|d| d.as_millis().to_string())
            .unwrap_or_default();
        let found = self
            .json(self.http.get("https://lyrics.kugou.com/search").query(&[
                ("ver", "1"),
                ("man", "yes"),
                ("client", "pc"),
                ("keyword", keyword.as_str()),
                ("duration", duration.as_str()),
                ("hash", ""),
            ]))
            .await?;
        let candidate = found["candidates"].as_array()?.iter().find(|c| {
            same_title(query, c["song"].as_str().unwrap_or_default())
                && c["duration"]
                    .as_u64()
                    .is_some_and(|ms| query.same_length(Duration::from_millis(ms)))
        })?;
        let id = candidate["id"]
            .as_str()
            .map(str::to_owned)
            .or_else(|| candidate["id"].as_u64().map(|n| n.to_string()))?;
        let key = candidate["accesskey"].as_str()?;
        let got = self
            .json(self.http.get("https://lyrics.kugou.com/download").query(&[
                ("ver", "1"),
                ("client", "pc"),
                ("id", id.as_str()),
                ("accesskey", key),
                ("fmt", "lrc"),
                ("charset", "utf8"),
            ]))
            .await?;
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(got["content"].as_str()?)
            .ok()?;
        lrc::parse(&String::from_utf8_lossy(&bytes), query.bare_title())
    }
}

/// An LRCLIB entry's lyrics: synced when it has them, else plain.
fn lrclib_lyrics(entry: &serde_json::Value, query: &Query) -> Option<Lyrics> {
    if entry["instrumental"].as_bool() == Some(true) {
        return None;
    }
    entry["syncedLyrics"]
        .as_str()
        .and_then(|t| lrc::parse(t, query.bare_title()))
        .or_else(|| {
            entry["plainLyrics"]
                .as_str()
                .and_then(|t| lrc::parse(t, query.bare_title()))
        })
}

/// `lyrics` timed line by line only, each line's text as it is.
fn line_by_line(mut lyrics: Lyrics) -> Lyrics {
    for line in &mut lyrics.lines {
        line.words.clear();
    }
    lyrics
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_sites_lyrics_are_timed_line_by_line() {
        let timed = lrc::parse("[00:10.00]<00:10.00>Hello <00:10.50>world\n[00:12.00]Next\n", "Song").unwrap();
        assert!(!timed.lines[0].words.is_empty());
        let lines = line_by_line(timed);
        assert_eq!(lines.lines[0].text, "Hello world");
        assert!(lines.synced && lines.lines.iter().all(|l| l.words.is_empty()));
    }

    fn query(title: &str, secs: u64) -> Query {
        Query {
            title: title.into(),
            artist: "Artist".into(),
            album: None,
            duration: Some(Duration::from_secs(secs)),
        }
    }

    #[test]
    fn features_are_left_out_of_the_title_searched() {
        assert_eq!(query("Work (feat. Drake)", 200).bare_title(), "Work");
        assert_eq!(query("Plain", 200).bare_title(), "Plain");
    }

    #[test]
    fn a_match_shares_the_title_and_the_length() {
        let q = query("Hold On, We're Going Home", 227);
        assert!(same_title(&q, "Hold On We’re Going Home"));
        assert!(!same_title(&q, "Started From the Bottom"));
        assert!(same_title(&q, "Hold On, We're Going Home (Album Version)"));
        assert!(!same_title(&q, "Hold On, We're Going Home (Remix)"));
        assert!(!same_title(&query("Intro", 60), ""));
        assert!(!same_title(&query("Hold", 60), "Hold On"));
        assert!(same_title(&query("Alive", 60), "Alive"));
        assert!(q.same_length(Duration::from_millis(228_918)));
        assert!(!q.same_length(Duration::from_secs(154)));
    }
}
