//! Music's desktop window drawn by GPUI. It stands on the same workings as
//! the window drawn by egui — the `desktop` library: the services and the
//! player behind it, the settings, the media controls, the other devices —
//! and draws, screen by screen, what the egui window draws.

mod app;
mod color;
mod covers;
mod fonts;
mod icons;
mod theme;
mod ui;

use gpui::{App, AppContext, Bounds, WindowBounds, WindowOptions, px, size};
use gpui_platform::application;

fn main() {
    // One Music at a time, whichever toolkit draws it: two would share the
    // settings, the playlists and the downloads and undo each other's writes.
    if desktop::mpris::raise_running() {
        return;
    }
    application().run(|cx: &mut App| {
        fonts::install(cx);
        // Greyscale, as egui sets text.
        cx.set_text_rendering_mode(gpui::TextRenderingMode::Grayscale);
        let bounds = Bounds::centered(None, size(px(1440.0), px(900.0)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                window_min_size: Some(size(px(760.0), px(520.0))),
                app_id: Some("music".into()),
                titlebar: Some(gpui::TitlebarOptions {
                    title: Some("Music".into()),
                    ..Default::default()
                }),
                ..Default::default()
            },
            |_, cx| cx.new(app::Music::new),
        )
        .expect("the window opens");
        cx.activate(true);
    });
}
