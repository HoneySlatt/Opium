//! M1.14's instruments, the one change the frozen window allows besides its
//! fixes ([ADR-0031](../../../docs/adr/0031-opium-moves-to-uwui-during-slimeengines-stage-6d.md)),
//! removed with egui: a record of every frame, and a script that scrolls or
//! animates the window on its own, so that uwui can be held to the same
//! figures.
//!
//! `OPIUM_MEASURE=<scenario>` turns them on and `OPIUM_MEASURE_OUT=<file>`
//! names the record, written once the scenario ends, and the window then
//! closes. Every scenario opens its page as soon as the library's songs are
//! in, Songs but for `scroll-albums`; that frame is the first useful one, and
//! [`SETTLE`] at rest follows. Then:
//!
//! - `start`: nothing more.
//! - `scroll-songs`, `scroll-albums`: Songs or the Albums grid scrolled down
//!   at [`SCROLL_SPEED`] to the end.
//! - `animate`: the window's transitions, [`STEP`] apart: four rounds of
//!   Albums, Artists, Home and Songs; the queue drawer opened and closed three
//!   times; Now Playing opened and closed three times.
//!
//! The window asks to be full screen at its first frame, so that what is
//! drawn is the display's whole size wherever it opens.
//!
//! The scroll moves the page by the same distance every second whatever the
//! frame rate, as fingers on a trackpad do, without the rubber band at the
//! end, so that the end shows. It is injected into egui's own input: nothing
//! reaches the compositor. The record is plain text: `#` lines say what was
//! measured, then one line a frame (`start_us,cpu_us,phase,continued,scrolled`):
//!
//! - `start_us`, when the frame began, in microseconds of the Unix epoch, so
//!   that a script compares it with the time it started the program;
//! - `cpu_us`, eframe's time for the frame, its interface, tessellation and
//!   drawing calls but not the wait for the display (`IntegrationInfo::cpu_usage`);
//!   empty for the last frame, which eframe reports only on the next one;
//! - `phase`, the part of the scenario the frame belongs to;
//! - `continued`, 1 if the frame before asked for this one at once, as a
//!   moving frame does: its distance from the frame before then says whether
//!   the display's pace was held. 0 after a rest, or a wait the window chose;
//! - `scrolled`, how far the page had scrolled, in points.

use std::fmt::Write as _;
use std::path::PathBuf;
use std::time::{Duration, SystemTime};

use egui::{Event, MouseWheelSource, MouseWheelUnit, TouchPhase, vec2};

use crate::app::Page;

/// The time at rest after the first useful frame, so that the first screen's
/// covers come in before a scenario moves anything.
pub const SETTLE: f64 = 3.0;
/// How fast the scroll goes, in points a second: a quick sweep of a long
/// list, 77 of Songs' rows a second.
pub const SCROLL_SPEED: f32 = 4000.0;
/// The time between two of `animate`'s steps: longer than the slowest
/// transition (0.38 s), so that each one ends at rest.
pub const STEP: f64 = 1.0;
/// The longest a scroll goes on, should its end never come.
const SCROLL_LIMIT: f64 = 300.0;
/// Frames in a row whose scroll no page took before it counts as the end.
const ENDED_AFTER: u32 = 10;

#[derive(Clone, Copy, PartialEq)]
enum Scenario {
    Start,
    ScrollSongs,
    ScrollAlbums,
    Animate,
}

impl Scenario {
    fn named(name: &str) -> Option<Self> {
        match name {
            "start" => Some(Self::Start),
            "scroll-songs" => Some(Self::ScrollSongs),
            "scroll-albums" => Some(Self::ScrollAlbums),
            "animate" => Some(Self::Animate),
            _ => None,
        }
    }

    fn name(self) -> &'static str {
        match self {
            Self::Start => "start",
            Self::ScrollSongs => "scroll-songs",
            Self::ScrollAlbums => "scroll-albums",
            Self::Animate => "animate",
        }
    }
}

/// What the scenario asks of the window this frame.
pub enum Action {
    Open(Page),
    ToggleDrawer,
    ToggleNowPlaying,
}

/// `animate`'s steps, in order, each with the phase its frames are put in.
const ANIMATION: [(&str, Step); 28] = {
    use Step::*;
    [
        ("pages", Albums),
        ("pages", Artists),
        ("pages", Home),
        ("pages", Songs),
        ("pages", Albums),
        ("pages", Artists),
        ("pages", Home),
        ("pages", Songs),
        ("pages", Albums),
        ("pages", Artists),
        ("pages", Home),
        ("pages", Songs),
        ("pages", Albums),
        ("pages", Artists),
        ("pages", Home),
        ("pages", Songs),
        ("drawer", Drawer),
        ("drawer", Drawer),
        ("drawer", Drawer),
        ("drawer", Drawer),
        ("drawer", Drawer),
        ("drawer", Drawer),
        ("now-playing", NowPlaying),
        ("now-playing", NowPlaying),
        ("now-playing", NowPlaying),
        ("now-playing", NowPlaying),
        ("now-playing", NowPlaying),
        ("now-playing", NowPlaying),
    ]
};

#[derive(Clone, Copy)]
enum Step {
    Albums,
    Artists,
    Home,
    Songs,
    Drawer,
    NowPlaying,
}

/// Where the scenario stands.
#[derive(Clone, Copy, PartialEq)]
enum Stage {
    /// Before the library's songs are in.
    Loading,
    /// At rest since the first useful frame, at the time given.
    Settling(f64),
    /// Scrolling since the time given; `None` until the first frame's touch.
    Scrolling(Option<f64>),
    /// At `animate`'s step given, taken at the time given.
    Animating(usize, f64),
    /// The record written, the window asked to close.
    Done,
}

struct Frame {
    start_us: u64,
    cpu_us: Option<u64>,
    phase: &'static str,
    continued: bool,
    scrolled: f32,
}

pub struct Measure {
    scenario: Scenario,
    out: PathBuf,
    frames: Vec<Frame>,
    stage: Stage,
    /// When the frame before began, in egui's time, for the scroll's step.
    last_time: Option<f64>,
    scrolled: f32,
    /// The scroll handed to egui this frame, in points.
    pushed: f32,
    /// Frames in a row whose scroll no page took.
    untaken: u32,
    /// Said in the record: the songs, the scale, the window's size.
    facts: String,
    /// Full screen asked for.
    full_screen: bool,
}

impl Measure {
    /// The instruments, if `OPIUM_MEASURE` asks for them. A scenario it
    /// doesn't know, or no `OPIUM_MEASURE_OUT`, is said on standard error and
    /// leaves them off.
    pub fn from_env() -> Option<Self> {
        let name = std::env::var("OPIUM_MEASURE").ok()?;
        let Some(scenario) = Scenario::named(&name) else {
            eprintln!(
                "OPIUM_MEASURE: no scenario named {name:?} (start, scroll-songs, scroll-albums, animate)"
            );
            return None;
        };
        let Some(out) = std::env::var_os("OPIUM_MEASURE_OUT") else {
            eprintln!("OPIUM_MEASURE: OPIUM_MEASURE_OUT must name the record's file");
            return None;
        };
        Some(Self {
            out: out.into(),
            ..Self::new(scenario)
        })
    }

    fn new(scenario: Scenario) -> Self {
        Self {
            scenario,
            out: PathBuf::new(),
            // A minute and more of frames at 240 Hz before it grows.
            frames: Vec::with_capacity(1 << 15),
            stage: Stage::Loading,
            last_time: None,
            scrolled: 0.0,
            pushed: 0.0,
            untaken: 0,
            facts: String::new(),
            full_screen: false,
        }
    }

    /// At the start of a frame, before egui reads its input: the frame is
    /// counted, and while scrolling, this frame's share of the scroll joins
    /// the input, with the pointer over the page's middle.
    pub fn frame_begins(&mut self, input: &mut egui::RawInput) {
        if self.stage == Stage::Done {
            return;
        }
        let start_us = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map_or(0, |d| d.as_micros() as u64);
        let time = input.time.unwrap_or(0.0);
        let dt = self.last_time.map_or(0.0, |last| (time - last) as f32);
        self.last_time = Some(time);
        self.pushed = 0.0;
        if let Stage::Scrolling(began) = self.stage {
            let delta = if began.is_none() {
                self.stage = Stage::Scrolling(Some(time));
                if let Some(rect) = input.screen_rect {
                    input.events.push(Event::PointerMoved(rect.center()));
                }
                (TouchPhase::Start, 0.0)
            } else {
                (TouchPhase::Move, SCROLL_SPEED * dt)
            };
            self.pushed = delta.1;
            input.events.push(Event::MouseWheel {
                unit: MouseWheelUnit::Point,
                // Down the page: the content moves up.
                delta: vec2(0.0, -delta.1),
                phase: delta.0,
                // Not `Trackpad`: its rubber band would take the scroll
                // past the end, which then never shows.
                source: MouseWheelSource::Unknown,
                modifiers: egui::Modifiers::NONE,
            });
        }
        self.frames.push(Frame {
            start_us,
            cpu_us: None,
            phase: "",
            continued: false,
            scrolled: self.scrolled,
        });
    }

    /// Before the window is drawn: eframe's time for the frame before, and
    /// what the scenario asks of this one.
    pub fn before_drawing(
        &mut self,
        ctx: &egui::Context,
        cpu_of_last_frame: Option<f32>,
        songs: usize,
    ) -> Option<Action> {
        if self.stage == Stage::Done {
            return None;
        }
        if !std::mem::replace(&mut self.full_screen, true) {
            ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(true));
        }
        if let (Some(cpu), [.., before, _]) = (cpu_of_last_frame, self.frames.as_mut_slice()) {
            before.cpu_us = Some((cpu as f64 * 1e6) as u64);
        }
        if let Some(frame) = self.frames.last_mut() {
            frame.continued = ctx.requested_repaint_last_pass();
        }
        let now = ctx.input(|i| i.time);
        let (phase, action) = match self.stage {
            Stage::Loading if songs > 0 => {
                self.stage = Stage::Settling(now);
                self.facts = facts(ctx, songs);
                let page = if self.scenario == Scenario::ScrollAlbums {
                    Page::Albums
                } else {
                    Page::Songs
                };
                ("useful", Some(Action::Open(page)))
            }
            Stage::Loading => ("loading", None),
            Stage::Settling(since) if now - since < SETTLE => {
                ctx.request_repaint_after(Duration::from_secs_f64(SETTLE - (now - since)));
                ("settle", None)
            }
            Stage::Settling(_) => match self.scenario {
                Scenario::Start => {
                    self.end(ctx);
                    ("end", None)
                }
                Scenario::ScrollSongs | Scenario::ScrollAlbums => {
                    self.stage = Stage::Scrolling(None);
                    ctx.request_repaint();
                    ("scroll", None)
                }
                Scenario::Animate => self.step(ctx, 0, now),
            },
            Stage::Scrolling(_) => {
                ctx.request_repaint();
                ("scroll", None)
            }
            Stage::Animating(step, since) if now - since < STEP => {
                ctx.request_repaint_after(Duration::from_secs_f64(STEP - (now - since)));
                (ANIMATION[step].0, None)
            }
            Stage::Animating(step, _) => self.step(ctx, step + 1, now),
            Stage::Done => return None,
        };
        if let Some(frame) = self.frames.last_mut() {
            frame.phase = phase;
        }
        action
    }

    /// After the window is drawn: how far the scroll went, and whether it
    /// reached the end.
    pub fn after_drawing(&mut self, ctx: &egui::Context) {
        let Stage::Scrolling(Some(began)) = self.stage else {
            return;
        };
        // A page that scrolls takes the scroll out of the input.
        let untaken = ctx.input(|i| i.smooth_scroll_delta().y) != 0.0;
        if self.pushed > 0.0 {
            if untaken {
                self.untaken += 1;
            } else {
                self.untaken = 0;
                self.scrolled += self.pushed;
            }
        }
        let now = ctx.input(|i| i.time);
        if self.untaken >= ENDED_AFTER || now - began > SCROLL_LIMIT {
            self.end(ctx);
        }
    }

    fn step(
        &mut self,
        ctx: &egui::Context,
        step: usize,
        now: f64,
    ) -> (&'static str, Option<Action>) {
        let Some(&(phase, what)) = ANIMATION.get(step) else {
            self.end(ctx);
            return ("end", None);
        };
        self.stage = Stage::Animating(step, now);
        ctx.request_repaint_after(Duration::from_secs_f64(STEP));
        let action = match what {
            Step::Albums => Action::Open(Page::Albums),
            Step::Artists => Action::Open(Page::Artists),
            Step::Home => Action::Open(Page::Home),
            Step::Songs => Action::Open(Page::Songs),
            Step::Drawer => Action::ToggleDrawer,
            Step::NowPlaying => Action::ToggleNowPlaying,
        };
        (phase, Some(action))
    }

    /// The record written, beside then renamed over, and the window closed.
    fn end(&mut self, ctx: &egui::Context) {
        self.stage = Stage::Done;
        if let Some(frame) = self.frames.last_mut() {
            frame.phase = "end";
        }
        let mut text = format!(
            "# opium frame record\n# scenario={}\n# scroll_speed={SCROLL_SPEED}\n# settle={SETTLE}\n# step={STEP}\n{}# scrolled={}\nstart_us,cpu_us,phase,continued,scrolled\n",
            self.scenario.name(),
            self.facts,
            self.scrolled,
        );
        for frame in &self.frames {
            let cpu = frame.cpu_us.map(|c| c.to_string()).unwrap_or_default();
            let _ = writeln!(
                text,
                "{},{cpu},{},{},{}",
                frame.start_us,
                frame.phase,
                u8::from(frame.continued),
                frame.scrolled
            );
        }
        let partial = self.out.with_extension("part");
        match std::fs::write(&partial, text).and_then(|()| std::fs::rename(&partial, &self.out)) {
            Ok(()) => eprintln!("frame record written to {}", self.out.display()),
            Err(e) => eprintln!("frame record not written to {}: {e}", self.out.display()),
        }
        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        ctx.request_repaint();
    }
}

/// The record's `#` lines on what was drawn: the songs, the scale, the
/// window's size in points and in pixels.
fn facts(ctx: &egui::Context, songs: usize) -> String {
    let ppp = ctx.pixels_per_point();
    let rect = ctx.content_rect();
    format!(
        "# songs={songs}\n# pixels_per_point={ppp}\n# window_points={}x{}\n# window_pixels={}x{}\n",
        rect.width().round(),
        rect.height().round(),
        (rect.width() * ppp).round(),
        (rect.height() * ppp).round(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scenarios_read_back_by_name() {
        for scenario in [
            Scenario::Start,
            Scenario::ScrollSongs,
            Scenario::ScrollAlbums,
            Scenario::Animate,
        ] {
            assert!(Scenario::named(scenario.name()) == Some(scenario));
        }
        assert!(Scenario::named("scroll").is_none());
    }

    /// One frame at egui's time `time`, with `songs` in the library: what
    /// the scenario asked of it.
    fn frame(
        measure: &mut Measure,
        ctx: &egui::Context,
        time: f64,
        songs: usize,
    ) -> Option<Action> {
        let mut input = egui::RawInput {
            time: Some(time),
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                vec2(800.0, 600.0),
            )),
            ..Default::default()
        };
        measure.frame_begins(&mut input);
        let mut action = None;
        let mut output = ctx.run_ui(input, |ui| {
            action = measure.before_drawing(ui.ctx(), Some(0.002), songs);
            measure.after_drawing(ui.ctx());
        });
        // No painter here takes the font atlas.
        output.textures_delta.clear();
        action
    }

    #[test]
    fn start_records_every_frame_and_marks_the_first_useful_one() {
        let dir = std::env::temp_dir().join(format!("opium-measure-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let out = dir.join("frames.csv");
        let mut measure = Measure {
            out: out.clone(),
            ..Measure::new(Scenario::Start)
        };
        let ctx = egui::Context::default();
        assert!(frame(&mut measure, &ctx, 0.0, 0).is_none());
        assert!(matches!(
            frame(&mut measure, &ctx, 0.1, 12),
            Some(Action::Open(Page::Songs))
        ));
        frame(&mut measure, &ctx, 0.2, 12);
        frame(&mut measure, &ctx, 0.1 + SETTLE, 12);
        assert!(measure.stage == Stage::Done);
        let text = std::fs::read_to_string(&out).unwrap();
        std::fs::remove_dir_all(&dir).unwrap();
        assert!(text.contains("# scenario=start\n"));
        assert!(text.contains("# songs=12\n"));
        let phases: Vec<&str> = text
            .lines()
            .filter(|l| !l.starts_with('#'))
            .skip(1)
            .map(|l| l.split(',').nth(2).unwrap())
            .collect();
        assert_eq!(phases, ["loading", "useful", "settle", "end"]);
        // Each frame's time comes on the next; the last has none.
        let cpus: Vec<&str> = text
            .lines()
            .filter(|l| !l.starts_with('#'))
            .skip(1)
            .map(|l| l.split(',').nth(1).unwrap())
            .collect();
        assert_eq!(cpus, ["2000", "2000", "2000", ""]);
    }

    #[test]
    fn a_scroll_injects_a_steady_wheel_and_ends_when_nothing_takes_it() {
        let mut measure = Measure {
            stage: Stage::Scrolling(None),
            ..Measure::new(Scenario::ScrollSongs)
        };
        let mut input = egui::RawInput {
            time: Some(10.0),
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                vec2(800.0, 600.0),
            )),
            ..Default::default()
        };
        measure.frame_begins(&mut input);
        assert!(matches!(input.events[0], Event::PointerMoved(p) if p == egui::pos2(400.0, 300.0)));
        assert!(matches!(
            input.events[1],
            Event::MouseWheel {
                phase: TouchPhase::Start,
                ..
            }
        ));
        let mut input = egui::RawInput {
            time: Some(10.25),
            ..Default::default()
        };
        measure.frame_begins(&mut input);
        let Event::MouseWheel { delta, phase, .. } = input.events[0] else {
            panic!("no wheel");
        };
        assert!(phase == TouchPhase::Move);
        assert_eq!(delta.y, -SCROLL_SPEED * 0.25);
        // No scroll area here takes it: the end, after a few frames.
        let ctx = egui::Context::default();
        let dir = std::env::temp_dir().join(format!("opium-measure-scroll-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        measure.out = dir.join("frames.csv");
        for i in 0..ENDED_AFTER {
            frame(&mut measure, &ctx, 10.5 + f64::from(i) * 0.01, 12);
        }
        assert!(measure.stage == Stage::Done);
        assert_eq!(measure.scrolled, 0.0);
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
