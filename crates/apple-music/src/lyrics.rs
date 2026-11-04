//! Apple Music's lyrics: TTML, line-timed (`lyrics`) or word-timed
//! (`syllable-lyrics`). Both are read as `<p begin end>` paragraphs, the
//! word-timed ones keeping each `<span begin end>` syllable as a word of its
//! line; background vocals are left out.

use std::time::Duration;

use opium_core::model::{LyricLine, LyricWord, Lyrics};
use quick_xml::Reader;
use quick_xml::events::{BytesStart, Event};

/// A pause this long between lines shows as an instrumental gap.
const GAP: Duration = Duration::from_secs(5);

pub(crate) fn parse_ttml(ttml: &str) -> Option<Lyrics> {
    let synced = !ttml.contains("timing=\"None\"");
    let mut reader = Reader::from_str(ttml);
    let mut lines = Vec::new();
    // Inside a <p>: its times, its text and words so far.
    let mut paragraph: Option<Paragraph> = None;
    // Inside a timed <span>: the word being read.
    let mut word: Option<LyricWord> = None;
    // Depth inside a background-vocals span, whose text is skipped.
    let mut skipping = 0usize;

    loop {
        match reader.read_event().ok()? {
            Event::Start(e) => {
                if e.local_name().as_ref() == "p" {
                    paragraph = Some(Paragraph {
                        begin: time_attr(&e, "begin"),
                        end: time_attr(&e, "end"),
                        text: String::new(),
                        words: Vec::new(),
                    });
                } else if skipping > 0 || is_background(&e) {
                    skipping += 1;
                } else if paragraph.is_some()
                    && let (Some(begin), Some(end)) = (time_attr(&e, "begin"), time_attr(&e, "end"))
                {
                    word = Some(LyricWord {
                        start: begin,
                        end,
                        text: String::new(),
                    });
                }
            }
            Event::End(e) => {
                if e.local_name().as_ref() == "p" {
                    if let Some(p) = paragraph.take() {
                        let text = p.text.split_whitespace().collect::<Vec<_>>().join(" ");
                        if !text.is_empty() {
                            lines.push(LyricLine {
                                start: p.begin.unwrap_or_default(),
                                end: p.end,
                                text,
                                words: p.words,
                            });
                        }
                    }
                } else if skipping > 0 {
                    skipping -= 1;
                } else if let Some(w) = word.take()
                    && let Some(p) = &mut paragraph
                    && !w.text.trim().is_empty()
                {
                    p.words.push(w);
                }
            }
            Event::Text(t) if skipping == 0 => {
                let content = t.xml10_content();
                add_text(&mut paragraph, &mut word, &content);
            }
            Event::GeneralRef(r) if skipping == 0 => {
                let c = match r.resolve_char_ref() {
                    Ok(Some(c)) => c.to_string(),
                    _ => match r.xml10_content().as_ref() {
                        "amp" => "&",
                        "lt" => "<",
                        "gt" => ">",
                        "quot" => "\"",
                        "apos" => "'",
                        _ => "",
                    }
                    .to_owned(),
                };
                add_text(&mut paragraph, &mut word, &c);
            }
            Event::Eof => break,
            _ => {}
        }
    }
    if lines.is_empty() {
        return None;
    }
    if synced {
        lines = with_gaps(lines);
    }
    Some(Lyrics { lines, synced })
}

/// A `<p>` being read.
struct Paragraph {
    begin: Option<Duration>,
    end: Option<Duration>,
    text: String,
    words: Vec<LyricWord>,
}

/// Text inside a paragraph: its word's when in one, else between words,
/// where a space goes to the word before.
fn add_text(paragraph: &mut Option<Paragraph>, word: &mut Option<LyricWord>, text: &str) {
    let Some(p) = paragraph else {
        return;
    };
    p.text.push_str(text);
    // Line breaks and runs of spaces are one space.
    let flat: String = text
        .chars()
        .map(|c| if c.is_whitespace() { ' ' } else { c })
        .collect();
    match word {
        Some(w) => w.text.push_str(&flat),
        None => {
            if let Some(last) = p.words.last_mut()
                && !last.text.ends_with(' ')
            {
                let gap = flat.trim();
                if flat.starts_with(' ') || gap.is_empty() {
                    last.text.push(' ');
                }
                last.text.push_str(gap);
            }
        }
    }
}

fn is_background(e: &BytesStart) -> bool {
    e.attributes()
        .flatten()
        .any(|a| a.key.local_name().as_ref() == "role" && a.value == "x-bg")
}

fn time_attr(e: &BytesStart, name: &str) -> Option<Duration> {
    let attr = e
        .attributes()
        .flatten()
        .find(|a| a.key.local_name().as_ref() == name)?;
    parse_time(&attr.value)
}

/// `83.456`, `1:23.456`, `0:01:23.456`, with or without a trailing `s`.
fn parse_time(s: &str) -> Option<Duration> {
    let s = s.trim().trim_end_matches('s');
    let mut seconds = 0.0;
    for part in s.split(':') {
        seconds = seconds * 60.0 + part.parse::<f64>().ok()?;
    }
    (seconds >= 0.0).then(|| Duration::from_secs_f64(seconds))
}

/// Mark long pauses (and a long intro) with an empty line, shown as dots.
fn with_gaps(lines: Vec<LyricLine>) -> Vec<LyricLine> {
    let mut out = Vec::with_capacity(lines.len() + 4);
    let mut previous_end = Duration::ZERO;
    for line in lines {
        if line.start >= previous_end + GAP {
            out.push(LyricLine {
                start: previous_end,
                end: Some(line.start),
                text: String::new(),
                words: Vec::new(),
            });
        }
        previous_end = line.end.unwrap_or(line.start);
        out.push(line);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const LINE_TIMED: &str = r#"<tt xmlns="http://www.w3.org/ns/ttml" xmlns:itunes="http://music.apple.com/lyric-ttml-internal" itunes:timing="Line"><body><div>
<p begin="7.2" end="9.9">First line &amp; more</p>
<p begin="10.1" end="12.0">Second <span>line</span></p>
<p begin="1:20.500" end="1:23.000">After the break</p>
</div></body></tt>"#;

    #[test]
    fn line_timed_lyrics_read_with_their_times() {
        let lyrics = parse_ttml(LINE_TIMED).unwrap();
        assert!(lyrics.synced);
        let texts: Vec<&str> = lyrics.lines.iter().map(|l| l.text.as_str()).collect();
        // A 7 s intro and a long break become gaps.
        assert_eq!(
            texts,
            [
                "",
                "First line & more",
                "Second line",
                "",
                "After the break"
            ]
        );
        assert_eq!(lyrics.lines[1].start, Duration::from_millis(7200));
        assert_eq!(lyrics.lines[4].start, Duration::from_millis(80_500));
        assert_eq!(lyrics.lines[3].start, Duration::from_secs(12));
    }

    #[test]
    fn word_timed_spans_join_into_lines_without_background_vocals() {
        let ttml = r#"<tt xmlns:ttm="http://www.w3.org/ns/ttml#metadata" itunes:timing="Word"><body><div>
<p begin="1.0" end="3.0"><span begin="1.0" end="1.5">Hello</span> <span begin="1.6" end="2.0">world</span><span ttm:role="x-bg"><span begin="2.1" end="2.5">(ooh)</span></span></p>
</div></body></tt>"#;
        let lyrics = parse_ttml(ttml).unwrap();
        let line = lyrics.lines.last().unwrap();
        assert_eq!(line.text, "Hello world");
        let words: Vec<(&str, u64)> = line
            .words
            .iter()
            .map(|w| (w.text.as_str(), w.start.as_millis() as u64))
            .collect();
        assert_eq!(words, [("Hello ", 1000), ("world", 1600)]);
    }

    #[test]
    fn syllables_of_one_word_stay_joined() {
        let ttml = r#"<tt itunes:timing="Word"><body><div>
<p begin="1.0" end="3.0"><span begin="1.0" end="1.3">Ex</span><span begin="1.3" end="1.6">pect</span> <span begin="1.7" end="2.0">it</span></p>
</div></body></tt>"#;
        let line = parse_ttml(ttml).unwrap().lines.pop().unwrap();
        let texts: Vec<&str> = line.words.iter().map(|w| w.text.as_str()).collect();
        assert_eq!(texts, ["Ex", "pect ", "it"]);
        assert_eq!(line.text, "Expect it");
    }

    #[test]
    fn unsynced_lyrics_keep_their_lines_and_no_gaps() {
        let ttml = r#"<tt itunes:timing="None"><body><div><p>One</p><p>Two</p></div></body></tt>"#;
        let lyrics = parse_ttml(ttml).unwrap();
        assert!(!lyrics.synced);
        assert_eq!(lyrics.lines.len(), 2);
        assert_eq!(lyrics.current(Duration::from_secs(5)), None);
    }

    #[test]
    fn times_parse_in_every_form() {
        assert_eq!(parse_time("83.456"), Some(Duration::from_millis(83_456)));
        assert_eq!(parse_time("1:23.456"), Some(Duration::from_millis(83_456)));
        assert_eq!(
            parse_time("0:01:23.456"),
            Some(Duration::from_millis(83_456))
        );
        assert_eq!(parse_time("12.5s"), Some(Duration::from_millis(12_500)));
        assert_eq!(parse_time("x"), None);
    }
}
