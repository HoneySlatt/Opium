//! Opium: Apple Music, SoundCloud, Navidrome and your own files in one
//! desktop player.

mod app;
mod color;
mod covers;
mod fonts;
mod icons;
mod palette;
mod screenshot;
mod theme;
mod ui;

// What the window doesn't draw itself: the library shared with whatever
// else draws it.
use desktop::{backend, devices, mpris, settings};

fn main() -> eframe::Result {
    // Files opened while Opium runs go to it, rather than to a second one;
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
            .with_title("Opium")
            .with_app_id("opium")
            .with_inner_size([1440.0, 900.0])
            .with_min_inner_size([760.0, 520.0])
            .with_icon(
                eframe::icon_data::from_png_bytes(include_bytes!("../assets/icon.png"))
                    .unwrap_or_default(),
            ),
        ..Default::default()
    };
    let result = eframe::run_native(
        "Opium",
        options,
        Box::new(|cc| Ok(Box::new(app::App::new(cc)))),
    );
    desktop::exit::reached("the window closed");
    // Decryption threads may still be finishing: keep them out of the CDM
    // while it is torn down.
    widevine::shutdown();
    desktop::exit::reached("exiting, the CDM's teardown and the libraries'");
    result
}
