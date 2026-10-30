//! LRC, the text the lyrics sites share: `[mm:ss.xx]` before each line,
//! several when a line comes back, `[offset:±ms]` shifting them all, and
//! tags such as `[ar:…]` to skip.

use std::time::Duration;

use music_core::model::{LyricLine, LyricWord, Lyrics};

/// A line's times, from its leading `[mm:ss.xx]` tags, and its text; no
/// times for a tag such as `[ar:…]`.
fn timed(line: &str) -> (Vec<i64>, &str) {
    let mut times = Vec::new();
    let mut rest = line.trim();
    while let Some(tag) = rest.strip_prefix('[') {
        let Some((inside, after)) = tag.split_once(']') else {
            break;
        };
        let Some(ms) = millis(inside) else {
            break;
        };
        times.push(ms);
        rest = after;
    }
    (times, rest.trim())
}

/// `mm:ss.xx`, `mm:ss.xxx` or `mm:ss`, in milliseconds.
fn millis(tag: &str) -> Option<i64> {
    let (m, s) = tag.split_once(':')?;
    let minutes: i64 = m.trim().parse().ok()?;
    let (secs, frac) = s.split_once(['.', ':']).unwrap_or((s, ""));
    let seconds: i64 = secs.trim().parse().ok()?;
    let frac_ms = match frac.len() {
        0 => 0,
        1 => frac.parse::<i64>().ok()? * 100,
        2 => frac.parse::<i64>().ok()? * 10,
        _ => frac.get(..3)?.parse().ok()?,
    };
    Some(minutes * 60_000 + seconds * 1000 + frac_ms)
}

/// A line of enhanced LRC, `<mm:ss.xx>` before each word: its text without
/// them, and its words, each ending where the next begins and the last
/// where the line does. A line without them has no words.
fn enhanced(text: &str, line_end: Option<Duration>, shift: i64) -> (String, Vec<LyricWord>) {
    if !text.contains('<') {
        return (text.to_owned(), Vec::new());
    }
    let mut starts_texts: Vec<(i64, String)> = Vec::new();
    let mut plain = String::new();
    let mut rest = text;
    while !rest.is_empty() {
        let (at, after) = match rest.strip_prefix('<').and_then(|r| r.split_once('>')) {
            Some((tag, after)) => match millis(tag) {
                Some(ms) => (Some(ms - shift), after),
                None => (None, rest),
            },
            None => (None, rest),
        };
        let Some(at) = at else {
            // Text before the first tag, or a stray `<`: the line's own.
            let cut = rest[1..].find('<').map_or(rest.len(), |i| i + 1);
            plain.push_str(&rest[..cut]);
            if let Some((_, word)) = starts_texts.last_mut() {
                word.push_str(&rest[..cut]);
            }
            rest = &rest[cut..];
            continue;
        };
        let cut = after.find('<').unwrap_or(after.len());
        let word = &after[..cut];
        plain.push_str(word);
        if !word.is_empty() {
            starts_texts.push((at.max(0), word.to_owned()));
        }
        rest = &after[cut..];
    }
    let words = starts_texts
        .iter()
        .enumerate()
        .map(|(i, (at, word))| LyricWord {
            start: Duration::from_millis(*at as u64),
            end: starts_texts
                .get(i + 1)
                .map(|(next, _)| Duration::from_millis(*next as u64))
                .or(line_end)
                .unwrap_or(Duration::from_millis(*at as u64 + 1000)),
            text: word.clone(),
        })
        .collect();
    (
        plain.split_whitespace().collect::<Vec<_>>().join(" "),
        words,
    )
}

/// The `[offset:±ms]` tag: positive is earlier.
fn offset(text: &str) -> i64 {
    text.lines()
        .find_map(|l| {
            let l = l.trim();
            l.strip_prefix("[offset:")?
                .strip_suffix(']')?
                .trim()
                .parse()
                .ok()
        })
        .unwrap_or(0)
}

/// A credit some sites put among the lines: `作词 : …`, `Composer: …`, or
/// the song's title and artist at the very start.
fn is_credit(text: &str, at: i64, title: &str) -> bool {
    const CREDITS: [&str; 10] = [
        "作词",
        "作曲",
        "编曲",
        "制作",
        "混音",
        "母带",
        "词：",
        "曲：",
        "lyrics by",
        "composed by",
    ];
    let lower = text.to_lowercase();
    if CREDITS.iter().any(|c| lower.starts_with(c)) {
        return true;
    }
    at < 1000
        && !title.is_empty()
        && lower.starts_with(&title.to_lowercase())
        && lower.contains(" - ")
}

/// The lyrics of an LRC text, `title` telling its credit line apart. Lines
/// without times make them unsynced.
pub fn parse(text: &str, title: &str) -> Option<Lyrics> {
    let shift = offset(text);
    let mut timed_lines: Vec<(i64, String)> = Vec::new();
    let mut plain: Vec<String> = Vec::new();
    for line in text.lines() {
        let (times, words) = timed(line);
        if times.is_empty() {
            let words = line.trim();
            if !words.is_empty() && !words.starts_with('[') {
                plain.push(words.to_owned());
            }
            continue;
        }
        for at in times {
            let at = (at - shift).max(0);
            if !is_credit(words, at, title) {
                timed_lines.push((at, words.to_owned()));
            }
        }
    }
    if timed_lines.iter().any(|(_, w)| !w.is_empty()) {
        timed_lines.sort_by_key(|(at, _)| *at);
        let starts: Vec<i64> = timed_lines.iter().map(|(at, _)| *at).collect();
        let lines = timed_lines
            .into_iter()
            .enumerate()
            .map(|(i, (at, text))| {
                let end = starts.get(i + 1).map(|&e| Duration::from_millis(e as u64));
                let (text, words) = enhanced(&text, end, shift);
                LyricLine {
                    start: Duration::from_millis(at as u64),
                    end,
                    text,
                    words,
                }
            })
            .collect();
        return Some(Lyrics {
            lines,
            synced: true,
        });
    }
    (!plain.is_empty()).then(|| Lyrics {
        lines: plain
            .into_iter()
            .map(|text| LyricLine {
                start: Duration::ZERO,
                end: None,
                text,
                words: Vec::new(),
            })
            .collect(),
        synced: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lines_are_timed_sorted_and_shifted() {
        let text =
            "[ar:Drake]\n[offset:500]\n[00:20.50][01:10.00]Chorus\n[00:10.25]First\n[00:15.00]\n";
        let l = parse(text, "Song").unwrap();
        assert!(l.synced);
        let got: Vec<(u64, &str)> = l
            .lines
            .iter()
            .map(|x| (x.start.as_millis() as u64, x.text.as_str()))
            .collect();
        assert_eq!(
            got,
            [
                (9750, "First"),
                (14500, ""),
                (20000, "Chorus"),
                (69500, "Chorus")
            ]
        );
        assert_eq!(l.lines[0].end, Some(Duration::from_millis(14500)));
    }

    #[test]
    fn credits_are_left_out() {
        let text = "[00:00.00] 作词 : A Graham\n[00:00.00]Hold On - Drake\n[00:18.24]I got my eyes on you\n";
        let l = parse(text, "Hold On").unwrap();
        assert_eq!(l.lines.len(), 1);
        assert_eq!(l.lines[0].text, "I got my eyes on you");
    }

    #[test]
    fn enhanced_lines_are_timed_word_by_word() {
        let text = "[00:10.00]<00:10.00>Hello <00:10.50>world\n[00:12.00]Next\n";
        let l = parse(text, "x").unwrap();
        assert_eq!(l.lines[0].text, "Hello world");
        let words: Vec<(u64, u64, &str)> = l.lines[0]
            .words
            .iter()
            .map(|w| {
                (
                    w.start.as_millis() as u64,
                    w.end.as_millis() as u64,
                    w.text.as_str(),
                )
            })
            .collect();
        assert_eq!(
            words,
            [(10_000, 10_500, "Hello "), (10_500, 12_000, "world")]
        );
        assert!(l.lines[1].words.is_empty());
    }

    #[test]
    fn text_without_times_is_unsynced() {
        let l = parse("First\nSecond\n", "x").unwrap();
        assert!(!l.synced);
        assert_eq!(l.lines.len(), 2);
    }
}
