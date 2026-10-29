//! Animated cover art: an HLS master playlist whose variants are each one MP4
//! file, cut into byte ranges. The app plays one variant's file in a loop, so
//! it is fetched whole.

use music_core::{Error, Result};

use crate::AppleMusic;
use crate::radio::{attribute, resolve};

/// The variant to fetch: H.264, the narrowest at least `size` pixels wide
/// (the best bitrate at that width), else the widest there is.
pub(crate) fn motion_variant(master: &str, size: u32) -> Option<String> {
    let mut variants: Vec<(u32, u64, String)> = Vec::new();
    let mut lines = master.lines();
    while let Some(line) = lines.next() {
        if !line.starts_with("#EXT-X-STREAM-INF:") {
            continue;
        }
        let h264 = attribute(line, "CODECS").is_some_and(|c| c.starts_with("avc1"));
        let width = attribute(line, "RESOLUTION")
            .and_then(|r| r.split_once('x'))
            .and_then(|(w, _)| w.parse().ok())
            .unwrap_or(0);
        let bandwidth = attribute(line, "BANDWIDTH")
            .and_then(|b| b.parse().ok())
            .unwrap_or(0);
        if let Some(uri) = lines.next()
            && h264
        {
            variants.push((width, bandwidth, uri.trim().to_owned()));
        }
    }
    let wide_enough = variants
        .iter()
        .filter(|v| v.0 >= size)
        .min_by_key(|v| (v.0, std::cmp::Reverse(v.1)));
    wide_enough
        .or_else(|| variants.iter().max_by_key(|v| (v.0, v.1)))
        .map(|v| v.2.clone())
}

impl AppleMusic {
    /// The MP4 of an animated cover, from its master playlist at `master`,
    /// the variant suited to `size` pixels a side.
    pub async fn motion_video(&self, master: &str, size: u32) -> Result<Vec<u8>> {
        let text = self.public_text(master).await?;
        let variant = motion_variant(&text, size)
            .ok_or_else(|| Error::Protocol("animated cover: no H.264 variant".into()))?;
        let variant = resolve(master, &variant);
        let playlist = self.public_text(&variant).await?;
        // Every segment is a range of the file the init segment names.
        let file = playlist
            .lines()
            .find(|l| l.starts_with("#EXT-X-MAP:"))
            .and_then(|l| attribute(l, "URI"))
            .ok_or_else(|| Error::Protocol("animated cover: no file".into()))?;
        let url = resolve(&variant, file);
        let resp = self
            .http
            .get(&url)
            .send()
            .await
            .map_err(|e| Error::Network(format!("animated cover: {e}")))?;
        if !resp.status().is_success() {
            return Err(Error::Protocol(format!(
                "animated cover: HTTP {}",
                resp.status()
            )));
        }
        resp.bytes()
            .await
            .map(|b| b.to_vec())
            .map_err(|e| Error::Network(format!("animated cover: {e}")))
    }

    /// A text file from Apple's public CDN, which wants no tokens.
    async fn public_text(&self, url: &str) -> Result<String> {
        let resp = self
            .http
            .get(url)
            .send()
            .await
            .map_err(|e| Error::Network(format!("animated cover: {e}")))?;
        if !resp.status().is_success() {
            return Err(Error::Protocol(format!(
                "animated cover: HTTP {}",
                resp.status()
            )));
        }
        resp.text()
            .await
            .map_err(|e| Error::Network(format!("animated cover: {e}")))
    }
}

#[cfg(test)]
mod tests {
    use super::motion_variant;

    const MASTER: &str = "#EXTM3U
#EXT-X-STREAM-INF:BANDWIDTH=1987477,CODECS=\"avc1.64001f\",RESOLUTION=768x768
a768.m3u8
#EXT-X-STREAM-INF:BANDWIDTH=1455343,CODECS=\"hvc1.2.20000000.L123.B0\",RESOLUTION=768x768
h768.m3u8
#EXT-X-STREAM-INF:BANDWIDTH=273236,CODECS=\"avc1.64001f\",RESOLUTION=360x360
a360.m3u8
#EXT-X-STREAM-INF:BANDWIDTH=818609,CODECS=\"avc1.64001f\",RESOLUTION=486x486
a486lo.m3u8
#EXT-X-STREAM-INF:BANDWIDTH=1259458,CODECS=\"avc1.64001f\",RESOLUTION=486x486
a486hi.m3u8
#EXT-X-STREAM-INF:BANDWIDTH=4676368,CODECS=\"avc1.640020\",RESOLUTION=1080x1080
a1080.m3u8
";

    #[test]
    fn the_narrowest_h264_variant_wide_enough_is_chosen_at_its_best_bitrate() {
        assert_eq!(motion_variant(MASTER, 400).as_deref(), Some("a486hi.m3u8"));
        assert_eq!(motion_variant(MASTER, 700).as_deref(), Some("a768.m3u8"));
        assert_eq!(motion_variant(MASTER, 100).as_deref(), Some("a360.m3u8"));
    }

    #[test]
    fn past_the_widest_the_widest_is_chosen() {
        assert_eq!(motion_variant(MASTER, 2000).as_deref(), Some("a1080.m3u8"));
    }
}
