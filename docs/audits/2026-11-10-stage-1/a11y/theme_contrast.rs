// Contrast of the built-in themes (stage audit M1.8, accessibility): proofs
// for findings F2 to F6 of the m18-a11y report. Two parts, for two files.
// Not compiled by the reviewer, who may not edit tracked files; every helper
// used exists today (named at each use). Each test fails today, with the
// ratio contrast.py computes (.delegation/m18-a11y/04.out) in its message.
//
// WCAG 2.2's contrast: (L1 + 0.05) / (L2 + 0.05) on relative luminance;
// 4.5:1 for text, 3:1 for large text (24 px, or 18.66 px bold) and controls.

// ---------------------------------------------------------------------------
// Part 1: append inside `mod tests` of crates/app-desktop/src/theme.rs (it
// already has `use super::*;`, which brings `Theme`, `Color32` and
// `BUILT_IN`, imported at theme.rs:11 and :140).
// Run: `cargo test -p app-desktop theme::tests`.

fn contrast(a: Color32, b: Color32) -> f32 {
    // crate::color::luminance is WCAG's relative luminance (color.rs:33-37).
    let (x, y) = (crate::color::luminance(a), crate::color::luminance(b));
    (x.max(y) + 0.05) / (x.min(y) + 0.05)
}

/// `top`, translucent, over an opaque `under`, blended in gamma space as
/// egui blends; Color32 is premultiplied.
fn over(top: Color32, under: Color32) -> Color32 {
    let a = f32::from(top.a()) / 255.0;
    let blend = |t: u8, u: u8| (f32::from(t) + f32::from(u) * (1.0 - a)).round() as u8;
    Color32::from_rgb(
        blend(top.r(), under.r()),
        blend(top.g(), under.g()),
        blend(top.b(), under.b()),
    )
}

#[test]
fn text_on_the_accent_reaches_wcag_aa_in_every_built_in_theme() {
    // Pill labels (ui/devices.rs:224-237, ui/playlist_dialog.rs:938-949) and
    // a hovered menu item (ui/menu.rs:150-154) are text on the accent.
    // Today: Stock 3.79:1, white on #fa2d48 (on_accent's 0.25 threshold,
    // theme.rs:341-347, picks white up to luminance 0.25; black and white
    // give equal contrast at 0.179).
    for (id, _) in BUILT_IN {
        let theme = Theme::named(id).unwrap();
        let ratio = contrast(theme.on_accent(), theme.accent);
        assert!(ratio >= 4.5, "{id}: on_accent() on the accent is {ratio:.2}:1");
    }
}

#[test]
fn tertiary_text_reaches_wcag_aa_in_the_popovers() {
    // The signal path's and the devices' popovers and the dialogs fill with
    // mantle (ui/signal.rs:366-372, ui/devices.rs:58-59,
    // ui/playlist_dialog.rs:41) and write in muted() (ui/signal.rs:298,
    // ui/devices.rs:142-160,211, ui/playlist_dialog.rs:788,912).
    // Today: Spotify 4.33:1, #8b8b8b on #282828.
    for (id, _) in BUILT_IN {
        let theme = Theme::named(id).unwrap();
        let ratio = contrast(theme.muted(), theme.mantle);
        assert!(ratio >= 4.5, "{id}: muted() on mantle is {ratio:.2}:1");
    }
}

#[test]
fn the_explicit_badge_reaches_wcag_aa() {
    // ui/mod.rs:1275-1283: the "E", 8.5 px bold, in base on 55 % subtext0.
    // Today: Stock 3.29, Spotify 3.17, M3E 3.66.
    for (id, _) in BUILT_IN {
        let theme = Theme::named(id).unwrap();
        let fill = crate::color::mix(theme.subtext0, theme.base, 0.55);
        let ratio = contrast(theme.base, fill);
        assert!(ratio >= 4.5, "{id}: the badge's E is {ratio:.2}:1");
    }
}

#[test]
fn unlit_lyrics_reach_wcag_aa_for_large_text() {
    // ui/lyrics.rs:234,253: a synced line not being sung is text at 30 %
    // (its type is 21 to 42 px bold: large). On the page (base: the side
    // panel and M3E's pane, ui/mod.rs:197,224,254). Today: Stock 2.55,
    // Spotify 2.67, M3E 2.35.
    for (id, _) in BUILT_IN {
        let theme = Theme::named(id).unwrap();
        let shown = over(theme.text_a(0.3), theme.base);
        let ratio = contrast(shown, theme.base);
        assert!(ratio >= 3.0, "{id}: an unlit line is {ratio:.2}:1");
    }
}

// ---------------------------------------------------------------------------
// Part 2: append inside `mod tests` of crates/opium-core/src/theme.rs (it has
// `use super::*;`, which brings `built_in`). The phone draws these roles
// (android/.../ui/theme/Theme.kt:57-87).
// Run: `cargo test -p opium-core theme::tests`.

fn contrast_rgb(a: u32, b: u32) -> f32 {
    let luminance = |v: u32| {
        // crate::color::to_linear is pub (opium-core/src/color.rs:4).
        let [r, g, b] = [(v >> 16) as u8, (v >> 8) as u8, v as u8].map(crate::color::to_linear);
        0.2126 * r + 0.7152 * g + 0.0722 * b
    };
    let (x, y) = (luminance(a), luminance(b));
    (x.max(y) + 0.05) / (x.min(y) + 0.05)
}

#[test]
fn stocks_light_colours_reach_wcag_aa_as_text_on_the_phones_page() {
    // On the phone in light appearance: tertiary text in `muted`
    // (e.g. ui/components/Sheet.kt:200, ui/connect/Connect.kt:431), errors and
    // "Remove", "Delete Playlist" in `red` (ui/actions/Sheets.kt:321,451,
    // ui/settings/SettingsSheet.kt:461), "Choose Cover" in `accent`
    // (ui/actions/Sheets.kt:317,421). Today: muted 3.44, red 3.55, accent 3.79.
    let light = built_in("stock").unwrap().light_colors.unwrap();
    for (role, color) in [
        ("muted", light.muted.unwrap()),
        ("red", light.red),
        ("accent", light.accent),
    ] {
        let ratio = contrast_rgb(color, light.base);
        assert!(ratio >= 4.5, "Stock's light {role} on base is {ratio:.2}:1");
    }
}

#[test]
fn m3es_tint_reaches_wcag_aa_as_text_on_its_page() {
    // The phone's `tint` is the accent when the theme gives none (Theme.kt:75),
    // and writes "Clear" (ui/browse/SearchPage.kt:599), "New Playlist…"
    // (ui/actions/Sheets.kt:103), Play and Shuffle
    // (ui/library/LibraryScreens.kt:653). Today: 3.85 on base.
    let m3e = built_in("m3e").unwrap().colors;
    let tint = m3e.tint.unwrap_or(m3e.accent);
    let ratio = contrast_rgb(tint, m3e.base);
    assert!(ratio >= 4.5, "M3E's tint on base is {ratio:.2}:1");
}
