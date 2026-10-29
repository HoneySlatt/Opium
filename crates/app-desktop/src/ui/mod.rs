//! The window: sidebar, page, top bar and player bar, after `design/…/DESIGN.md`.

mod detail;
mod drawer;
mod player_bar;
mod sidebar;
mod signal;
mod songs;
mod tracks;

use std::sync::Arc;
use std::time::Duration;

use egui::text::{LayoutJob, TextWrapping};
use egui::{
    Color32, CornerRadius, FontId, Galley, Mesh, Pos2, Rect, Response, Sense, Stroke, Ui, Vec2,
    pos2, vec2,
};

use crate::app::{App, Page, SignalAnchor};
use crate::color::alpha;
use crate::fonts::{Weight, font};
use crate::icons::{self, Icon};
use crate::theme::Theme;

/// Below this window width the sidebar collapses and padding tightens.
pub const NARROW: f32 = 1100.0;
pub const TOP_BAR: f32 = 52.0;

pub fn show(app: &mut App, ui: &mut Ui) {
    let narrow = ui.available_width() < NARROW;
    let collapsed = app.sidebar_collapsed || narrow;
    let theme = app.theme.clone();

    egui::Panel::left("sidebar")
        .exact_size(if collapsed { 68.0 } else { 232.0 })
        .resizable(false)
        .show_separator_line(false)
        .frame(egui::Frame::new().fill(theme.crust))
        .show(ui, |ui| sidebar::show(app, ui, collapsed));

    egui::CentralPanel::no_frame()
        .frame(egui::Frame::new().fill(theme.base))
        .show(ui, |ui| {
            let main = ui.max_rect();
            // The sidebar's right hairline.
            ui.painter()
                .vline(main.left(), main.y_range(), Stroke::new(1.0, theme.line()));
            let scrolled = page(app, ui, narrow);
            top_bar(app, ui, main, scrolled);
            drawer::show(app, ui, main);
            player_bar::show(app, ui, main);
            signal::show(app, ui, main);
            toast(app, ui, main);
        });

    shortcuts(app, ui);
}

/// The cover colours behind album and playlist pages, fitted to the theme.
pub fn backdrop(app: &mut App) -> Option<[Color32; 3]> {
    let url = match &app.page {
        Page::Album(id) => match app.albums.get(id) {
            Some(crate::app::Loaded::Ready((album, _))) => album.artwork.as_ref()?.url(96),
            _ => return None,
        },
        Page::Playlist(id) => app.playlist(id)?.artwork.as_ref()?.url(96),
        _ => return None,
    };
    let light = app.theme.light;
    app.palette(&url)
        .map(|p| p.map(|c| crate::palette::fit(c, light)))
}

/// The page under the top bar; returns how far it is scrolled.
fn page(app: &mut App, ui: &mut Ui, narrow: bool) -> f32 {
    let pad = if narrow { 24.0 } else { 40.0 };
    match app.page.clone() {
        Page::Songs => songs::show(app, ui, pad),
        Page::Album(id) => detail::album(app, ui, pad, &id),
        Page::Playlist(id) => detail::playlist(app, ui, pad, &id),
        _ => placeholder(app, ui, pad),
    }
}

fn placeholder(app: &mut App, ui: &mut Ui, pad: f32) -> f32 {
    let theme = &app.theme;
    let rect = ui.max_rect();
    glow(
        ui,
        theme,
        Rect::from_min_size(rect.min, vec2(rect.width(), 460.0)),
    );
    let title = app.page_title();
    let left = rect.left() + pad;
    let galley = text(
        ui,
        &title,
        font(30.0, Weight::Bold),
        theme.text,
        rect.width(),
    );
    ui.painter()
        .galley(pos2(left, rect.top() + 70.0), galley, theme.text);
    let note = text(
        ui,
        "This page comes in a later step.",
        font(13.0, Weight::Regular),
        theme.subtext0,
        rect.width(),
    );
    ui.painter()
        .galley(pos2(left, rect.top() + 120.0), note, theme.subtext0);
    0.0
}

fn top_bar(app: &mut App, ui: &mut Ui, main: Rect, scrolled: f32) {
    let theme = app.theme.clone();
    let bar = Rect::from_min_size(main.min, vec2(main.width(), TOP_BAR));
    let solid =
        ui.ctx()
            .animate_bool_with_time(egui::Id::unique("top-bar-solid"), scrolled > 24.0, 0.2);
    if solid > 0.0 {
        let painter = ui.painter();
        painter.rect_filled(bar, 0.0, alpha(theme.base, 0.96 * solid));
        painter.hline(
            bar.x_range(),
            bar.bottom() - 0.5,
            Stroke::new(1.0, alpha(theme.line(), solid)),
        );
        let title = text(
            ui,
            &app.page_title(),
            font(13.0, Weight::SemiBold),
            theme.text,
            400.0,
        );
        let pos = bar.center() - title.size() / 2.0;
        ui.painter().galley(pos, title, alpha(theme.text, solid));
    }

    let mut left = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(Rect::from_min_size(
                bar.min + vec2(18.0, 10.0),
                vec2(200.0, 32.0),
            ))
            .layout(egui::Layout::left_to_right(egui::Align::Center)),
    );
    left.spacing_mut().item_spacing.x = 4.0;
    if icon_button(
        &mut left,
        &theme,
        icons::LEFT,
        32.0,
        18.0,
        theme.subtext0,
        app.can_go_back(),
    )
    .clicked()
    {
        app.go_back();
    }
    if icon_button(
        &mut left,
        &theme,
        icons::RIGHT,
        32.0,
        18.0,
        theme.subtext0,
        app.can_go_forward(),
    )
    .clicked()
    {
        app.go_forward();
    }
    let (sep, _) = left.allocate_exact_size(vec2(9.0, 16.0), Sense::hover());
    left.painter().vline(
        sep.center().x,
        sep.y_range(),
        Stroke::new(1.0, theme.line()),
    );
    if icon_button(
        &mut left,
        &theme,
        icons::PANEL,
        32.0,
        17.0,
        theme.subtext0,
        true,
    )
    .clicked()
    {
        app.sidebar_collapsed = !app.sidebar_collapsed;
    }

    let mut right = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(Rect::from_min_max(
                pos2(bar.right() - 18.0 - 104.0, bar.top() + 10.0),
                pos2(bar.right() - 18.0, bar.top() + 42.0),
            ))
            .layout(egui::Layout::left_to_right(egui::Align::Center)),
    );
    right.spacing_mut().item_spacing.x = 4.0;
    icon_button(
        &mut right,
        &theme,
        icons::LYRICS,
        32.0,
        17.0,
        theme.subtext0,
        true,
    )
    .on_hover_text("Lyrics — comes in a later step");
    let queue_color = if app.drawer_open {
        theme.accent
    } else {
        theme.subtext0
    };
    if icon_button(
        &mut right,
        &theme,
        icons::QUEUE,
        32.0,
        17.0,
        queue_color,
        true,
    )
    .on_hover_text("Playing next")
    .clicked()
    {
        app.drawer_open = !app.drawer_open;
    }
    let wave_color = if app.signal_open == Some(SignalAnchor::TopBar) {
        theme.accent
    } else {
        theme.subtext0
    };
    let wave = icon_button(
        &mut right,
        &theme,
        icons::WAVE,
        32.0,
        17.0,
        wave_color,
        true,
    )
    .on_hover_text("Signal path");
    signal::remember_anchor(&right, SignalAnchor::TopBar, wave.rect);
    if wave.clicked() {
        app.signal_open = match app.signal_open {
            Some(SignalAnchor::TopBar) => None,
            _ => app.now.is_some().then_some(SignalAnchor::TopBar),
        };
    }
}

fn toast(app: &mut App, ui: &mut Ui, main: Rect) {
    let Some((message, since)) = app.toast.clone() else {
        return;
    };
    let now = ui.input(|i| i.time);
    if now - since > 5.0 {
        app.toast = None;
        return;
    }
    ui.ctx()
        .request_repaint_after(Duration::from_secs_f64(5.0 - (now - since)));
    let theme = &app.theme;
    let galley = text(ui, &message, font(13.0, Weight::Regular), theme.text, 560.0);
    let size = galley.size() + vec2(28.0, 18.0);
    let rect = Rect::from_center_size(
        pos2(
            main.center().x,
            main.bottom() - 14.0 - 44.0 - 16.0 - size.y / 2.0,
        ),
        size,
    );
    let painter = ui.painter();
    painter.add(
        egui::Shadow {
            offset: [0, 12],
            blur: 32,
            spread: 0,
            color: Color32::from_black_alpha(110),
        }
        .as_shape(rect, 10.0),
    );
    painter.rect(
        rect,
        10.0,
        theme.mantle,
        Stroke::new(1.0, theme.line()),
        egui::StrokeKind::Inside,
    );
    painter.galley(rect.min + vec2(14.0, 9.0), galley, theme.text);
}

fn shortcuts(app: &mut App, ui: &mut Ui) {
    let typing = ui.ctx().egui_wants_keyboard_input();
    if !typing && ui.input(|i| i.key_pressed(egui::Key::Space)) {
        app.toggle_play();
    }
    // Escape closes the popover first, then the drawer.
    if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
        if app.signal_open.is_some() {
            app.signal_open = None;
        } else {
            app.drawer_open = false;
        }
    }
}

// ---- Shared drawing helpers ----

/// One line of text, cut with an ellipsis past `max_width`.
pub fn text(ui: &Ui, s: &str, font: FontId, color: Color32, max_width: f32) -> Arc<Galley> {
    let mut job = LayoutJob::simple_singleline(s.to_owned(), font, color);
    job.wrap = TextWrapping::truncate_at_width(max_width.max(0.0));
    ui.painter().layout_job(job)
}

/// Text vertically centred on `y`, from `x`, with `align` horizontally.
pub fn paint_text(
    ui: &Ui,
    galley: Arc<Galley>,
    x: f32,
    y: f32,
    align: egui::Align,
    color: Color32,
) {
    let size = galley.size();
    let left = match align {
        egui::Align::Min => x,
        egui::Align::Center => x - size.x / 2.0,
        egui::Align::Max => x - size.x,
    };
    ui.painter()
        .galley(pos2(left, y - size.y / 2.0), galley, color);
}

pub fn paint_icon(ui: &Ui, icon: Icon, center: Pos2, size: f32, color: Color32) {
    icon.image(size, color)
        .paint_at(ui, Rect::from_center_size(center, Vec2::splat(size)));
}

/// A square icon button: no background until hovered.
pub fn icon_button(
    ui: &mut Ui,
    theme: &Theme,
    icon: Icon,
    size: f32,
    glyph: f32,
    color: Color32,
    enabled: bool,
) -> Response {
    let sense = if enabled {
        Sense::click()
    } else {
        Sense::hover()
    };
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(size), sense);
    let hovered = enabled && response.hovered();
    if hovered {
        let fill = if response.is_pointer_button_down_on() {
            theme.press()
        } else {
            theme.hover()
        };
        ui.painter().rect_filled(rect, 8.0, fill);
    }
    let color = if !enabled {
        alpha(color, 0.35 * color.a() as f32 / 255.0)
    } else if hovered {
        theme.text
    } else {
        color
    };
    paint_icon(ui, icon, rect.center(), glyph, color);
    if enabled {
        response.on_hover_cursor(egui::CursorIcon::PointingHand)
    } else {
        response
    }
}

/// The "E" badge after explicit titles.
pub fn explicit_badge(ui: &Ui, theme: &Theme, center: Pos2) {
    let rect = Rect::from_center_size(center, Vec2::splat(12.0));
    ui.painter().rect_filled(
        rect,
        2.5,
        crate::color::mix(theme.subtext0, theme.base, 0.55),
    );
    let e = text(ui, "E", font(8.5, Weight::Bold), theme.base, 12.0);
    paint_text(ui, e, center.x, center.y, egui::Align::Center, theme.base);
}

/// A track's quality tag (`AAC 256`, `FLAC 24/96`). Returns its width.
pub fn quality_tag(ui: &Ui, theme: &Theme, label: &str, lossless: bool, left_center: Pos2) -> f32 {
    let color = if lossless {
        crate::color::mix(theme.accent, theme.text, 0.70)
    } else {
        theme.subtext0
    };
    let border = if lossless {
        alpha(theme.accent, 0.55)
    } else {
        alpha(theme.subtext0, 0.40)
    };
    let galley = text(ui, label, font(10.0, Weight::SemiBold), color, 200.0);
    let rect = Rect::from_min_size(
        pos2(left_center.x, left_center.y - 8.5),
        vec2(galley.size().x + 12.0, 17.0),
    );
    ui.painter().rect_stroke(
        rect,
        4.0,
        Stroke::new(1.0, border),
        egui::StrokeKind::Inside,
    );
    paint_text(
        ui,
        galley,
        rect.center().x,
        rect.center().y,
        egui::Align::Center,
        color,
    );
    rect.width()
}

/// `AAC 256`, `FLAC 24/96`… from what the provider says it delivers.
pub fn quality_label(track: &music_core::model::Track) -> Option<(String, bool)> {
    use music_core::model::Codec;
    let q = track.quality.as_ref()?;
    let codec = match &q.codec {
        Codec::Aac => "AAC",
        Codec::Alac => "ALAC",
        Codec::Flac => "FLAC",
        Codec::Mp3 => "MP3",
        Codec::Opus => "Opus",
        Codec::Vorbis => "Vorbis",
        Codec::Wav => "WAV",
        Codec::Other(name) => name.as_str(),
    };
    let detail = match (q.bit_depth, q.sample_rate, q.bitrate_kbps) {
        (Some(bits), Some(rate), _) if q.codec.is_lossless() => {
            format!(" {bits}/{}", trim_khz(rate))
        }
        (_, _, Some(kbps)) => format!(" {kbps}"),
        _ => String::new(),
    };
    Some((format!("{codec}{detail}"), q.codec.is_lossless()))
}

fn trim_khz(rate: u32) -> String {
    let khz = f64::from(rate) / 1000.0;
    if khz.fract() == 0.0 {
        format!("{khz:.0}")
    } else {
        format!("{khz:.1}")
    }
}

pub fn format_time(d: Duration) -> String {
    let s = d.as_secs();
    format!("{}:{:02}", s / 60, s % 60)
}

/// Artwork from a URL template, as large as needed on this screen.
pub fn artwork(ui: &Ui, url: Option<String>, rect: Rect, radius: f32, theme: &Theme) {
    ui.painter().rect_filled(rect, radius, theme.surface0);
    if let Some(url) = url {
        egui::Image::new(url)
            .corner_radius(CornerRadius::same(radius as u8))
            .show_loading_spinner(false)
            .paint_at(ui, rect);
    }
}

/// Artwork URLs share one size so the list, the sidebar and the player bar hit
/// the same cached image.
pub fn thumb_url(track: &music_core::model::Track) -> Option<String> {
    track.artwork.as_ref().map(|a| a.url(120))
}

/// A radial blob of colour: centre and radii as fractions of the painted rect,
/// opacity at the centre, and where (as a fraction of the radius) it fades out.
pub struct Blob {
    pub center: Vec2,
    pub radii: Vec2,
    pub color: Color32,
    pub alpha: f32,
    pub fade: f32,
}

/// Blobs over `base` as a coarse mesh, with per-vertex colours. With
/// `fade_to_base`, a vertical gradient then dissolves them into `base`: clear to
/// 18 %, `base` at 55 % opacity by 55 %, solid at the bottom.
pub fn wash(ui: &Ui, rect: Rect, base: Color32, blobs: &[Blob], fade_to_base: bool) {
    let (cols, rows) = (24, 16);
    let mut mesh = Mesh::default();
    for row in 0..=rows {
        for col in 0..=cols {
            let f = vec2(col as f32 / cols as f32, row as f32 / rows as f32);
            let mut color = base;
            for blob in blobs {
                let d = ((f - blob.center) / blob.radii).length();
                let t = (1.0 - d / blob.fade).clamp(0.0, 1.0);
                color = over(alpha(blob.color, blob.alpha * t), color);
            }
            if fade_to_base {
                let a = if f.y < 0.18 {
                    0.0
                } else if f.y < 0.55 {
                    0.55 * (f.y - 0.18) / (0.55 - 0.18)
                } else {
                    0.55 + 0.45 * (f.y - 0.55) / 0.45
                };
                color = over(alpha(base, a), color);
            }
            mesh.colored_vertex(rect.min + f * rect.size(), color);
        }
    }
    for row in 0..rows {
        for col in 0..cols {
            let i = (row * (cols + 1) + col) as u32;
            let w = (cols + 1) as u32;
            mesh.add_triangle(i, i + 1, i + w);
            mesh.add_triangle(i + 1, i + w + 1, i + w);
        }
    }
    ui.painter().add(mesh);
}

/// The violet glow at the top of pages without a cover: two radial gradients
/// (accent at 26 % from the top left, mauve at 20 % from the top centre).
pub fn glow(ui: &Ui, theme: &Theme, rect: Rect) {
    let blobs = [
        Blob {
            center: vec2(0.08, -0.10),
            radii: vec2(0.55, 0.90),
            color: theme.accent,
            alpha: 0.26,
            fade: 0.70,
        },
        Blob {
            center: vec2(0.62, -0.25),
            radii: vec2(0.45, 0.80),
            color: theme.mauve,
            alpha: 0.20,
            fade: 0.72,
        },
    ];
    wash(ui, rect, theme.base, &blobs, false);
}

/// The Play and Shuffle buttons of a page, from `at`. Returns `Some(shuffle)`
/// when one was clicked.
pub fn page_buttons(ui: &mut Ui, theme: &Theme, at: Pos2, salt: &str) -> Option<bool> {
    let play = Rect::from_min_size(at, vec2(88.0, 36.0));
    let shuffle = Rect::from_min_size(pos2(play.right() + 10.0, at.y), vec2(107.0, 36.0));
    if page_button(ui, theme, play, icons::PLAY_FILL, "Play", true, salt) {
        return Some(false);
    }
    if page_button(ui, theme, shuffle, icons::SHUFFLE, "Shuffle", false, salt) {
        return Some(true);
    }
    None
}

/// A 36 px button: accent-filled for the primary action, else a faint fill.
fn page_button(
    ui: &mut Ui,
    theme: &Theme,
    rect: Rect,
    icon: Icon,
    label: &str,
    primary: bool,
    salt: &str,
) -> bool {
    let response = ui.interact(
        rect,
        egui::Id::unique(("page-button", salt, label)),
        Sense::click(),
    );
    let (fill, fg) = if primary {
        let fill = if response.hovered() {
            crate::color::mix(theme.accent, theme.text, 0.88)
        } else {
            theme.accent
        };
        (fill, theme.on_accent())
    } else {
        let fill = theme.text_a(if response.hovered() { 0.14 } else { 0.09 });
        (fill, theme.text)
    };
    let rect = if response.is_pointer_button_down_on() {
        rect.shrink(0.4)
    } else {
        rect
    };
    ui.painter().rect_filled(rect, 9.0, fill);
    paint_icon(
        ui,
        icon,
        pos2(rect.left() + 20.0 + 7.0, rect.center().y),
        14.0,
        fg,
    );
    let galley = text(ui, label, font(13.0, Weight::Medium), fg, rect.width());
    paint_text(
        ui,
        galley,
        rect.left() + 20.0 + 14.0 + 8.0,
        rect.center().y,
        egui::Align::Min,
        fg,
    );
    response
        .on_hover_cursor(egui::CursorIcon::PointingHand)
        .clicked()
}

/// `top` composited over `bottom`, both premultiplied.
fn over(top: Color32, bottom: Color32) -> Color32 {
    let a = top.a() as f32 / 255.0;
    let mix = |t: u8, b: u8| (f32::from(t) + f32::from(b) * (1.0 - a)).round().min(255.0) as u8;
    Color32::from_rgba_premultiplied(
        mix(top.r(), bottom.r()),
        mix(top.g(), bottom.g()),
        mix(top.b(), bottom.b()),
        mix(top.a(), bottom.a()),
    )
}
