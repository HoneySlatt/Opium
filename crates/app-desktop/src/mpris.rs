//! What plays, told to the desktop: MPRIS on the session bus, which media
//! keys, `playerctl` and bars' player widgets talk to. Their calls come back
//! as [`Action`]s for the app to carry out; what they read is the [`State`]
//! the app gave last.

use std::sync::mpsc::{Receiver, Sender};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant};

use mpris_server::zbus::{Result, fdo};
use mpris_server::{
    LoopStatus, Metadata, PlaybackRate, PlaybackStatus, PlayerInterface, Property, RootInterface,
    Server, Signal, Time, TrackId, Volume,
};
use music_core::Repeat;
use music_core::model::{ProviderKind, Track};

/// What a controller asked for.
#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    PlayPause,
    Play,
    Pause,
    Stop,
    Next,
    Previous,
    SeekTo(Duration),
    Volume(f32),
    Shuffle(bool),
    Repeat(Repeat),
    Raise,
    Quit,
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum Playback {
    #[default]
    Stopped,
    Paused,
    Playing,
}

/// What controllers are shown.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct State {
    pub track: Option<Track>,
    /// `None` for a live stream, which can't seek.
    pub length: Option<Duration>,
    pub playback: Playback,
    pub position: Duration,
    pub volume: f32,
    pub shuffle: bool,
    pub repeat: Repeat,
}

/// The state, and when its position was read: it moves on from there while
/// playing.
struct Shared {
    state: State,
    at: Instant,
}

impl Shared {
    fn position(&self, now: Instant) -> Duration {
        let mut position = self.state.position;
        if self.state.playback == Playback::Playing {
            position += now.saturating_duration_since(self.at);
        }
        self.state.length.map_or(position, |l| position.min(l))
    }
}

fn lock(shared: &Mutex<Shared>) -> MutexGuard<'_, Shared> {
    shared.lock().unwrap_or_else(|e| e.into_inner())
}

/// Properties that changed, and where a jump landed.
struct Change {
    changed: Vec<Property>,
    seeked: Option<Duration>,
    /// A cover to have kept on disk before the metadata naming its file goes
    /// out.
    cover: Option<String>,
}

/// The app's side: hands the state over, takes the actions.
pub struct Mpris {
    shared: Arc<Mutex<Shared>>,
    changes: tokio::sync::mpsc::UnboundedSender<Change>,
    actions: Receiver<Action>,
}

impl Mpris {
    /// Take the name `org.mpris.MediaPlayer2.music` on a thread of its own;
    /// `ctx` is woken when an action comes. Without a session bus nothing
    /// happens, and the app plays on.
    pub fn start(ctx: egui::Context) -> Self {
        let shared = Arc::new(Mutex::new(Shared {
            state: State::default(),
            at: Instant::now(),
        }));
        let (actions_tx, actions) = std::sync::mpsc::channel();
        let (changes, mut changes_rx) = tokio::sync::mpsc::unbounded_channel::<Change>();
        let imp = Imp {
            shared: shared.clone(),
            actions: actions_tx,
            ctx,
        };
        let _ = std::thread::Builder::new()
            .name("mpris".into())
            .spawn(move || {
                let Ok(runtime) = tokio::runtime::Builder::new_current_thread().build() else {
                    return;
                };
                runtime.block_on(async move {
                    let server = match Server::new("music", imp).await {
                        Ok(server) => server,
                        Err(e) => {
                            eprintln!("media controls unavailable: {e}");
                            return;
                        }
                    };
                    while let Some(change) = changes_rx.recv().await {
                        if let Some(url) = &change.cover
                            && crate::covers::read(url).is_none()
                            && let Ok(resp) = ehttp::fetch_blocking(&ehttp::Request::get(url))
                            && resp.ok
                        {
                            crate::covers::write(url, &resp.bytes);
                        }
                        if !change.changed.is_empty() {
                            let _ = server.properties_changed(change.changed).await;
                        }
                        if let Some(position) = change.seeked {
                            let _ = server
                                .emit(Signal::Seeked {
                                    position: time(position),
                                })
                                .await;
                        }
                    }
                });
            });
        Self {
            shared,
            changes,
            actions,
        }
    }

    pub fn next_action(&self) -> Option<Action> {
        self.actions.try_recv().ok()
    }

    /// Show `state`, telling controllers what changed: a jump in the
    /// position counts when it is more than playing on would explain.
    pub fn update(&self, state: State) {
        let now = Instant::now();
        let mut changed = Vec::new();
        let mut seeked = None;
        let mut cover = None;
        {
            let mut shared = lock(&self.shared);
            let old = &shared.state;
            if old.track != state.track || old.length != state.length {
                cover = state.track.as_ref().and_then(private_cover);
                changed.push(Property::Metadata(metadata(&state)));
                changed.push(Property::CanSeek(state.length.is_some()));
            } else if state.track.is_some()
                && shared.position(now).abs_diff(state.position) > Duration::from_secs(1)
            {
                seeked = Some(state.position);
            }
            if old.playback != state.playback {
                changed.push(Property::PlaybackStatus(playback(state.playback)));
            }
            if old.volume != state.volume {
                changed.push(Property::Volume(f64::from(state.volume)));
            }
            if old.shuffle != state.shuffle {
                changed.push(Property::Shuffle(state.shuffle));
            }
            if old.repeat != state.repeat {
                changed.push(Property::LoopStatus(loop_status(state.repeat)));
            }
            shared.state = state;
            shared.at = now;
        }
        if !changed.is_empty() || seeked.is_some() {
            let _ = self.changes.send(Change {
                changed,
                seeked,
                cover,
            });
        }
    }
}

fn time(d: Duration) -> Time {
    Time::from_micros(i64::try_from(d.as_micros()).unwrap_or(i64::MAX))
}

fn playback(p: Playback) -> PlaybackStatus {
    match p {
        Playback::Stopped => PlaybackStatus::Stopped,
        Playback::Paused => PlaybackStatus::Paused,
        Playback::Playing => PlaybackStatus::Playing,
    }
}

fn loop_status(r: Repeat) -> LoopStatus {
    match r {
        Repeat::Off => LoopStatus::None,
        Repeat::All => LoopStatus::Playlist,
        Repeat::One => LoopStatus::Track,
    }
}

/// A song's id on the bus: the same each time it plays.
fn track_id(track: &Track) -> TrackId {
    let hash = track.id.id.bytes().fold(0xcbf2_9ce4_8422_2325_u64, |h, b| {
        (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3)
    });
    TrackId::try_from(format!("/music/track/{hash:016x}")).unwrap_or(TrackId::NO_TRACK)
}

fn metadata(state: &State) -> Metadata {
    let Some(track) = &state.track else {
        return Metadata::builder().trackid(TrackId::NO_TRACK).build();
    };
    let mut m = Metadata::builder()
        .trackid(track_id(track))
        .title(&track.title)
        .artist(track.artists.clone());
    if let Some(album) = &track.album {
        m = m.album(album);
    }
    if let Some(length) = state.length {
        m = m.length(time(length));
    }
    if let Some(url) = private_cover(track) {
        if let Some(file) = crate::covers::file(&url) {
            m = m.art_url(format!("file://{}", file.display()));
        }
    } else if let Some(art) = &track.artwork {
        m = m.art_url(art.url(ART_SIZE));
    }
    if let Some(n) = track.track_number {
        m = m.track_number(n as i32);
    }
    if let Some(n) = track.disc_number {
        m = m.disc_number(n as i32);
    }
    m.build()
}

const ART_SIZE: u32 = 600;

/// The cover of a Navidrome song: its URL carries the sign-in, which the bus
/// mustn't see, so its controllers get the file it is kept in.
fn private_cover(track: &Track) -> Option<String> {
    (track.id.provider == ProviderKind::Navidrome)
        .then(|| track.artwork.as_ref().map(|a| a.url(ART_SIZE)))
        .flatten()
}

/// The bus's side, answering from the state shared.
struct Imp {
    shared: Arc<Mutex<Shared>>,
    actions: Sender<Action>,
    ctx: egui::Context,
}

impl Imp {
    fn act(&self, action: Action) {
        let _ = self.actions.send(action);
        self.ctx.request_repaint();
    }

    fn state<R>(&self, f: impl FnOnce(&State) -> R) -> R {
        f(&lock(&self.shared).state)
    }
}

impl RootInterface for Imp {
    async fn raise(&self) -> fdo::Result<()> {
        self.act(Action::Raise);
        Ok(())
    }

    async fn quit(&self) -> fdo::Result<()> {
        self.act(Action::Quit);
        Ok(())
    }

    async fn can_quit(&self) -> fdo::Result<bool> {
        Ok(true)
    }

    async fn fullscreen(&self) -> fdo::Result<bool> {
        Ok(false)
    }

    async fn set_fullscreen(&self, _: bool) -> Result<()> {
        Ok(())
    }

    async fn can_set_fullscreen(&self) -> fdo::Result<bool> {
        Ok(false)
    }

    async fn can_raise(&self) -> fdo::Result<bool> {
        Ok(true)
    }

    async fn has_track_list(&self) -> fdo::Result<bool> {
        Ok(false)
    }

    async fn identity(&self) -> fdo::Result<String> {
        Ok("Music".into())
    }

    async fn desktop_entry(&self) -> fdo::Result<String> {
        Ok("music".into())
    }

    async fn supported_uri_schemes(&self) -> fdo::Result<Vec<String>> {
        Ok(Vec::new())
    }

    async fn supported_mime_types(&self) -> fdo::Result<Vec<String>> {
        Ok(Vec::new())
    }
}

impl PlayerInterface for Imp {
    async fn next(&self) -> fdo::Result<()> {
        self.act(Action::Next);
        Ok(())
    }

    async fn previous(&self) -> fdo::Result<()> {
        self.act(Action::Previous);
        Ok(())
    }

    async fn pause(&self) -> fdo::Result<()> {
        self.act(Action::Pause);
        Ok(())
    }

    async fn play_pause(&self) -> fdo::Result<()> {
        self.act(Action::PlayPause);
        Ok(())
    }

    async fn stop(&self) -> fdo::Result<()> {
        self.act(Action::Stop);
        Ok(())
    }

    async fn play(&self) -> fdo::Result<()> {
        self.act(Action::Play);
        Ok(())
    }

    /// Back before the start is the start; past the end is the next track.
    async fn seek(&self, offset: Time) -> fdo::Result<()> {
        let (position, length) = {
            let shared = lock(&self.shared);
            if shared.state.track.is_none() {
                return Ok(());
            }
            (shared.position(Instant::now()), shared.state.length)
        };
        let Some(length) = length else {
            return Ok(());
        };
        let to = time(position).saturating_add(offset);
        if to >= time(length) {
            self.act(Action::Next);
        } else {
            let micros = u64::try_from(to.as_micros()).unwrap_or(0);
            self.act(Action::SeekTo(Duration::from_micros(micros)));
        }
        Ok(())
    }

    /// Only within the track playing, as the one named.
    async fn set_position(&self, track_id: TrackId, position: Time) -> fdo::Result<()> {
        let length = self.state(|s| {
            s.track
                .as_ref()
                .filter(|t| self::track_id(t) == track_id)
                .and(s.length)
        });
        if let Some(length) = length
            && !position.is_negative()
            && position <= time(length)
        {
            let micros = u64::try_from(position.as_micros()).unwrap_or(0);
            self.act(Action::SeekTo(Duration::from_micros(micros)));
        }
        Ok(())
    }

    async fn open_uri(&self, _: String) -> fdo::Result<()> {
        Err(fdo::Error::NotSupported("no URIs are opened".into()))
    }

    async fn playback_status(&self) -> fdo::Result<PlaybackStatus> {
        Ok(self.state(|s| playback(s.playback)))
    }

    async fn loop_status(&self) -> fdo::Result<LoopStatus> {
        Ok(self.state(|s| loop_status(s.repeat)))
    }

    async fn set_loop_status(&self, loop_status: LoopStatus) -> Result<()> {
        self.act(Action::Repeat(match loop_status {
            LoopStatus::None => Repeat::Off,
            LoopStatus::Playlist => Repeat::All,
            LoopStatus::Track => Repeat::One,
        }));
        Ok(())
    }

    async fn rate(&self) -> fdo::Result<PlaybackRate> {
        Ok(1.0)
    }

    async fn set_rate(&self, _: PlaybackRate) -> Result<()> {
        Ok(())
    }

    async fn shuffle(&self) -> fdo::Result<bool> {
        Ok(self.state(|s| s.shuffle))
    }

    async fn set_shuffle(&self, shuffle: bool) -> Result<()> {
        self.act(Action::Shuffle(shuffle));
        Ok(())
    }

    async fn metadata(&self) -> fdo::Result<Metadata> {
        Ok(self.state(metadata))
    }

    async fn volume(&self) -> fdo::Result<Volume> {
        Ok(self.state(|s| f64::from(s.volume)))
    }

    async fn set_volume(&self, volume: Volume) -> Result<()> {
        self.act(Action::Volume(volume.clamp(0.0, 1.0) as f32));
        Ok(())
    }

    async fn position(&self) -> fdo::Result<Time> {
        Ok(time(lock(&self.shared).position(Instant::now())))
    }

    async fn minimum_rate(&self) -> fdo::Result<PlaybackRate> {
        Ok(1.0)
    }

    async fn maximum_rate(&self) -> fdo::Result<PlaybackRate> {
        Ok(1.0)
    }

    async fn can_go_next(&self) -> fdo::Result<bool> {
        Ok(true)
    }

    async fn can_go_previous(&self) -> fdo::Result<bool> {
        Ok(true)
    }

    async fn can_play(&self) -> fdo::Result<bool> {
        Ok(true)
    }

    async fn can_pause(&self) -> fdo::Result<bool> {
        Ok(true)
    }

    async fn can_seek(&self) -> fdo::Result<bool> {
        Ok(self.state(|s| s.length.is_some()))
    }

    async fn can_control(&self) -> fdo::Result<bool> {
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn shared(playback: Playback, length: Option<Duration>) -> Shared {
        Shared {
            state: State {
                playback,
                length,
                position: Duration::from_secs(10),
                ..State::default()
            },
            at: Instant::now(),
        }
    }

    #[test]
    fn the_position_moves_on_only_while_playing_and_stops_at_the_end() {
        let later = |s: &Shared| s.position(s.at + Duration::from_secs(5));
        let length = Some(Duration::from_secs(12));
        assert_eq!(
            later(&shared(Playback::Playing, None)),
            Duration::from_secs(15)
        );
        assert_eq!(
            later(&shared(Playback::Paused, None)),
            Duration::from_secs(10)
        );
        assert_eq!(
            later(&shared(Playback::Playing, length)),
            Duration::from_secs(12)
        );
    }
}
