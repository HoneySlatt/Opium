//! Music: Apple Music, SoundCloud, Navidrome and your own files in one
//! desktop player.

mod app;
mod backend;
mod cache;
mod color;
mod covers;
mod devices;
mod fonts;
mod icons;
mod mpris;
mod palette;
mod screenshot;
mod settings;
mod soundcloud_browser;
mod theme;
mod ui;

fn main() -> eframe::Result {
    // Files opened while Music runs go to it, rather than to a second one;
    // started again with none, the one running comes forward.
    let files = mpris::files_from_args();
    let running = if files.is_empty() {
        mpris::raise_running()
    } else {
        mpris::open_in_running(&files)
    };
    if running {
        return Ok(());
    }
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Music")
            .with_app_id("music")
            .with_inner_size([1440.0, 900.0])
            .with_min_inner_size([760.0, 520.0])
            .with_icon(
                eframe::icon_data::from_png_bytes(include_bytes!("../assets/icon.png"))
                    .unwrap_or_default(),
            ),
        ..Default::default()
    };
    let result = eframe::run_native(
        "Music",
        options,
        Box::new(|cc| Ok(Box::new(app::App::new(cc)))),
    );
    // Decryption threads may still be finishing: keep them out of the CDM
    // while it is torn down.
    widevine::shutdown();
    result
}
