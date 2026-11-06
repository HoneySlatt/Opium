//! Opium's own readers beside Apple's: the enhanced playlist, its master's variant, the
//! playlists given to ExoPlayer and to players of Widevine CENC, the map's range and the AAC
//! configuration's rate. Whatever the input, no panic, and each keeps its invariants.

#![no_main]

use libfuzzer_sys::fuzz_target;

const URL: &str = "https://aod.example/a/p.m3u8";
const WIDEVINE: &str = "edef8ba9-79d6-4ace-a3c8-27dcd51d21ed";

fuzz_target!(|data: &[u8]| {
    if let Some(rate) = apple_music::fuzzing::aac_rate(data) {
        assert!((7_350..=96_000).contains(&rate), "{rate}");
    }
    let body = String::from_utf8_lossy(data);
    if let Some((_, mut uris)) = apple_music::fuzzing::enhanced_file(URL, &body) {
        assert!(!uris.is_empty());
        let count = uris.len();
        uris.sort();
        uris.dedup();
        assert_eq!(uris.len(), count, "each key once");
    }
    let exo = apple_music::fuzzing::exoplayer_playlist(&body, URL);
    assert!(
        exo.lines()
            .filter(|line| line.starts_with("#EXT-X-KEY"))
            .all(|line| line.contains(WIDEVINE)),
        "Widevine's keys only"
    );
    let _ = apple_music::fuzzing::widevine_playlist(&body, URL, data);
    if apple_music::fuzzing::map_range(&body).is_some() {
        assert!(body.contains("#EXT-X-MAP"));
    }
    if apple_music::fuzzing::variant(URL, &body, "audio-stereo-256-binaural").is_some() {
        assert!(body.contains("#EXT-X-STREAM-INF"));
    }
});
