//! What plays, told to the desktop: MPRIS on the session bus, which media
//! keys, `playerctl` and bars' player widgets talk to. Their calls come back
//! as [`Action`]s for the app to carry out; what they read is the [`State`]
//! the app gave last.

use std::sync::mpsc::{Receiver, Sender};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant};

use mpris_server::zbus::fdo::{RequestNameFlags, RequestNameReply};
use mpris_server::zbus::names::WellKnownName;
use mpris_server::zbus::{Connection, Result, fdo};
use mpris_server::{
    LoopStatus, Metadata, PlaybackRate, PlaybackStatus, PlayerInterface, Property, RootInterface,
    Server, Signal, Time, TrackId, Volume,
};
use opium_core::Repeat;
use opium_core::model::{ProviderKind, Track};

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
    /// A music file to play, or a folder of them, opened from elsewhere.
    Open(std::path::PathBuf),
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
    /// Set once the name is held and kept ([`Mpris::on_the_bus`]).
    on_the_bus: Arc<std::sync::atomic::AtomicBool>,
}

/// The audio files opened here, as the desktop entry says too.
const MIME_TYPES: [&str; 14] = [
    "audio/flac",
    "audio/x-flac",
    "audio/mpeg",
    "audio/mp3",
    "audio/wav",
    "audio/x-wav",
    "audio/aiff",
    "audio/x-aiff",
    "audio/mp4",
    "audio/x-m4a",
    "audio/aac",
    "audio/ogg",
    "audio/vorbis",
    "audio/x-vorbis+ogg",
];

/// The name Opium takes on the bus, and looks for an Opium already running
/// under.
const BUS_NAME: &str = "org.mpris.MediaPlayer2.opium";

/// The files the command line names (paths, or `file://` addresses).
pub fn files_from_args() -> Vec<std::path::PathBuf> {
    std::env::args_os()
        .skip(1)
        .filter_map(|arg| match arg.to_str() {
            Some(uri) if uri.starts_with("file://") => crate::file_path(uri),
            _ => std::path::absolute(arg).ok(),
        })
        .collect()
}

/// Whether the player on the bus under Opium's name is Opium itself: any
/// program may take that name first, and a stranger's player mustn't be
/// handed this one's files, nor stand for it and keep it from starting.
async fn its_opium(bus: &mpris_server::zbus::Connection) -> bool {
    let said = |property: &'static str| {
        let bus = bus.clone();
        async move {
            let reply = bus
                .call_method(
                    Some(BUS_NAME),
                    "/org/mpris/MediaPlayer2",
                    Some("org.freedesktop.DBus.Properties"),
                    "Get",
                    &("org.mpris.MediaPlayer2", property),
                )
                .await
                .ok()?;
            reply
                .body()
                .deserialize::<mpris_server::zbus::Variant>()
                .ok()?
                .downcast_ref::<String>()
                .ok()
        }
    };
    said("DesktopEntry").await.as_deref() == Some("opium")
        && said("Identity").await.as_deref() == Some("Opium")
}

/// Bring the Opium already running forward; whether one was there. Two of
/// them would share the settings, the downloads and the library kept on
/// disk, each undoing the other's writes.
pub fn raise_running() -> bool {
    let Ok(runtime) = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    else {
        return false;
    };
    runtime.block_on(async {
        let Ok(bus) = mpris_server::zbus::Connection::session().await else {
            return false;
        };
        if !its_opium(&bus).await {
            return false;
        }
        bus.call_method(
            Some(BUS_NAME),
            "/org/mpris/MediaPlayer2",
            Some("org.mpris.MediaPlayer2"),
            "Raise",
            &(),
        )
        .await
        .is_ok()
    })
}

/// Hand `files` to the Opium already running, through its media controls,
/// and bring it forward; whether one was there to take them.
pub fn open_in_running(files: &[std::path::PathBuf]) -> bool {
    let Ok(runtime) = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    else {
        return false;
    };
    runtime.block_on(async {
        let Ok(bus) = mpris_server::zbus::Connection::session().await else {
            return false;
        };
        if !its_opium(&bus).await {
            return false;
        }
        let call = |interface: &'static str, method: &'static str, uri: Option<String>| {
            let bus = bus.clone();
            async move {
                let path = "/org/mpris/MediaPlayer2";
                let to = Some(BUS_NAME);
                match uri {
                    Some(uri) => {
                        bus.call_method(to, path, Some(interface), method, &(uri,))
                            .await
                    }
                    None => {
                        bus.call_method(to, path, Some(interface), method, &())
                            .await
                    }
                }
                .is_ok()
            }
        };
        for file in files {
            if !call(
                "org.mpris.MediaPlayer2.Player",
                "OpenUri",
                Some(crate::file_uri(file)),
            )
            .await
            {
                return false;
            }
        }
        call("org.mpris.MediaPlayer2", "Raise", None).await;
        true
    })
}

/// Ask again for Opium's name, on the connection that holds it, so that no
/// later request takes it: another Opium, on another profile, would
/// otherwise take this one's media controls, which nothing would tell.
///
/// mpris-server asks for the name as zbus's connection builder does, with
/// `RequestNameFlags::default()`: `ALLOW_REPLACEMENT | REPLACE_EXISTING |
/// DO_NOT_QUEUE` (zbus 6.2, `connection/builder.rs` and `fdo/dbus.rs`).
/// Asked again by its owner, the bus sets the name's flags anew (dbus-daemon,
/// `bus/services.c`, `bus_registry_acquire_service`; dbus-broker,
/// `src/bus/name.c`, `name_ownership_update`); with `DO_NOT_QUEUE` alone, a
/// later request is refused. A request in the moment between the two still
/// takes it: this one then finds it gone, and fails.
async fn keep_the_name(bus: &Connection) -> Result<()> {
    let reply = fdo::DBusProxy::new(bus)
        .await?
        .request_name(
            WellKnownName::try_from(BUS_NAME)?,
            RequestNameFlags::DO_NOT_QUEUE,
        )
        .await?;
    match reply {
        // Held still, or again: either way now with these flags alone.
        RequestNameReply::AlreadyOwner | RequestNameReply::PrimaryOwner => Ok(()),
        RequestNameReply::InQueue | RequestNameReply::Exists => {
            Err(mpris_server::zbus::Error::NameTaken)
        }
    }
}

impl Mpris {
    /// Take the name `org.mpris.MediaPlayer2.opium` on a thread of its own,
    /// so that no later request takes it (`keep_the_name`); `wake` is
    /// called when an action comes. Without a session bus, or with the name
    /// held by another program, nothing happens but a line on standard
    /// error, and the app plays on.
    pub fn start(wake: crate::Wake) -> Self {
        let shared = Arc::new(Mutex::new(Shared {
            state: State::default(),
            at: Instant::now(),
        }));
        let (actions_tx, actions) = std::sync::mpsc::channel();
        let on_the_bus = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let on_the_bus_now = on_the_bus.clone();
        let (changes, mut changes_rx) = tokio::sync::mpsc::unbounded_channel::<Change>();
        let imp = Imp {
            shared: shared.clone(),
            actions: actions_tx,
            wake,
        };
        let _ = std::thread::Builder::new()
            .name("mpris".into())
            .spawn(move || {
                let Ok(runtime) = tokio::runtime::Builder::new_current_thread().build() else {
                    return;
                };
                runtime.block_on(async move {
                    let server = match Server::new("opium", imp).await {
                        Ok(server) => server,
                        Err(e) => {
                            eprintln!("media controls unavailable: {e}");
                            return;
                        }
                    };
                    if let Err(e) = keep_the_name(server.connection()).await {
                        eprintln!("media controls unavailable: {e}");
                        return;
                    }
                    on_the_bus_now.store(true, std::sync::atomic::Ordering::Release);
                    while let Some(change) = changes_rx.recv().await {
                        if let Some(url) = &change.cover
                            && crate::covers::read(url).is_none()
                            && let Ok(resp) = ehttp::fetch_blocking(&ehttp::Request::get(
                                crate::covers::signed(url),
                            ))
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
            on_the_bus,
        }
    }

    /// Whether the media controls are on the session bus, their name held and
    /// kept from later requests: not yet while they start, and never when
    /// they are unavailable.
    pub fn on_the_bus(&self) -> bool {
        self.on_the_bus.load(std::sync::atomic::Ordering::Acquire)
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
    TrackId::try_from(format!("/opium/track/{hash:016x}")).unwrap_or(TrackId::NO_TRACK)
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
    wake: crate::Wake,
}

impl Imp {
    fn act(&self, action: Action) {
        let _ = self.actions.send(action);
        (self.wake)();
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
        Ok("Opium".into())
    }

    async fn desktop_entry(&self) -> fdo::Result<String> {
        Ok("opium".into())
    }

    async fn supported_uri_schemes(&self) -> fdo::Result<Vec<String>> {
        Ok(vec!["file".into()])
    }

    async fn supported_mime_types(&self) -> fdo::Result<Vec<String>> {
        Ok(MIME_TYPES.iter().map(|&t| t.to_owned()).collect())
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

    async fn open_uri(&self, uri: String) -> fdo::Result<()> {
        let path = crate::file_path(&uri)
            .ok_or_else(|| fdo::Error::NotSupported("only files are opened".into()))?;
        self.act(Action::Open(path));
        Ok(())
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
