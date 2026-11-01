//! Recently Added, every service's in one list: by when each item came
//! into the library, whichever service it is on, each service's own order
//! kept (Apple's puts an album a song was just added to first).

use std::collections::VecDeque;

use crate::model::{Collection, ProviderKind};

/// A service's items in its order, each with when it was added.
type Group = Vec<(Option<i64>, Collection)>;

/// The services' Recently Added, merged by when each item was added. One
/// whose service gives no time stays under the item above it.
pub fn in_time_order(items: Vec<Collection>) -> Vec<Collection> {
    let mut groups: Vec<(ProviderKind, Group)> = Vec::new();
    for item in items {
        let provider = provider(&item);
        let at = added(&item);
        match groups.iter_mut().find(|(p, _)| *p == provider) {
            Some((_, group)) => group.push((at, item)),
            None => groups.push((provider, vec![(at, item)])),
        }
    }
    let total = groups.iter().map(|(_, g)| g.len()).sum();
    let mut queues: Vec<VecDeque<(i64, Collection)>> = groups
        .into_iter()
        .map(|(_, group)| {
            // At the top, before any time, the first one below.
            let mut carry = group.iter().find_map(|(at, _)| *at).unwrap_or(i64::MIN);
            group
                .into_iter()
                .map(|(at, item)| {
                    carry = at.unwrap_or(carry);
                    (carry, item)
                })
                .collect()
        })
        .collect();
    let mut out = Vec::with_capacity(total);
    // The newest of the services' next ones, each time; of two at once, the
    // service listed first.
    while let Some(next) = queues
        .iter()
        .enumerate()
        .filter_map(|(i, q)| q.front().map(|(at, _)| (i, *at)))
        .max_by(|a, b| a.1.cmp(&b.1).then(b.0.cmp(&a.0)))
        .map(|(i, _)| i)
    {
        if let Some((_, item)) = queues[next].pop_front() {
            out.push(item);
        }
    }
    out
}

fn provider(item: &Collection) -> ProviderKind {
    match item {
        Collection::Album(a) => a.id.provider,
        Collection::Playlist(p) => p.id.provider,
        Collection::Station(s) => s.id.provider,
    }
}

fn added(item: &Collection) -> Option<i64> {
    let text = match item {
        Collection::Album(a) => a.added.as_deref(),
        Collection::Playlist(p) => p.added.as_deref(),
        Collection::Station(_) => None,
    }?;
    seconds(text)
}

/// Seconds since 1970 of a time as the services write it: RFC 3339
/// (`2025-10-31T12:00:00Z`, `…T12:00:00.123+01:00`), or SoundCloud's older
/// `2025/10/31 12:00:00 +0000`.
fn seconds(text: &str) -> Option<i64> {
    let number = |at: usize, len: usize| -> Option<i64> { text.get(at..at + len)?.parse().ok() };
    let (year, month, day) = (number(0, 4)?, number(5, 2)?, number(8, 2)?);
    let (hour, minute, second) = (number(11, 2)?, number(14, 2)?, number(17, 2)?);
    // After the seconds, a fraction, then the offset from UTC.
    let rest = text
        .get(19..)
        .unwrap_or("")
        .trim_start_matches(|c: char| c == '.' || c.is_ascii_digit())
        .trim();
    let offset = match rest {
        "" | "Z" => 0,
        _ => {
            let sign = match rest.as_bytes()[0] {
                b'+' => 1,
                b'-' => -1,
                _ => return None,
            };
            let digits: String = rest[1..].chars().filter(char::is_ascii_digit).collect();
            let hours: i64 = digits.get(0..2)?.parse().ok()?;
            let minutes: i64 = digits.get(2..4).and_then(|m| m.parse().ok()).unwrap_or(0);
            sign * (hours * 3600 + minutes * 60)
        }
    };
    Some(days(year, month, day) * 86_400 + hour * 3600 + minute * 60 + second - offset)
}

/// `secs` since 1970 as RFC 3339, in UTC.
pub fn rfc3339(secs: i64) -> String {
    let (days, rest) = (secs.div_euclid(86_400), secs.rem_euclid(86_400));
    // Howard Hinnant's `civil_from_days`.
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let of_era = z - era * 146_097;
    let year_of_era = (of_era - of_era / 1460 + of_era / 36_524 - of_era / 146_096) / 365;
    let of_year = of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let mp = (5 * of_year + 2) / 153;
    let day = of_year - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = year_of_era + era * 400 + i64::from(month <= 2);
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}Z",
        rest / 3600,
        rest / 60 % 60,
        rest % 60
    )
}

/// Days since 1970-01-01 of a date (Howard Hinnant's `days_from_civil`).
fn days(year: i64, month: i64, day: i64) -> i64 {
    let year = if month <= 2 { year - 1 } else { year };
    let era = year.div_euclid(400);
    let of_era = year - era * 400;
    let of_year = (153 * (if month > 2 { month - 3 } else { month + 9 }) + 2) / 5 + day - 1;
    let of_cycle = of_era * 365 + of_era / 4 - of_era / 100 + of_year;
    era * 146_097 + of_cycle - 719_468
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Album, ItemId, Playlist};

    fn album(provider: ProviderKind, id: &str, added: Option<&str>) -> Collection {
        Collection::Album(Album {
            id: ItemId::new(provider, id),
            title: id.to_owned(),
            artists: Vec::new(),
            artist_id: None,
            artist_artwork: None,
            released: None,
            genre: None,
            description: None,
            catalog_id: None,
            in_favourites: None,
            year: None,
            artwork: None,
            track_count: None,
            explicit: false,
            added: added.map(str::to_owned),
            motion: None,
            copyright: None,
            mastered: false,
        })
    }

    fn playlist(provider: ProviderKind, id: &str, added: &str) -> Collection {
        Collection::Playlist(Playlist {
            id: ItemId::new(provider, id),
            name: id.to_owned(),
            description: None,
            curator: None,
            favourites: false,
            artwork: None,
            track_count: None,
            catalog_id: None,
            in_favourites: None,
            editable: false,
            added: Some(added.to_owned()),
        })
    }

    fn ids(items: &[Collection]) -> Vec<String> {
        items
            .iter()
            .map(|c| match c {
                Collection::Album(a) => a.id.id.clone(),
                Collection::Playlist(p) => p.id.id.clone(),
                Collection::Station(s) => s.id.id.clone(),
            })
            .collect()
    }

    #[test]
    fn the_services_interleave_by_when_each_was_added() {
        use ProviderKind::*;
        let items = vec![
            album(AppleMusic, "apple-today", Some("2026-11-01T10:00:00Z")),
            album(AppleMusic, "apple-monday", Some("2026-10-27T10:00:00Z")),
            album(
                SoundCloud,
                "sc-yesterday",
                Some("2026/10/31 10:00:00 +0000"),
            ),
            playlist(SoundCloud, "sc-last-week", "2026-10-25T10:00:00Z"),
            album(
                Navidrome,
                "nd-wednesday",
                Some("2026-10-29T12:00:00.123+02:00"),
            ),
        ];
        assert_eq!(
            ids(&in_time_order(items)),
            [
                "apple-today",
                "sc-yesterday",
                "nd-wednesday",
                "apple-monday",
                "sc-last-week"
            ]
        );
    }

    #[test]
    fn a_services_own_order_stays_and_an_item_without_a_time_keeps_its_place() {
        use ProviderKind::*;
        let items = vec![
            // Apple's: an old album a song was just added to, first.
            album(AppleMusic, "touched", Some("2026-11-01T09:00:00Z")),
            album(AppleMusic, "undated", None),
            album(AppleMusic, "older", Some("2026-10-20T09:00:00Z")),
            album(SoundCloud, "sc", Some("2026-10-30T09:00:00Z")),
        ];
        assert_eq!(
            ids(&in_time_order(items)),
            ["touched", "undated", "sc", "older"]
        );
    }

    #[test]
    fn times_read_alike_whatever_the_format() {
        assert_eq!(seconds("1970-01-01T00:00:00Z"), Some(0));
        assert_eq!(
            seconds("2026-11-01T01:00:00+01:00"),
            seconds("2026-11-01T00:00:00Z")
        );
        assert_eq!(
            seconds("2026/11/01 00:00:00 +0000"),
            seconds("2026-11-01T00:00:00.5Z")
        );
        assert_eq!(seconds("2000-03-01T00:00:00Z"), Some(951_868_800));
        assert_eq!(seconds("soon"), None);
        for t in [
            "2026-11-01T05:10:48Z",
            "2000-02-29T23:59:59Z",
            "1970-01-01T00:00:00Z",
        ] {
            assert_eq!(rfc3339(seconds(t).unwrap()), t);
        }
    }
}
