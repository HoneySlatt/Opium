"""Contrast ratios of the built-in themes, as both apps draw them.

Rebuilds each built-in theme the way `opium_core::theme::parse` does: the
file's `[colors]` laid over `Palette::STOCK` and `[colors.light]` over
`Palette::STOCK_LIGHT`, both read from crates/opium-core/src/theme.rs. Then
derives what each app adds:

- desktop (crates/app-desktop/src/theme.rs): `muted()` = OKLab mix of 78 %
  subtext0 and 22 % base (theme.rs:334-336; `mix(a, b, p)` weights a by p,
  CONTRIBUTING's pitfall); `on_accent()` = black when the accent's WCAG
  luminance is over 0.25, else white (theme.rs:341-347); `text_a(p)` = text
  at opacity p (theme.rs:350); the tonal roles from the file, else
  `Tonal::from_accent` (opium-core/src/theme.rs:86-101). The desktop never
  reads `[colors.light]`.
- phone (android/.../ui/theme/Theme.kt:57-87 `rolesOf`): grouped = base,
  cell = mantle, capsule = surface0, muted = lerp(sub, base, 0.22) in Oklab,
  tint = accent, when the file leaves them out; surface/surface2/surface3 =
  surface0/1/2; onAccent as the desktop's (Theme.kt:52). Sheets: #1C1C1C
  dark, grouped light, M3E's containerLow = lerp(base, cell, 0.5)
  (ui/components/Sheet.kt:60-72, ui/m3e/Menus.kt:48).

Translucent colours are composited over their background in gamma-encoded
sRGB, as egui and Android's Skia blend; egui's alpha is a byte
(`color::alpha` rounds p x 255). Contrast is WCAG 2.2's
(L1 + 0.05) / (L2 + 0.05) on relative luminance.

Part 1 prints every text role on every background role of each theme.
Part 2 prints the pairs the code draws, each with where it is drawn, and
flags those under the threshold of its kind: 4.5 for text, 3 for large text
(at least 24 px, or 18.66 px bold; WCAG's bold is 700, so SemiBold 600 is not)
and for the parts of controls (WCAG 1.4.11).
Part 3 sweeps every colour a cover can be fitted to (opium-core/src/palette.rs
`fit_in`: OKLab lightness clamped to the band, chroma scaled down to its
limit) and gives the worst and best ratio of the text drawn on it.

Usage: python3 -I contrast.py <repository root>
"""

import math
import pathlib
import re
import sys
import tomllib

# ---- colour maths, as crates/opium-core/src/color.rs ----


def to_linear(c: int) -> float:
    c = c / 255.0
    return c / 12.92 if c <= 0.04045 else ((c + 0.055) / 1.055) ** 2.4


def to_srgb(c: float) -> int:
    c = min(max(c, 0.0), 1.0)
    c = c * 12.92 if c <= 0.0031308 else 1.055 * c ** (1 / 2.4) - 0.055
    return int(math.floor(c * 255.0 + 0.5))


def rgb_of(v: int):
    return ((v >> 16) & 255, (v >> 8) & 255, v & 255)


def hexs(rgb) -> str:
    return "#%02x%02x%02x" % tuple(rgb)


def oklab(rgb):
    r, g, b = (to_linear(x) for x in rgb)
    l = (0.41222146 * r + 0.53633255 * g + 0.051445995 * b) ** (1 / 3)
    m = (0.2119035 * r + 0.6806995 * g + 0.10739696 * b) ** (1 / 3)
    s = (0.08830246 * r + 0.28171885 * g + 0.6299787 * b) ** (1 / 3)
    return (
        0.21045426 * l + 0.7936178 * m - 0.004072047 * s,
        1.9779985 * l - 2.4285922 * m + 0.4505937 * s,
        0.025904037 * l + 0.78277177 * m - 0.80867577 * s,
    )


def cube(x: float) -> float:
    return x * x * x


def from_oklab(o):
    L, a, b = o
    l = cube(L + 0.39633778 * a + 0.21580376 * b)
    m = cube(L - 0.105561346 * a - 0.06385417 * b)
    s = cube(L - 0.08948418 * a - 1.2914855 * b)
    return (
        to_srgb(4.0767417 * l - 3.3077116 * m + 0.23096994 * s),
        to_srgb(-1.268438 * l + 2.6097574 * m - 0.34131938 * s),
        to_srgb(-0.0041960863 * l - 0.7034186 * m + 1.7076147 * s),
    )


def oklch(L, c, hue):
    sin, cos = math.sin(math.radians(hue)), math.cos(math.radians(hue))
    chroma = c
    while True:
        o = (L, chroma * cos, chroma * sin)
        rgb = from_oklab(o)
        back = oklab(rgb)
        fits = abs(back[0] - L) < 0.01 and math.hypot(back[1] - o[1], back[2] - o[2]) < 0.01
        if fits or chroma < 0.005:
            return rgb
        chroma -= 0.005


def chroma_hue(rgb):
    o = oklab(rgb)
    return math.hypot(o[1], o[2]), math.degrees(math.atan2(o[2], o[1]))


def tonal_from_accent(accent):
    c, h = chroma_hue(accent)
    return {
        "primary": oklch(0.82, min(c, 0.12), h),
        "on_primary": oklch(0.25, min(c, 0.12), h),
        "secondary_container": oklch(0.36, 0.06, h),
        "on_secondary_container": oklch(0.92, 0.04, h),
        "tertiary_container": oklch(0.38, 0.07, h + 60.0),
        "on_tertiary_container": oklch(0.92, 0.04, h + 60.0),
    }


def mix(a, b, p):
    """`color::mix(a, b, p)`: p of a, the rest of b, in OKLab."""
    x, y = oklab(a), oklab(b)
    return from_oklab(tuple(x[i] * p + y[i] * (1 - p) for i in range(3)))


def over(fg, a, bg):
    """fg at opacity a over an opaque bg, in gamma-encoded sRGB, the opacity
    quantised to a byte first as `color::alpha` does."""
    a = round(min(max(a, 0.0), 1.0) * 255) / 255
    return tuple(int(math.floor(f * a + b * (1 - a) + 0.5)) for f, b in zip(fg, bg))


def lerp_rgb(a, b, t):
    """A gradient's colour t of the way from a to b, interpolated in gamma
    space as the GPU interpolates egui's vertex colours."""
    return tuple(int(math.floor(x + (y - x) * t + 0.5)) for x, y in zip(a, b))


def luminance(rgb):
    r, g, b = (to_linear(x) for x in rgb)
    return 0.2126 * r + 0.7152 * g + 0.0722 * b


def ratio(fg, bg):
    l1, l2 = sorted((luminance(fg), luminance(bg)), reverse=True)
    return (l1 + 0.05) / (l2 + 0.05)


BLACK, WHITE = (0, 0, 0), (255, 255, 255)

# ---- the themes, as the core reads them ----

ROLES = [
    "base", "mantle", "crust", "surface0", "surface1", "surface2", "overlay0",
    "overlay1", "text", "subtext0", "accent", "mauve", "red", "green", "yellow",
]


def core_constant(src: str, name: str) -> dict:
    body = re.search(r"pub const %s: Palette = Palette \{(.*?)\n    \};" % name, src, re.S).group(1)
    palette = {k: int(v, 16) for k, v in re.findall(r"(\w+): 0x([0-9a-fA-F]{6})", body)}
    palette["light"] = "light: true" in body
    return palette


def lay_over(base: dict, table: dict) -> dict:
    p = dict(base)
    for key, value in table.items():
        if key == "light":
            if isinstance(value, bool):
                p["light"] = value
            continue
        p[key] = int(value.strip().lstrip("#"), 16)
    return p


def themes(root: pathlib.Path):
    src = (root / "crates/opium-core/src/theme.rs").read_text()
    stock, stock_light = core_constant(src, "STOCK"), core_constant(src, "STOCK_LIGHT")
    for theme_id in ["stock", "spotify", "m3e"]:
        file = tomllib.loads((root / f"crates/opium-core/assets/themes/{theme_id}.toml").read_text())
        colors = dict(file.get("colors", {}))
        light = colors.pop("light", None)
        yield f"{file['name']} dark", lay_over(stock, colors)
        if isinstance(light, dict):
            yield f"{file['name']} light", lay_over(stock_light, light)


def desktop_roles(p: dict) -> dict:
    r = {k: rgb_of(p[k]) for k in ROLES}
    derived = tonal_from_accent(r["accent"])
    for k in derived:
        r[k] = rgb_of(p[k]) if k in p else derived[k]
    r["muted()"] = mix(r["subtext0"], r["base"], 0.78)
    r["on_accent()"] = BLACK if luminance(r["accent"]) > 0.25 else WHITE
    return r


def phone_roles(p: dict, name: str) -> dict:
    d = desktop_roles(p)
    r = {
        "base": d["base"], "grouped": rgb_of(p["grouped"]) if "grouped" in p else d["base"],
        "cell": rgb_of(p["cell"]) if "cell" in p else d["mantle"],
        "surface": d["surface0"], "surface2": d["surface1"], "surface3": d["surface2"],
        "capsule": rgb_of(p["capsule"]) if "capsule" in p else d["surface0"],
        "text": d["text"], "sub": d["subtext0"],
        # Compose's lerp(start, stop, f) in Oklab: 1 - f of start.
        "muted": rgb_of(p["muted"]) if "muted" in p else mix(d["subtext0"], d["base"], 0.78),
        "accent": d["accent"], "tint": rgb_of(p["tint"]) if "tint" in p else d["accent"],
        "mauve": d["mauve"], "red": d["red"], "green": d["green"], "yellow": d["yellow"],
        "onAccent": d["on_accent()"],
    }
    for k in ["primary", "on_primary", "secondary_container", "on_secondary_container",
              "tertiary_container", "on_tertiary_container"]:
        r[k] = d[k]
    dark = not p.get("light", False)
    m3e = name.startswith("M3E")
    r["sheet"] = mix(r["cell"], r["base"], 0.5) if m3e else ((0x1C, 0x1C, 0x1C) if dark else r["grouped"])
    r["sheetCell"] = r["surface"] if m3e else ((0x26, 0x26, 0x26) if dark else r["cell"])
    return r


def check_port(root: pathlib.Path) -> None:
    """The port must give what the code holds: Theme::STOCK's tonal roles
    (app-desktop/src/theme.rs:173-180), derived from the pink."""
    src = (root / "crates/app-desktop/src/theme.rs").read_text()
    block = re.search(r"tonal: Tonal \{(.*?)\},", src, re.S).group(1)
    held = {k: rgb_of(int(v, 16)) for k, v in re.findall(r"(\w+): rgb\(0x([0-9a-f]{6})\)", block)}
    got = tonal_from_accent(rgb_of(0xFA2D48))
    for k, v in held.items():
        mark = "same" if got[k] == v else "DIFFERS"
        print(f"  {k}: code {hexs(v)}, script {hexs(got[k])} {mark}")
        assert all(abs(a - b) <= 1 for a, b in zip(got[k], v)), k


# ---- part 2: the pairs the code draws, each checked in the code ----

D, P = "desktop", "phone"
STOCK, SPOTIFY, M3E, STOCK_LIGHT = "Stock dark", "Spotify dark", "M3E dark", "Stock light"
DARK3 = [STOCK, SPOTIFY, M3E]


def drawn():
    t = []

    def add(app, where, what, kind, fg, bg, only):
        t.append((app, where, what, kind, fg, bg, only))

    # ---- desktop ----
    add(D, "signal.rs:293-299,366-372; devices.rs:58-59,140-160,211-212; playlist_dialog.rs:41,788,912",
        "muted() 11.5-12 px in the popovers and dialogs, on mantle", "text",
        lambda r: r["muted()"], lambda r: r["mantle"], DARK3)
    add(D, "theme.rs:334; e.g. tracks.rs:135, grid.rs:319, settings.rs:421-422,447-448",
        "muted() 12-13 px on the page", "text",
        lambda r: r["muted()"], lambda r: r["base"], DARK3)
    add(D, "devices.rs:224-237", "pill label 12.5 Medium, on_accent() on accent", "text",
        lambda r: r["on_accent()"], lambda r: r["accent"], DARK3)
    add(D, "playlist_dialog.rs:938-949", "primary pill 14 Medium, on_accent() on accent", "text",
        lambda r: r["on_accent()"], lambda r: r["accent"], DARK3)
    add(D, "playlist_dialog.rs:939-943", "primary pill hovered, on_accent() on mix(accent, text, 0.85)",
        "text", lambda r: r["on_accent()"], lambda r: mix(r["accent"], r["text"], 0.85), DARK3)
    add(D, "menu.rs:146-160 (not M3E: menu.rs:147)", "menu item hovered 13 px, on_accent() on accent",
        "text", lambda r: r["on_accent()"], lambda r: r["accent"], [STOCK, SPOTIFY])
    add(D, "sidebar.rs:387,421-454 (Stock's sidebar, no picture)", "avatar initial 13 SemiBold on the gradient's mauve end",
        "text", lambda r: r["on_accent()"], lambda r: r["mauve"], [STOCK])
    add(D, "sidebar.rs:421-454", "avatar initial 13 SemiBold on the gradient's accent end",
        "text", lambda r: r["on_accent()"], lambda r: r["accent"], [STOCK])
    add(D, "window_bar.rs:109 (Spotify's bar, no picture)", "avatar initial 13 SemiBold on mauve",
        "text", lambda r: r["on_accent()"], lambda r: r["mauve"], [SPOTIFY])
    add(D, "dock.rs:41-56 (Spotify's dock)", "'Playing on' 13 Bold, on_accent() on accent", "text",
        lambda r: r["on_accent()"], lambda r: r["accent"], [SPOTIFY])
    add(D, "fab.rs:97, detail.rs:673, artist.rs:1092, mod.rs:707", "Play glyph, on_accent() on accent", "control",
        lambda r: r["on_accent()"], lambda r: r["accent"], DARK3)
    add(D, "mod.rs:1275-1283", "explicit 'E' 8.5 Bold in base on mix(subtext0, base, 0.55)", "text",
        lambda r: r["base"], lambda r: mix(r["subtext0"], r["base"], 0.55), DARK3)
    add(D, "lyrics.rs:234,253 (unlit synced line, 0.3)", "lyrics 21-42 px Bold, text_a(0.3) on mantle (Stock's drawer, drawer.rs:68-74)",
        "large", lambda r: over(r["text"], 0.3, r["mantle"]), lambda r: r["mantle"], [STOCK])
    add(D, "lyrics.rs:234,253 (unlit synced line, 0.3)", "lyrics 24-34 px Bold, text_a(0.3) on base (panes and cards, mod.rs:197,224,254)",
        "large", lambda r: over(r["text"], 0.3, r["base"]), lambda r: r["base"], DARK3)
    add(D, "lyrics.rs:336,384-390 (the line sung, not yet sung: 0.35)", "lyrics Bold, text_a(0.35) on mantle (Stock's drawer)",
        "large", lambda r: over(r["text"], 0.35, r["mantle"]), lambda r: r["mantle"], [STOCK])
    add(D, "lyrics.rs:336,384-390", "lyrics Bold, text_a(0.35) on base", "large",
        lambda r: over(r["text"], 0.35, r["base"]), lambda r: r["base"], DARK3)
    add(D, "lyrics.rs:310-331", "time chip 12.5 SemiBold text_a(0.6) on crust@0.5 over hover() over mantle",
        "text", lambda r: over(r["text"], 0.6, over(r["crust"], 0.5, over(r["text"], 0.07, r["mantle"]))),
        lambda r: over(r["crust"], 0.5, over(r["text"], 0.07, r["mantle"])), [STOCK])
    add(D, "lyrics.rs:310-331", "time chip 12.5 SemiBold text_a(0.6) on crust@0.5 over hover() over base",
        "text", lambda r: over(r["text"], 0.6, over(r["crust"], 0.5, over(r["text"], 0.07, r["base"]))),
        lambda r: over(r["crust"], 0.5, over(r["text"], 0.07, r["base"])), DARK3)
    add(D, "grid.rs:1209-1216", "'…' glyph of a shelf's song row, text_a(0.35) on base", "control",
        lambda r: over(r["text"], 0.35, r["base"]), lambda r: r["base"], DARK3)
    add(D, "mod.rs:883-885 (M3E)", "snackbar: mix(base, text, 0.9) on text", "text",
        lambda r: mix(r["base"], r["text"], 0.9), lambda r: r["text"], [M3E])
    add(D, "rail.rs:109-115,258-270,339-349; menu.rs:101-104; fab.rs:154-176", "on_secondary_container on secondary_container",
        "text", lambda r: r["on_secondary_container"], lambda r: r["secondary_container"], [M3E])
    add(D, "rail.rs:418-421; toolbar.rs:521-522; search_bar.rs:116-124", "on_tertiary_container on tertiary_container",
        "text", lambda r: r["on_tertiary_container"], lambda r: r["tertiary_container"], [M3E])
    add(D, "rail.rs:481", "on_secondary_container at 0.8 on secondary_container", "text",
        lambda r: over(r["on_secondary_container"], 0.8, r["secondary_container"]),
        lambda r: r["secondary_container"], [M3E])
    add(D, "pane.rs:121-127 (M3E's tab, on)", "tab label 13.5 SemiBold, on_accent() on accent", "text",
        lambda r: r["on_accent()"], lambda r: r["accent"], [M3E])
    # The song table's row (tracks.rs:385-405 fills, :459-463 the title's colour).
    add(D, "tracks.rs:459-463,469-474", "playing song's title 13 Medium / 14.5, accent on base", "text",
        lambda r: r["accent"], lambda r: r["base"], [STOCK, SPOTIFY])
    add(D, "tracks.rs:398-403,459-463", "playing song's title, accent on the hovered row (text at 0.07)", "text",
        lambda r: r["accent"], lambda r: over(r["text"], 0.07, r["base"]), [STOCK, SPOTIFY])
    add(D, "tracks.rs:396-397,459-463", "playing song's title, accent on the selected row (accent at 0.16)", "text",
        lambda r: r["accent"], lambda r: over(r["accent"], 0.16, r["base"]), [STOCK, SPOTIFY])
    add(D, "tracks.rs:396-397", "a selected row's artists, subtext0 on accent at 0.16", "text",
        lambda r: r["subtext0"], lambda r: over(r["accent"], 0.16, r["base"]), DARK3)

    # ---- phone ----
    add(P, "ui/chrome/BottomChrome.kt:117-127 (Stock's tab bar)", "tab label 10.5 Medium, tint on surface2 (tab chosen)",
        "text", lambda r: r["tint"], lambda r: r["surface2"], [STOCK, STOCK_LIGHT])
    add(P, "ui/chrome/BottomChrome.kt:117-126", "tab glyph, tint on surface2 (tab chosen)", "control",
        lambda r: r["tint"], lambda r: r["surface2"], [STOCK, STOCK_LIGHT])
    add(P, "ui/components/Sheet.kt:210-219", "sheet's primary button 15 SemiBold, onAccent on accent", "text",
        lambda r: r["onAccent"], lambda r: r["accent"], [STOCK, STOCK_LIGHT, SPOTIFY, M3E])
    add(P, "ui/components/Page.kt:272-283 (via ui/App.kt:94)", "avatar initial 17 SemiBold, onAccent on the gradient's accent end",
        "text", lambda r: r["onAccent"], lambda r: r["accent"], [STOCK, STOCK_LIGHT, SPOTIFY, M3E])
    add(P, "ui/components/Page.kt:272-283", "avatar initial 17 SemiBold, onAccent on the gradient's mauve end",
        "text", lambda r: r["onAccent"], lambda r: r["mauve"], [STOCK, STOCK_LIGHT, SPOTIFY, M3E])
    add(P, "ui/library/LibraryScreens.kt:640-653 (ui/detail/ArtistPage.kt:569-583)", "Play / Shuffle 16 SemiBold, tint on surface",
        "text", lambda r: r["tint"], lambda r: r["surface"], [STOCK, STOCK_LIGHT, SPOTIFY, M3E])
    add(P, "ui/actions/Sheets.kt:98-103 (in a Group: sheetCell)", "'New Playlist…' 16 px, tint on sheetCell",
        "text", lambda r: r["tint"], lambda r: r["sheetCell"], [STOCK, STOCK_LIGHT, SPOTIFY, M3E])
    add(P, "ui/actions/Sheets.kt:316-318,420-421 (on the sheet)", "'Choose/Change Cover' 15 Medium, accent on sheet",
        "text", lambda r: r["accent"], lambda r: r["sheet"], [STOCK, STOCK_LIGHT, SPOTIFY, M3E])
    add(P, "ui/actions/Sheets.kt:320-321,424-425 (on the sheet)", "'Remove' 15 Medium, red on sheet",
        "text", lambda r: r["red"], lambda r: r["sheet"], [STOCK, STOCK_LIGHT, SPOTIFY, M3E])
    add(P, "ui/actions/Sheets.kt:449-451 (in a Group)", "'Delete Playlist' 16 px, red on sheetCell",
        "text", lambda r: r["red"], lambda r: r["sheetCell"], [STOCK, STOCK_LIGHT, SPOTIFY, M3E])
    add(P, "ui/settings/SettingsSheet.kt:460-461 (on the sheet)", "sign-in failure 13 px, red on sheet",
        "text", lambda r: r["red"], lambda r: r["sheet"], [STOCK, STOCK_LIGHT, SPOTIFY, M3E])
    add(P, "ui/components/Sheet.kt:196-200", "a group's footer 12.5 px, muted on sheet",
        "text", lambda r: r["muted"], lambda r: r["sheet"], [STOCK, STOCK_LIGHT, SPOTIFY, M3E])
    add(P, "ui/components/Filter.kt:62-81 (SearchReveal)", "placeholder 15 px, muted on surface",
        "text", lambda r: r["muted"], lambda r: r["surface"], [STOCK, STOCK_LIGHT, SPOTIFY, M3E])
    add(P, "ui/nowplaying/Devices.kt:200-210", "pairing code placeholder '000000' 40 Bold, muted at 0.4 on sheet",
        "large", lambda r: over(r["muted"], 0.4, r["sheet"]), lambda r: r["sheet"], [STOCK, STOCK_LIGHT, SPOTIFY, M3E])
    add(P, "ui/browse/SearchPage.kt:179-189,596-599 (M3E's search view)", "'Clear' 15 Medium, tint on base",
        "text", lambda r: r["tint"], lambda r: r["base"], [M3E])
    add(P, "ui/chrome/M3EChrome.kt:139-142", "tab label 12 Bold, text on the bar; glyph on secondaryContainer",
        "control", lambda r: r["on_secondary_container"], lambda r: r["secondary_container"], [M3E])
    for bg in ["base", "cell", "surface"]:
        add(P, "color = roles.sub (65 uses)", f"sub text on {bg}", "text",
            lambda r: r["sub"], lambda r, bg=bg: r[bg], [STOCK, STOCK_LIGHT, SPOTIFY, M3E])
        add(P, "color = roles.muted (8 uses), e.g. ui/connect/Connect.kt:431", f"muted text on {bg}", "text",
            lambda r: r["muted"], lambda r, bg=bg: r[bg], [STOCK, STOCK_LIGHT, SPOTIFY, M3E])
    return t


def threshold(kind):
    return 4.5 if kind == "text" else 3.0


# ---- part 3: text on the colours of covers ----


def fitted(L, chroma_steps, hue_step=5):
    """Every colour `fit_in` can give at lightness L (opium-core/src/palette.rs:203-217)."""
    for h in range(0, 360, hue_step):
        for c in chroma_steps:
            yield from_oklab((L, c * math.cos(math.radians(h)), c * math.sin(math.radians(h))))


def sweep(name, colours, fg_on):
    ratios = [(ratio(fg_on(c)[0], fg_on(c)[1]), c) for c in colours]
    lo, hi = min(ratios), max(ratios)
    print(f"  {name}: worst {lo[0]:.2f} (on {hexs(lo[1])}), best {hi[0]:.2f} (on {hexs(hi[1])})")
    return lo[0]


def wash_at(f, base, palette):
    """now_playing.rs:393-428 with the drift at 0: the four blobs over base,
    then base again at the vignette's opacity (mod.rs:1488-1507)."""
    c1, c2, c3 = palette
    blobs = [((0.22, 0.34), (0.42, 0.52), c1, 0.95), ((0.78, 0.22), (0.40, 0.50), c2, 0.85),
             ((0.70, 0.86), (0.46, 0.56), c3, 0.80), ((0.12, 0.92), (0.36, 0.46), c2, 0.60)]
    color = base
    for (cx, cy), (rx, ry), c, a in blobs:
        d = math.hypot((f[0] - cx) / rx, (f[1] - cy) / ry)
        t = min(max(1 - d / 0.85, 0.0), 1.0)
        color = over(c, a * t, color)
    d = math.hypot((f[0] - 0.55) / 1.2, (f[1] - 0.45) / 1.0) * 2
    veil = 0.45 * min(d, 1.0)
    if veil > 0:
        color = over(base, veil, color)
    return color


def part3(all_themes):
    chroma = [0.0, 0.04, 0.08, 0.12, 0.16, 0.20, 0.24]
    print("\nPart 3: text on the colours of covers")
    print(" Spotify's full screen player (now_playing.rs:98-141): white 15 Bold at 32 px from the top,")
    print(" on the cover's colour fitted to L 0.42-0.66, chroma <= 0.24 (palette.rs:25-35 fit_header);")
    print(" a cover lighter than 0.66 (a white or pale cover) is clamped to 0.66:")
    for L in [0.66, 0.60, 0.56, 0.50, 0.42]:
        sweep(f"L {L:.2f}, white text (needs 4.5)", list(fitted(L, chroma)), lambda c: (WHITE, c))
    print("  its close, full screen and lyrics glyphs (now_playing.rs:143-178), white, need 3:")
    sweep("L 0.66, white glyph", list(fitted(0.66, chroma)), lambda c: (WHITE, c))
    spotify = dict(all_themes)[SPOTIFY]
    green = desktop_roles(spotify)["accent"]
    sweep("L 0.66, the lyrics glyph when on, accent #1ed760 (needs 3)", list(fitted(0.66, chroma)), lambda c: (green, c))
    print(" Spotify's lyrics page (now_playing.rs:198-215): white lyrics on the same colour; an unlit line")
    print(" (0.3, lyrics.rs:234) near the top, where the gradient is still the cover's colour:")
    sweep("L 0.66, white at 0.3 (large text, needs 3)", list(fitted(0.66, chroma)), lambda c: (over(WHITE, 0.3, c), c))
    print(" Stock's and M3E's Now Playing (now_playing.rs:393-428): the cover's three colours fitted to")
    print(" L 0.34-0.56, chroma <= 0.20 (palette.rs:181-184), as blobs over base with a vignette; the")
    print(" lyrics' column, x 0.60-1.00, y 0.08-1.00 of the window (now_playing.rs:55-63). Worst point:")
    grid = [(x / 24, y / 16) for x in range(15, 25) for y in range(2, 17)]
    for name in [STOCK, M3E]:
        r = desktop_roles(dict(all_themes)[name])
        worst = {"lit": (99, None), "unlit": (99, None), "unsung": (99, None)}
        for L in [0.56]:
            for c in fitted(L, [0.0, 0.10, 0.20], hue_step=15):
                pal = (c, c, c)
                for f in grid:
                    bg = wash_at(f, r["base"], pal)
                    for k, a in [("lit", 1.0), ("unlit", 0.3), ("unsung", 0.35)]:
                        x = ratio(over(r["text"], a, bg), bg)
                        if x < worst[k][0]:
                            worst[k] = (x, (hexs(bg), f))
        for k, (x, at) in worst.items():
            print(f"  {name}: {k} line (large, needs 3): worst {x:.2f} on {at[0]} at {at[1][0]:.2f},{at[1][1]:.2f}")


def main(root: str) -> None:
    root = pathlib.Path(root)
    print("Port of the colour maths, checked against the code:")
    check_port(root)
    all_themes = list(themes(root))

    print("\nPart 1: every text role on every background role (* under 4.5, ** under 3)")
    fgs_d = ["text", "subtext0", "muted()", "overlay1", "overlay0", "accent", "mauve", "red",
             "green", "yellow", "primary", "on_accent()"]
    bgs_d = ["base", "mantle", "crust", "surface0", "surface1", "surface2"]
    fgs_p = ["text", "sub", "muted", "tint", "accent", "red", "green", "yellow", "primary", "onAccent"]
    bgs_p = ["base", "grouped", "cell", "surface", "surface2", "surface3", "capsule", "sheet", "sheetCell"]
    for name, p in all_themes:
        for app, roles, fgs, bgs in [(D, desktop_roles(p), fgs_d, bgs_d), (P, phone_roles(p, name), fgs_p, bgs_p)]:
            if app == D and "light" in name:
                continue
            print(f"\n{name}, {app}")
            print("  " + "fg \\ bg".ljust(22) + "".join(f"{b} {hexs(roles[b])}".rjust(18) for b in bgs))
            for f in fgs:
                cells = []
                for b in bgs:
                    x = ratio(roles[f], roles[b])
                    mark = "**" if x < 3 else "*" if x < 4.5 else ""
                    cells.append(f"{x:5.2f}{mark}".rjust(18))
                print("  " + f"{f} {hexs(roles[f])}".ljust(22) + "".join(cells))

    print("\nPart 2: the pairs the code draws (FAIL: under the threshold of its kind)")
    fails = 0
    for app, where, what, kind, fg, bg, only in drawn():
        for name, p in all_themes:
            if name not in only:
                continue
            roles = desktop_roles(p) if app == D else phone_roles(p, name)
            f, b = fg(roles), bg(roles)
            x = ratio(f, b)
            verdict = "FAIL" if x < threshold(kind) else "pass"
            fails += verdict == "FAIL"
            print(f"  {verdict} {x:5.2f} (needs {threshold(kind)}) {app:7} {name:12} {what} "
                  f"[{hexs(f)} on {hexs(b)}] {where}")
    print(f"  {fails} failing pairs")
    part3(all_themes)


if __name__ == "__main__":
    main(sys.argv[1])
