//! Geist (SIL Open Font License, `assets/fonts/OFL.txt`) in the four weights the
//! design uses. egui has no font weights, so each one is its own family.

use std::sync::Arc;

use egui::{FontData, FontDefinitions, FontFamily, FontId};

#[derive(Clone, Copy)]
pub enum Weight {
    Regular,
    Medium,
    SemiBold,
    Bold,
}

impl Weight {
    fn family(self) -> &'static str {
        match self {
            Weight::Regular => "geist-400",
            Weight::Medium => "geist-500",
            Weight::SemiBold => "geist-600",
            Weight::Bold => "geist-700",
        }
    }
}

pub fn font(size: f32, weight: Weight) -> FontId {
    FontId::new(size, FontFamily::Name(weight.family().into()))
}

pub fn install(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();
    let faces: [(Weight, &'static [u8]); 4] = [
        (
            Weight::Regular,
            include_bytes!("../assets/fonts/Geist-Regular.otf"),
        ),
        (
            Weight::Medium,
            include_bytes!("../assets/fonts/Geist-Medium.otf"),
        ),
        (
            Weight::SemiBold,
            include_bytes!("../assets/fonts/Geist-SemiBold.otf"),
        ),
        (
            Weight::Bold,
            include_bytes!("../assets/fonts/Geist-Bold.otf"),
        ),
    ];
    // egui's bundled fonts stay behind Geist for glyphs it lacks (CJK, emoji…).
    let fallbacks = fonts
        .families
        .get(&FontFamily::Proportional)
        .cloned()
        .unwrap_or_default();
    for (weight, bytes) in faces {
        let name = weight.family();
        fonts
            .font_data
            .insert(name.into(), Arc::new(FontData::from_static(bytes)));
        let mut family = vec![name.to_owned()];
        family.extend(fallbacks.iter().cloned());
        fonts.families.insert(FontFamily::Name(name.into()), family);
    }
    fonts
        .families
        .entry(FontFamily::Proportional)
        .or_default()
        .insert(0, Weight::Regular.family().into());
    ctx.set_fonts(fonts);
}
