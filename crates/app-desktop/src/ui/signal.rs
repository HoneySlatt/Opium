//! The signal path popover: whether the card receives the decoded samples
//! unchanged, and every step from the service to the card.

use egui::{Color32, Id, LayerId, Order, Rect, Sense, Stroke, Ui, pos2, vec2};
use music_core::model::{Codec, ProviderKind, Track};

use crate::app::{App, SignalAnchor};
use crate::color::alpha;
use crate::fonts::{Weight, font};
use crate::icons::{self, Icon};

use super::{icon_button, paint_icon, quality_label};

const WIDTH: f32 = 380.0;
const PAD: f32 = 16.0;

/// What stands between the decoder and the card.
pub struct Assessment {
    pub issues: Vec<String>,
    volume_issue: bool,
}

impl Assessment {
    pub fn perfect(&self) -> bool {
        self.issues.is_empty()
    }
}

/// Judge the path as the player reports it. Before anything plays there is
/// nothing to judge, and nothing is flagged.
pub fn assess(app: &App) -> Assessment {
    let mut issues = Vec::new();
    let mut volume_issue = false;
    if let (Some(source), Some(output)) = (&app.status.source, &app.status.output) {
        if output.sample_rate != source.sample_rate {
            issues.push(format!(
                "PipeWire resamples {} → {} kHz",
                khz(source.sample_rate),
                khz(output.sample_rate)
            ));
        }
        if app.muted {
            issues.push("Muted".into());
            volume_issue = true;
        } else if app.volume < 1.0 {
            issues.push(format!(
                "Software volume {:.1} dB",
                20.0 * app.volume.log10()
            ));
            volume_issue = true;
        }
        let mixing = &output.mixing;
        if mixing.device_muted {
            issues.push("The device is muted in PipeWire".into());
            volume_issue = true;
        } else if let Some(gain) = mixing.device_volume {
            issues.push(format!("Device volume at {} % in PipeWire", percent(gain)));
            volume_issue = true;
        }
        if !mixing.others.is_empty() {
            issues.push(format!("Mixed with {}", mixing.others.join(", ")));
        }
    }
    Assessment {
        issues,
        volume_issue,
    }
}

/// PipeWire's volumes are linear gains; `wpctl` and the desktop show their
/// cube root.
fn percent(gain: f32) -> u32 {
    (gain.cbrt() * 100.0).round() as u32
}

fn khz(rate: u32) -> String {
    let khz = f64::from(rate) / 1000.0;
    if khz.fract() == 0.0 {
        format!("{khz:.0}")
    } else {
        format!("{khz:.1}")
    }
}

/// Wrapped text, `width` wide.
fn para(
    ui: &Ui,
    s: &str,
    size: f32,
    weight: Weight,
    color: Color32,
    width: f32,
) -> std::sync::Arc<egui::Galley> {
    ui.painter()
        .layout(s.to_owned(), font(size, weight), color, width)
}

/// One step of the path: an icon in a disc, a title, and lines under it.
pub(super) struct Step {
    pub icon: Icon,
    pub tone: Tone,
    pub title: String,
    pub lines: Vec<String>,
}

#[derive(Clone, Copy, PartialEq)]
pub(super) enum Tone {
    Neutral,
    Ok,
    Warn,
}

pub(super) fn steps(app: &App, track: &Track) -> Vec<Step> {
    let source = app.status.source.as_ref();
    let output = app.status.output.as_ref();
    let rate = source.map(|s| s.sample_rate).unwrap_or(44_100);
    let channels = source.map(|s| s.channels).unwrap_or(2);
    let lossless = track
        .quality
        .as_ref()
        .is_some_and(|q| q.codec.is_lossless());

    let (service, service_icon, transport) = match track.id.provider {
        ProviderKind::AppleMusic => (
            "Apple Music",
            icons::APPLE_MUSIC,
            "Widevine CENC, decrypted in-process",
        ),
        ProviderKind::SoundCloud => (
            "SoundCloud",
            icons::SOUNDCLOUD,
            match track.quality.as_ref().map(|q| &q.codec) {
                Some(Codec::Other(o)) if o == "Original" => "the uploader's own file",
                _ => "HLS; Widevine CENC for a label's tracks",
            },
        ),
        ProviderKind::Navidrome => (
            "Navidrome",
            icons::NAVIDROME,
            "stream?format=raw, original file",
        ),
        // No song is Music's own.
        ProviderKind::Music => ("Music", icons::NOTE, ""),
    };
    let detail = match track.quality.as_ref() {
        Some(q) if q.codec == Codec::Aac => match q.bitrate_kbps {
            Some(kbps) => format!("AAC-LC · {kbps} kb/s"),
            None => "AAC-LC".into(),
        },
        _ => quality_label(track).map(|(l, _)| l).unwrap_or_default(),
    };
    let codec = source.map(|s| s.codec.clone()).unwrap_or_default();
    let layout = if channels == 2 {
        "stereo".to_owned()
    } else {
        format!("{channels} channels")
    };
    let mut decoder = format!("{codec} → 32-bit integer PCM, {} kHz, {layout}", khz(rate));
    if !lossless {
        decoder.push_str(". Lossy at the source: this is as good as the service sends.");
    }

    let volume_line = if app.muted {
        "Muted".to_owned()
    } else if app.volume < 1.0 {
        format!("Software volume {:.1} dB", 20.0 * app.volume.log10())
    } else {
        "Software volume at 100 %, samples untouched".to_owned()
    };
    let volume_ok = !app.muted && app.volume >= 1.0;

    let mut pipewire = Vec::new();
    let mut pipewire_ok = true;
    if let Some(output) = output {
        if output.sample_rate == rate {
            pipewire.push(format!(
                "Graph forced to {} kHz (node.force-rate). The music is not resampled.",
                khz(rate)
            ));
        } else {
            pipewire_ok = false;
            pipewire.push(format!(
                "Graph at {} kHz: the music is resampled from {} kHz.",
                khz(output.sample_rate),
                khz(rate)
            ));
        }
        for other in &output.mixing.others {
            pipewire_ok = false;
            pipewire.push(format!("{other} · playing, mixed in"));
        }
        if output.mixing.device_muted {
            pipewire_ok = false;
            pipewire.push("Device muted".into());
        } else if let Some(gain) = output.mixing.device_volume {
            pipewire_ok = false;
            pipewire.push(format!(
                "Device volume at {} %: samples scaled",
                percent(gain)
            ));
        }
    }
    let device = output
        .map(|o| o.device.clone())
        .unwrap_or_else(|| "PipeWire".into());
    let format = output
        .map(|o| {
            format!(
                "{} · {} kHz · {} ch",
                o.sample_format,
                khz(o.sample_rate),
                o.channels
            )
        })
        .unwrap_or_default();
    let tone = |ok: bool| if ok { Tone::Ok } else { Tone::Warn };

    vec![
        Step {
            icon: service_icon,
            tone: Tone::Neutral,
            title: service.into(),
            lines: vec![detail, transport.into()],
        },
        Step {
            icon: icons::CPU,
            tone: Tone::Neutral,
            title: "Decoder".into(),
            lines: vec![decoder],
        },
        Step {
            icon: icons::VOL,
            tone: tone(volume_ok),
            title: "Volume".into(),
            lines: vec![volume_line],
        },
        Step {
            icon: icons::PW,
            tone: tone(pipewire_ok),
            title: "PipeWire".into(),
            lines: pipewire,
        },
        Step {
            icon: icons::SPEAKER,
            tone: Tone::Ok,
            title: device,
            lines: vec![format],
        },
    ]
}

/// Show the popover if it is open. `anchor` is the rect it hangs from.
pub fn show(app: &mut App, ui: &mut Ui, main: Rect) {
    let Some(anchor) = app.signal_open else {
        return;
    };
    // Nothing to describe until a track plays.
    let Some(track) = app.now.clone() else {
        return;
    };
    let anchor_rect = ui
        .ctx()
        .data(|d| d.get_temp::<Rect>(anchor.id()))
        .unwrap_or(Rect::from_min_size(main.right_top(), vec2(0.0, 0.0)));
    let theme = app.theme.clone();
    let assessment = assess(app);
    let steps = steps(app, &track);
    let inner = WIDTH - 2.0 * PAD;

    // Lay everything out first: the popover's height decides where it goes.
    let (title, summary) = if assessment.perfect() {
        (
            "Bit-perfect",
            "The card receives the decoded samples unchanged.".to_owned(),
        )
    } else {
        let mut s = assessment.issues.join(" · ");
        s.push('.');
        if assessment.volume_issue {
            s.push_str(" Leave the volumes at 100 % and use the device's own knob.");
        }
        ("Not bit-perfect", s)
    };
    let title_g = para(
        ui,
        title,
        15.0,
        Weight::SemiBold,
        theme.text,
        inner - 32.0 - 10.0 - 36.0,
    );
    let summary_g = para(
        ui,
        &summary,
        12.0,
        Weight::Regular,
        theme.muted(),
        inner - 32.0 - 10.0 - 36.0,
    );
    let head_h = (title_g.size().y + summary_g.size().y).max(32.0);
    let text_w = inner - 24.0 - 12.0;
    let step_g: Vec<_> = steps
        .iter()
        .map(|s| {
            let t = para(ui, &s.title, 13.0, Weight::Medium, theme.text, text_w);
            let lines: Vec<_> = s
                .lines
                .iter()
                .filter(|l| !l.is_empty())
                .map(|l| para(ui, l, 12.0, Weight::Regular, theme.muted(), text_w))
                .collect();
            (t, lines)
        })
        .collect();
    let step_h: Vec<f32> = step_g
        .iter()
        .map(|(t, lines)| {
            (t.size().y + lines.iter().map(|l| l.size().y).sum::<f32>()).max(24.0) + 14.0
        })
        .collect();
    let foot_g = para(
        ui,
        "When another program makes sound on this device, it is mixed in and the status turns amber.",
        12.0,
        Weight::Regular,
        theme.muted(),
        inner,
    );
    let height =
        PAD + head_h + 14.0 + step_h.iter().sum::<f32>() + 12.0 + 12.0 + foot_g.size().y + PAD;

    // Above the player bar's chip, or below the top bar's button.
    let x = (anchor_rect.right() - WIDTH).clamp(main.left() + 8.0, main.right() - WIDTH - 8.0);
    let y = match anchor {
        SignalAnchor::TopBar => anchor_rect.bottom() + 6.0,
        SignalAnchor::PlayerBar => anchor_rect.top() - 6.0 - height,
    }
    .clamp(
        main.top() + 8.0,
        (main.bottom() - height - 8.0).max(main.top() + 8.0),
    );
    let rect = Rect::from_min_size(pos2(x, y), vec2(WIDTH, height));

    let mut layer = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(rect)
            .layer_id(LayerId::new(Order::Tooltip, Id::unique("signal-path"))),
    );
    let ui = &mut layer;
    ui.painter().add(
        egui::Shadow {
            offset: [0, 18],
            blur: 48,
            spread: 0,
            color: Color32::from_black_alpha(120),
        }
        .as_shape(rect.shrink(8.0), 14.0),
    );
    ui.painter().rect(
        rect,
        14.0,
        theme.mantle,
        Stroke::new(1.0, theme.line()),
        egui::StrokeKind::Inside,
    );
    ui.interact(rect, Id::unique("signal-path-bg"), Sense::click());

    // Status.
    let left = rect.left() + PAD;
    let top = rect.top() + PAD;
    let (dot_color, dot_icon) = if assessment.perfect() {
        (theme.green, icons::CHECK)
    } else {
        (theme.yellow, icons::VOL)
    };
    let dot = pos2(left + 16.0, top + head_h / 2.0);
    ui.painter()
        .circle_filled(dot, 16.0, alpha(dot_color, 0.18));
    paint_icon(ui, dot_icon, dot, 16.0, dot_color);
    let tx = left + 32.0 + 10.0;
    let ty = top + (head_h - title_g.size().y - summary_g.size().y) / 2.0;
    let th = title_g.size().y;
    ui.painter().galley(pos2(tx, ty), title_g, theme.text);
    ui.painter()
        .galley(pos2(tx, ty + th), summary_g, theme.muted());
    let close = Rect::from_min_size(
        pos2(rect.right() - PAD - 32.0, top + head_h / 2.0 - 16.0),
        vec2(32.0, 32.0),
    );
    let mut close_ui = ui.new_child(egui::UiBuilder::new().max_rect(close));
    let mut closed = icon_button(
        &mut close_ui,
        &theme,
        icons::X,
        32.0,
        16.0,
        theme.subtext0,
        true,
    )
    .on_hover_text("Close")
    .clicked();

    // The path: discs joined by a line, a step each.
    let mut y = top + head_h + 14.0;
    let first_center = pos2(left + 12.0, y + 7.0 + 12.0);
    let mut last_center = first_center;
    let mut centers = Vec::new();
    for ((title, lines), h) in step_g.into_iter().zip(&step_h) {
        let center = pos2(left + 12.0, y + 7.0 + 12.0);
        centers.push(center);
        last_center = center;
        let mut ly = y + 7.0;
        let tx = left + 24.0 + 12.0;
        let th = title.size().y;
        ui.painter().galley(pos2(tx, ly), title, theme.text);
        ly += th;
        for line in lines {
            let lh = line.size().y;
            ui.painter().galley(pos2(tx, ly), line, theme.muted());
            ly += lh;
        }
        y += h;
    }
    ui.painter().vline(
        left + 12.0,
        first_center.y..=last_center.y,
        Stroke::new(1.5, theme.line()),
    );
    for (step, center) in steps.iter().zip(centers) {
        ui.painter().circle_filled(center, 12.0, theme.surface1);
        let color = match step.tone {
            Tone::Neutral => theme.subtext0,
            Tone::Ok => theme.green,
            Tone::Warn => theme.yellow,
        };
        paint_icon(ui, step.icon, center, 13.0, color);
    }

    // Foot.
    let foot_top = y + 12.0;
    ui.painter().hline(
        rect.left() + PAD..=rect.right() - PAD,
        foot_top,
        Stroke::new(1.0, theme.line()),
    );
    ui.painter()
        .galley(pos2(left, foot_top + 12.0), foot_g, theme.muted());

    // A click anywhere else closes it (Escape is handled with the other keys).
    let outside_click = ui.input(|i| {
        i.pointer.any_pressed()
            && i.pointer
                .interact_pos()
                .is_some_and(|p| !rect.contains(p) && !anchor_rect.contains(p))
    });
    if outside_click {
        closed = true;
    }
    if closed {
        app.signal_open = None;
    }
}

/// Remember where an anchor was drawn, so the popover can hang from it.
pub fn remember_anchor(ui: &Ui, anchor: SignalAnchor, rect: Rect) {
    ui.ctx().data_mut(|d| d.insert_temp(anchor.id(), rect));
}
