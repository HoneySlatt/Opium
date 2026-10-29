//! Three colours from a cover, for the gradient behind album, playlist and artist
//! pages, as `DESIGN.md` specifies: quantise the cover to 8 colours at 48 × 48,
//! score each by `count × (0.25 + 2.5 × saturation × value)`, keep the three
//! best that differ in hue or brightness, then pull them into the theme's OKLCH
//! band so they sit in any theme.

use egui::Color32;

use crate::color::Oklab;

/// Cover colours, best first. Repeats the best when the cover has fewer.
pub type Palette = [Color32; 3];

/// The palette of an encoded image (JPEG, PNG…).
pub fn from_image(bytes: &[u8]) -> Option<Palette> {
    let image = image::load_from_memory(bytes).ok()?.to_rgb8();
    let small = image::imageops::resize(&image, 48, 48, image::imageops::FilterType::Triangle);
    let pixels: Vec<[u8; 3]> = small.pixels().map(|p| p.0).collect();
    Some(pick(&quantise(pixels, 8)))
}

/// Median cut for a start, then a few k-means passes so each group settles on a
/// colour the cover actually has rather than a blend across a split.
fn quantise(pixels: Vec<[u8; 3]>, n: usize) -> Vec<([u8; 3], usize)> {
    let mut centres: Vec<[f32; 3]> = median_cut(pixels.clone(), n)
        .into_iter()
        .map(|(c, _)| c.map(f32::from))
        .collect();
    let mut counts = vec![0usize; centres.len()];
    for _ in 0..6 {
        let mut sums = vec![[0f32; 3]; centres.len()];
        counts.iter_mut().for_each(|c| *c = 0);
        for p in &pixels {
            let p = p.map(f32::from);
            let nearest = (0..centres.len())
                .min_by(|&a, &b| distance(&p, &centres[a]).total_cmp(&distance(&p, &centres[b])))
                .unwrap_or(0);
            counts[nearest] += 1;
            for c in 0..3 {
                sums[nearest][c] += p[c];
            }
        }
        for (k, centre) in centres.iter_mut().enumerate() {
            if counts[k] > 0 {
                *centre = sums[k].map(|s| s / counts[k] as f32);
            }
        }
    }
    centres
        .into_iter()
        .zip(counts)
        .filter(|&(_, n)| n > 0)
        .map(|(c, n)| (c.map(|v| v.round() as u8), n))
        .collect()
}

fn distance(a: &[f32; 3], b: &[f32; 3]) -> f32 {
    (0..3).map(|c| (a[c] - b[c]).powi(2)).sum()
}

/// Split the pixels into up to `n` boxes, each time cutting the box with the
/// widest channel at its median. Returns each box's mean colour and size.
fn median_cut(pixels: Vec<[u8; 3]>, n: usize) -> Vec<([u8; 3], usize)> {
    let mut boxes = vec![pixels];
    while boxes.len() < n {
        let widest = boxes
            .iter()
            .enumerate()
            .filter(|(_, b)| b.len() > 1)
            .map(|(i, b)| {
                let (channel, range) = widest_channel(b);
                (i, channel, range)
            })
            .max_by_key(|&(_, _, range)| range);
        let Some((i, channel, range)) = widest else {
            break;
        };
        if range == 0 {
            break;
        }
        let mut b = boxes.swap_remove(i);
        b.sort_unstable_by_key(|p| p[channel]);
        let upper = b.split_off(b.len() / 2);
        boxes.push(b);
        boxes.push(upper);
    }
    boxes
        .into_iter()
        .filter(|b| !b.is_empty())
        .map(|b| {
            let mut sum = [0u64; 3];
            for p in &b {
                for c in 0..3 {
                    sum[c] += u64::from(p[c]);
                }
            }
            let n = b.len() as u64;
            (
                [(sum[0] / n) as u8, (sum[1] / n) as u8, (sum[2] / n) as u8],
                b.len(),
            )
        })
        .collect()
}

fn widest_channel(pixels: &[[u8; 3]]) -> (usize, u8) {
    (0..3)
        .map(|c| {
            let (lo, hi) = pixels
                .iter()
                .fold((u8::MAX, 0), |(lo, hi), p| (lo.min(p[c]), hi.max(p[c])));
            (c, hi.saturating_sub(lo))
        })
        .max_by_key(|&(_, range)| range)
        .unwrap_or((0, 0))
}

/// HSV hue (degrees), saturation and value, all from 0.
fn hsv([r, g, b]: [u8; 3]) -> (f32, f32, f32) {
    let (r, g, b) = (
        f32::from(r) / 255.0,
        f32::from(g) / 255.0,
        f32::from(b) / 255.0,
    );
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let delta = max - min;
    let hue = if delta == 0.0 {
        0.0
    } else if max == r {
        60.0 * ((g - b) / delta).rem_euclid(6.0)
    } else if max == g {
        60.0 * ((b - r) / delta + 2.0)
    } else {
        60.0 * ((r - g) / delta + 4.0)
    };
    let saturation = if max == 0.0 { 0.0 } else { delta / max };
    (hue, saturation, max)
}

/// Whether two colours read as different: another hue (when both carry colour),
/// another brightness, or colour against grey. Colourfulness is saturation
/// times value, so near-blacks, whatever their tint, count as grey.
fn distinct(a: [u8; 3], b: [u8; 3]) -> bool {
    let (ha, sa, va) = hsv(a);
    let (hb, sb, vb) = hsv(b);
    let (ca, cb) = (sa * va, sb * vb);
    let hue_gap = (ha - hb).abs().min(360.0 - (ha - hb).abs());
    (ca > 0.15 && cb > 0.15 && hue_gap > 25.0) || (va - vb).abs() > 0.15 || (ca - cb).abs() > 0.25
}

fn pick(colours: &[([u8; 3], usize)]) -> Palette {
    let mut scored: Vec<([u8; 3], f32)> = colours
        .iter()
        .map(|&(c, count)| {
            let (_, s, v) = hsv(c);
            (c, count as f32 * (0.25 + 2.5 * s * v))
        })
        .collect();
    scored.sort_by(|a, b| b.1.total_cmp(&a.1));
    let mut chosen: Vec<[u8; 3]> = Vec::with_capacity(3);
    for (c, _) in &scored {
        if chosen.len() < 3 && chosen.iter().all(|&k| distinct(k, *c)) {
            chosen.push(*c);
        }
    }
    let first = chosen.first().copied().unwrap_or([0, 0, 0]);
    let at = |i: usize| {
        let [r, g, b] = chosen.get(i).copied().unwrap_or(first);
        Color32::from_rgb(r, g, b)
    };
    [at(0), at(1), at(2)]
}

/// Pull a colour into the theme's band: lightness 0.34–0.56 and chroma at most
/// 0.20 on dark themes, 0.80–0.90 and 0.12 on light ones. A black cover still
/// gives a visible wash, a vivid one never glares on a pastel theme.
pub fn fit(c: Color32, light: bool) -> Color32 {
    let (l_min, l_max, c_max) = if light {
        (0.80, 0.90, 0.12)
    } else {
        (0.34, 0.56, 0.20)
    };
    let lab = Oklab::from_color(c);
    let chroma = lab.a.hypot(lab.b);
    let scale = if chroma > c_max { c_max / chroma } else { 1.0 };
    Oklab {
        l: lab.l.clamp(l_min, l_max),
        a: lab.a * scale,
        b: lab.b * scale,
    }
    .to_color()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn block(colour: [u8; 3], n: usize) -> Vec<[u8; 3]> {
        vec![colour; n]
    }

    #[test]
    fn three_distinct_colours_come_out_dominant_first() {
        let mut pixels = block([200, 30, 30], 1200);
        pixels.extend(block([30, 120, 200], 700));
        pixels.extend(block([220, 180, 60], 404));
        let palette = pick(&quantise(pixels, 8));
        assert_eq!(palette[0], Color32::from_rgb(200, 30, 30));
        assert!(palette.contains(&Color32::from_rgb(30, 120, 200)));
        assert!(palette.contains(&Color32::from_rgb(220, 180, 60)));
    }

    #[test]
    fn a_flat_cover_repeats_its_colour() {
        let palette = pick(&quantise(block([147, 213, 1], 2304), 8));
        assert_eq!(palette, [Color32::from_rgb(147, 213, 1); 3]);
    }

    #[test]
    fn saturation_outweighs_area() {
        // A large dull grey and a smaller vivid red: the red leads.
        let mut pixels = block([90, 90, 90], 1600);
        pixels.extend(block([220, 20, 30], 704));
        let palette = pick(&quantise(pixels, 8));
        assert_eq!(palette[0], Color32::from_rgb(220, 20, 30));
    }

    #[test]
    fn colours_are_pulled_into_the_dark_band() {
        let black = Oklab::from_color(fit(Color32::BLACK, false));
        assert!(
            (black.l - 0.34).abs() < 0.01,
            "black lifts to the floor: {}",
            black.l
        );
        let white = Oklab::from_color(fit(Color32::WHITE, false));
        assert!(
            (white.l - 0.56).abs() < 0.01,
            "white drops to the ceiling: {}",
            white.l
        );
        let vivid = Oklab::from_color(fit(Color32::from_rgb(0, 255, 0), false));
        assert!(vivid.a.hypot(vivid.b) <= 0.201);
    }

    #[test]
    fn light_themes_get_pastels() {
        let c = Oklab::from_color(fit(Color32::from_rgb(160, 34, 31), true));
        assert!(c.l >= 0.79 && c.a.hypot(c.b) <= 0.121);
    }
}
