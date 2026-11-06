//! Everything that reads what Apple sends, fed thousands of damaged inputs made from seeds by
//! a seeded generator: bytes flipped, cut, repeated, fields of another type, huge numbers,
//! empty and very long strings. Each reader must not panic, must finish (the searches count
//! their steps), and its result must keep its invariants.
//!
//! Here: the generator's rounds, and the readers of the crate's root (the ids a change may
//! carry, the next page's address) and of `station` (its id, its answers). The private readers of `playback` (the
//! ids, the `webPlayback` entry, the media playlists, the licence, and Opium's own beside
//! them) and of `auth` (the page and the script) are fed the same way in a `mutation_tests`
//! module of their own file.

use serde_json::{Value, json};

use crate::test_support::{Rng, mutate_bytes, mutate_json, mutate_text};

/// How many damaged inputs each reader gets.
pub(crate) const ROUNDS: usize = 3000;

/// `seeds` damaged [`ROUNDS`] times over, once or twice each, as text.
pub(crate) fn damaged_texts(seed: u64, seeds: &[String]) -> Vec<String> {
    let mut rng = Rng::new(seed);
    (0..ROUNDS)
        .map(|round| {
            let mut text = mutate_text(&mut rng, &seeds[round % seeds.len()]);
            if round % 3 == 2 {
                text = mutate_text(&mut rng, &text);
            }
            text
        })
        .collect()
}

/// `seeds` damaged [`ROUNDS`] times over: a value changed, or the JSON's text damaged and
/// read again when it still reads.
pub(crate) fn damaged_json(seed: u64, seeds: &[Value]) -> Vec<Value> {
    let mut rng = Rng::new(seed);
    (0..ROUNDS)
        .map(|round| {
            let seed = &seeds[round % seeds.len()];
            let mut value = mutate_json(&mut rng, seed);
            if round % 4 == 3 {
                value = mutate_json(&mut rng, &value);
            }
            if round % 5 == 4
                && let Ok(read) =
                    serde_json::from_slice(&mutate_bytes(&mut rng, seed.to_string().as_bytes()))
            {
                value = read;
            }
            value
        })
        .collect()
}

pub(crate) fn is_digits(s: &str) -> bool {
    !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit())
}

#[test]
fn apple_ids() {
    let seeds = [
        "1234567890",
        "i.AbC9-x",
        "pl.u-1b2",
        "ra.123",
        "l.x",
        "a.Z-9",
        "",
    ]
    .map(String::from);
    for id in damaged_texts(0x1D6, &seeds) {
        if let Ok(apple) = crate::apple_id(&id) {
            // Unescaped in an address, it stays one segment of the API's path.
            let url = url::Url::parse(&format!("{}/v1/me/library/songs/{apple}", crate::API))
                .expect("an address");
            assert_eq!(url.host_str(), Some("amp-api.music.apple.com"));
            assert_eq!(
                url.path_segments().and_then(|mut s| s.next_back()),
                Some(apple)
            );
            assert!(url.query().is_none() && url.fragment().is_none());
        }
    }
}

#[test]
fn station_ids() {
    let seeds = ["ra.cp-pl.u-AbC9", "ra.1234", "ra.u-a_b", "..", ""].map(String::from);
    let mut accepted = 0;
    for id in damaged_texts(0x57A, &seeds) {
        if let Ok(station) = crate::station::station_id(&id) {
            accepted += 1;
            // Unescaped in the address, it stays its last segment, as it is.
            let address = format!("{}/v1/me/stations/next-tracks/{station}", crate::API);
            let url = url::Url::parse(&address).expect("an address");
            assert_eq!(url.host_str(), Some("amp-api.music.apple.com"));
            assert_eq!(url.as_str(), address, "{id:?}");
            assert_eq!(
                url.path_segments().and_then(|mut s| s.next_back()),
                Some(station)
            );
            assert!(url.query().is_none() && url.fragment().is_none());
        }
    }
    assert!(accepted > 0, "some ids are still stations'");
}

#[test]
fn next_page_addresses() {
    let seeds = [
        "/v1/me/library/songs?offset=100",
        "https://amp-api.music.apple.com/v1/me/library/albums?offset=25&l=en",
        "/v1/catalog/zz/playlists/pl.u-x/tracks?offset=300&ids%5Bsongs%5D=1",
        "",
    ]
    .map(String::from);
    let firsts = [
        "/v1/me/library/songs?limit=100&relate=albums",
        "/v1/me/library/albums?limit=100&include=catalog&fields%5Balbums%5D=contentRating",
    ]
    .map(String::from);
    let nexts = damaged_texts(0x9E7, &seeds);
    let firsts = damaged_texts(0xF15, &firsts);
    for (round, (next, first)) in nexts.iter().zip(&firsts).enumerate() {
        for path in [
            crate::next_page_path(next, first),
            crate::at_offset(next, round * 100, first),
        ] {
            assert!(path.starts_with('/'), "{path:?}");
            if let Ok(url) = url::Url::parse(&format!("{}{path}", crate::API)) {
                assert_eq!(url.host_str(), Some("amp-api.music.apple.com"), "{path:?}");
            }
        }
        let _ = crate::offset_in(next);
    }
}

#[test]
fn station_answers() {
    use crate::station::{Gathering, MOST_CALLS, MOST_IN_FLIGHT, songs_in};
    let song = |id: usize| json!({ "id": format!("90000{id}"), "type": "songs", "attributes": { "name": "Made up" } });
    let mut rng = Rng::new(0xC0DE);
    let mut next_id = 0;
    let mut answers = 0;
    let mut whole = 0;
    while answers < ROUNDS {
        let count = rng.below(30);
        let mut gathering = Gathering::new(count);
        let mut turns = 0;
        loop {
            let more = gathering.calls_to_send();
            gathering.sent(more);
            assert!(gathering.calls_in_flight() <= MOST_IN_FLIGHT);
            assert!(gathering.calls_sent() <= MOST_CALLS);
            if gathering.is_over() {
                break;
            }
            // Two new songs, or the same again, damaged.
            let seed = json!({ "data": [song(next_id), song(next_id + rng.below(2))] });
            next_id += rng.below(3);
            let mut answer = mutate_json(&mut rng, &seed).to_string();
            if rng.below(4) == 0 {
                answer = mutate_text(&mut rng, &answer);
            }
            gathering.answered(songs_in("/p", &answer));
            answers += 1;
            turns += 1;
            assert!(turns <= MOST_CALLS, "a gathering ends within its budget");
        }
        if let Ok(songs) = gathering.into_songs() {
            assert!(songs.len() <= count);
            let ids: std::collections::HashSet<&str> =
                songs.iter().map(|s| s.id.id.as_str()).collect();
            assert_eq!(ids.len(), songs.len(), "each song once");
            whole += usize::from(count > 0 && songs.len() == count);
        }
    }
    assert!(whole > 0, "some gatherings get every song asked");
}
