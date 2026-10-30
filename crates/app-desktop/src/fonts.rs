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

impl Weight {
    const ALL: [Weight; 4] = [
        Weight::Regular,
        Weight::Medium,
        Weight::SemiBold,
        Weight::Bold,
    ];

    fn bytes(self) -> &'static [u8] {
        match self {
            Weight::Regular => include_bytes!("../assets/fonts/Geist-Regular.otf"),
            Weight::Medium => include_bytes!("../assets/fonts/Geist-Medium.otf"),
            Weight::SemiBold => include_bytes!("../assets/fonts/Geist-SemiBold.otf"),
            Weight::Bold => include_bytes!("../assets/fonts/Geist-Bold.otf"),
        }
    }

    fn wght(self) -> f32 {
        match self {
            Weight::Regular => 400.0,
            Weight::Medium => 500.0,
            Weight::SemiBold => 600.0,
            Weight::Bold => 700.0,
        }
    }
}

/// Geist now; the system's fonts for the scripts it lacks (Greek, CJK,
/// Arabic, Hebrew, Thai, Devanagari) once they are found, in the background.
pub fn install(ctx: &egui::Context) {
    ctx.set_fonts(definitions(&[]));
    let ctx = ctx.clone();
    let _ = std::thread::Builder::new()
        .name("fonts".into())
        .spawn(move || {
            let found = system_fallbacks();
            if !found.is_empty() {
                ctx.set_fonts(definitions(&found));
                ctx.request_repaint();
            }
        });
}

/// A system font for a script Geist lacks.
struct Fallback {
    name: String,
    bytes: Arc<Vec<u8>>,
    index: u32,
    /// Whether it has a weight axis, to follow Geist's weights.
    variable: bool,
}

fn definitions(system: &[Fallback]) -> FontDefinitions {
    let mut fonts = FontDefinitions::default();
    // egui's bundled fonts stay last, for emoji.
    let bundled = fonts
        .families
        .get(&FontFamily::Proportional)
        .cloned()
        .unwrap_or_default();
    for weight in Weight::ALL {
        let name = weight.family();
        fonts
            .font_data
            .insert(name.into(), Arc::new(FontData::from_static(weight.bytes())));
        let mut family = vec![name.to_owned()];
        for fallback in system {
            let key = format!("{}-{}", fallback.name, weight.wght());
            let mut data = FontData::from_blob(fallback.bytes.clone(), fallback.index);
            if fallback.variable {
                data.tweak.coords =
                    egui::epaint::text::VariationCoords::new([(b"wght", weight.wght())]);
            }
            fonts.font_data.insert(key.clone(), Arc::new(data));
            family.push(key);
        }
        family.extend(bundled.iter().cloned());
        fonts.families.insert(FontFamily::Name(name.into()), family);
    }
    fonts
        .families
        .entry(FontFamily::Proportional)
        .or_default()
        .insert(0, Weight::Regular.family().into());
    fonts
}

/// The best sans font fontconfig knows for each script Geist lacks.
fn system_fallbacks() -> Vec<Fallback> {
    let mut found: Vec<Fallback> = Vec::new();
    for lang in ["el", "ja", "ar", "he", "th", "hi"] {
        let Ok(out) = std::process::Command::new("fc-list")
            .arg(format!(":lang={lang}"))
            .arg("-f")
            .arg("%{file}|%{index}|%{family[0]}|%{style[0]}|%{variable}\\n")
            .output()
        else {
            return found;
        };
        let list = String::from_utf8_lossy(&out.stdout);
        let best = list
            .lines()
            .filter_map(|line| {
                let mut f = line.split('|');
                let (file, index, family, style, variable) =
                    (f.next()?, f.next()?, f.next()?, f.next()?, f.next()?);
                let index: u32 = index.parse().ok()?;
                let variable = variable == "True";
                let odd = [
                    "Mono", "Serif", "Looped", "Kufi", "Rashi", "Unifont", "Geist",
                ]
                .iter()
                .any(|w| family.contains(w))
                    || ["Condensed", "Oblique", "Italic"]
                        .iter()
                        .any(|w| style.contains(w));
                // A named instance of a variable font, or another weight.
                let regular = variable && index >> 16 == 0
                    || !variable && (style == "Regular" || style == "Book");
                if odd || !regular {
                    return None;
                }
                let score = 4 * u8::from(family.starts_with("Noto Sans"))
                    + 2 * u8::from(variable)
                    + u8::from(family.ends_with(" JP"));
                Some((score, file, index & 0xffff, family, variable))
            })
            .max_by_key(|c| c.0);
        let Some((_, file, index, family, variable)) = best else {
            continue;
        };
        if found.iter().any(|f| f.name == family) {
            continue;
        }
        if let Ok(bytes) = std::fs::read(file) {
            found.push(Fallback {
                name: family.to_owned(),
                bytes: Arc::new(bytes),
                index,
                variable,
            });
        }
    }
    found
}
