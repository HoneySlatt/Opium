//! Music's desktop window drawn by GPUI. It stands on the same workings as
//! the window drawn by egui — the `desktop` library: the services and the
//! player behind it, the settings, the media controls, the other devices —
//! and for now shows only what of the library has come.

use std::collections::HashMap;
use std::sync::Arc;

use desktop::backend::{Backend, Own, Reply, Request};
use gpui::{
    App, Bounds, Context, Window, WindowBounds, WindowOptions, div, prelude::*, px, rgb, size,
};
use gpui_platform::application;
use music_core::model::ProviderKind;

/// The window and what it stands on.
struct Music {
    backend: Backend,
    _devices: desktop::devices::Devices,
    _mpris: desktop::mpris::Mpris,
    /// The library's songs as they come, by service.
    songs: HashMap<ProviderKind, usize>,
}

impl Music {
    fn new(cx: &mut Context<Self>) -> Self {
        // The workings wake the window from their own threads: one wake
        // waiting is enough, the window then reads all that came.
        let (woken, wakes) = async_channel::bounded::<()>(1);
        let wake: desktop::Wake = Arc::new(move || {
            let _ = woken.try_send(());
        });
        cx.spawn(async move |this, cx| {
            while wakes.recv().await.is_ok() {
                if this.update(cx, |this, cx| this.take_replies(cx)).is_err() {
                    break;
                }
            }
        })
        .detach();

        let settings = desktop::settings::Settings::load();
        // The history kept before it was shared starts it, the last first.
        let kept = synced::Kept::open(&desktop::settings::dir(), || {
            let mut history = read_history();
            history.reverse();
            history
        });
        let devices =
            desktop::devices::Devices::start(wake.clone(), kept.clone(), settings.relay.clone());
        let own = Own::new(kept, devices.node.sharer());
        let backend = Backend::start(wake.clone(), settings.services_off.clone(), own);
        backend.send(Request::SetLocalFolders(settings.local_folders.clone()));
        backend.send(Request::Library);
        Self {
            backend,
            _devices: devices,
            _mpris: desktop::mpris::Mpris::start(wake),
            songs: HashMap::new(),
        }
    }

    /// What the workings have said since last time.
    fn take_replies(&mut self, cx: &mut Context<Self>) {
        while let Some(reply) = self.backend.try_recv() {
            if let Reply::Songs {
                provider,
                tracks,
                whole,
                ..
            } = reply
            {
                let count = self.songs.entry(provider).or_default();
                *count = if whole { tracks.len() } else { *count + tracks.len() };
            }
        }
        cx.notify();
    }
}

impl Render for Music {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let mut services: Vec<_> = self.songs.iter().collect();
        services.sort_by_key(|(provider, _)| format!("{provider:?}"));
        let total: usize = self.songs.values().sum();
        div()
            .size_full()
            .bg(rgb(0x121212))
            .text_color(rgb(0xffffff))
            .p_8()
            .gap_2()
            .flex()
            .flex_col()
            .child(div().text_3xl().child(format!("{total} songs")))
            .children(services.into_iter().map(|(provider, n)| {
                div()
                    .text_color(rgb(0xb3b3b3))
                    .child(format!("{provider:?}: {n}"))
            }))
    }
}

/// The history as it was kept before devices shared it.
fn read_history() -> Vec<music_core::model::Track> {
    std::fs::read_to_string(desktop::settings::dir().join("history.json"))
        .ok()
        .and_then(|text| serde_json::from_str(&navidrome::scrub_kept(&text)).ok())
        .unwrap_or_default()
}

fn main() {
    // One Music at a time, whichever toolkit draws it: two would share the
    // settings, the playlists and the downloads and undo each other's writes.
    if desktop::mpris::raise_running() {
        return;
    }
    application().run(|cx: &mut App| {
        let bounds = Bounds::centered(None, size(px(1440.0), px(900.0)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                app_id: Some("music".into()),
                ..Default::default()
            },
            |_, cx| cx.new(Music::new),
        )
        .expect("the window opens");
        cx.activate(true);
    });
}
