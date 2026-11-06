//! A `webPlayback` entry, as JSON: no panic; its `songId` is taken only when it is a catalog
//! id; an upload is its one asset without a flavour; an encode is the first asset of the
//! highest bit rate that Widevine can open, its bit rate read from its flavour; no choice only
//! when no asset is a candidate.

#![no_main]

use apple_music::fuzzing::{Asset, Chosen};
use libfuzzer_sys::fuzz_target;

/// The bit rate `asset` offers Widevine: the number after its flavour's `:ctrp`, when its
/// `URL` is not empty.
fn kbps_of(asset: &Asset) -> Option<u32> {
    let (_, rest) = asset.flavor.as_deref()?.split_once(":ctrp")?;
    if rest.is_empty() || !rest.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let url = asset.url.as_deref().filter(|url| !url.is_empty());
    url.and(rest.parse().ok())
}

fuzz_target!(|data: &[u8]| {
    if let Some(id) = apple_music::fuzzing::song_id(data) {
        assert_eq!(apple_music::fuzzing::id_kind(&id), Some("catalog"), "{id:?}");
    }
    let Some(entry) = apple_music::fuzzing::web_playback_entry(data) else {
        return;
    };
    let assets = entry.assets.as_deref().unwrap_or_default();
    match entry.chosen {
        Ok(Chosen::Upload(url)) => {
            assert_eq!(assets.len(), 1);
            assert!(assets[0].no_flavor);
            assert!(!url.is_empty());
            assert_eq!(assets[0].url.as_deref(), Some(url.as_str()));
        }
        Ok(Chosen::Encode { playlist, kbps }) => {
            let first_best = assets
                .iter()
                .position(|asset| kbps_of(asset) == Some(kbps))
                .expect("the chosen asset's flavour names its bit rate");
            assert_eq!(assets[first_best].url.as_deref(), Some(playlist.as_str()));
            assert!(assets.iter().filter_map(kbps_of).all(|other| other <= kbps));
        }
        Err(_) => assert!(assets.iter().all(|asset| kbps_of(asset).is_none())),
    }
});
