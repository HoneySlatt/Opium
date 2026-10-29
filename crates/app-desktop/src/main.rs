//! Music: Apple Music, SoundCloud and Navidrome in one desktop player.

mod app;
mod backend;
mod cache;
mod color;
mod covers;
mod fonts;
mod icons;
mod mpris;
mod palette;
mod screenshot;
mod settings;
mod theme;
mod ui;

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Music")
            .with_app_id("music")
            .with_inner_size([1440.0, 900.0])
            .with_min_inner_size([760.0, 520.0]),
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
