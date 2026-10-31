package io.github.honeyslatt.music.playback

import android.content.Context
import android.net.Uri
import androidx.media3.common.AudioAttributes
import androidx.media3.common.Format
import androidx.media3.common.C
import androidx.media3.common.MediaItem
import androidx.media3.common.MediaMetadata
import androidx.media3.common.PlaybackException
import androidx.media3.common.Player
import androidx.media3.common.util.UnstableApi
import androidx.media3.exoplayer.DefaultLoadControl
import androidx.media3.exoplayer.DecoderReuseEvaluation
import androidx.media3.exoplayer.ExoPlayer
import androidx.media3.exoplayer.analytics.AnalyticsListener
import androidx.media3.exoplayer.audio.AudioSink
import androidx.media3.exoplayer.source.MediaSource
import android.os.SystemClock
import io.github.honeyslatt.music.core.MusicException
import io.github.honeyslatt.music.core.PlayQueue
import io.github.honeyslatt.music.core.QueueRepeat
import io.github.honeyslatt.music.core.connect.Command
import io.github.honeyslatt.music.core.connect.Event
import io.github.honeyslatt.music.core.connect.Handoff
import io.github.honeyslatt.music.core.connect.Peer
import io.github.honeyslatt.music.core.connect.Playing
import io.github.honeyslatt.music.core.model.AudioQuality
import io.github.honeyslatt.music.core.model.Codec
import io.github.honeyslatt.music.core.model.Repeat
import io.github.honeyslatt.music.core.model.ProviderKind
import io.github.honeyslatt.music.core.model.Track
import io.github.honeyslatt.music.data.Core
import io.github.honeyslatt.music.data.Devices
import io.github.honeyslatt.music.ui.components.at
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Job
import kotlinx.coroutines.flow.MutableSharedFlow
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.SharedFlow
import kotlinx.coroutines.flow.asSharedFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.SharingStarted
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.collectLatest
import kotlinx.coroutines.flow.combine
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.flow.stateIn
import kotlinx.coroutines.launch

/**
 * What the player made of the sound, as Media3 says it: the decoder, what
 * went in, and the track it opened with Android.
 */
data class Signal(
    val decoder: String? = null,
    val input: Format? = null,
    val output: AudioSink.AudioTrackConfig? = null,
)

/** How many songs of a station are asked for at once. */
private const val STATION_SONGS = 20u

/** How many songs in a row may fail to play before playback stops. */
private const val MAX_SKIPS = 5

/**
 * What plays: the queue (the Rust core's, as on the desktop) and the
 * player. The player only ever holds the current song and the next one,
 * each made playable just before it is needed, so that the next follows
 * without a gap while the whole queue isn't resolved at once.
 *
 * Another device can be chosen to play on instead (Music Connect): the
 * screens then show what it plays, and what they ask goes to it.
 */
@UnstableApi
class Playback(private val context: Context, private val core: Core, private val scope: CoroutineScope, val devices: Devices) {
    val queue = PlayQueue()

    // Decoded to float rather than 16 bits: a 24-bit file keeps its bits
    // on the way to Android's mixer, which works in float itself.
    val player: ExoPlayer = ExoPlayer.Builder(context, androidx.media3.exoplayer.DefaultRenderersFactory(context).setEnableAudioFloatOutput(true))
        .setAudioAttributes(
            AudioAttributes.Builder().setUsage(C.USAGE_MEDIA).setContentType(C.AUDIO_CONTENT_TYPE_MUSIC).build(),
            true,
        )
        .setHandleAudioBecomingNoisy(true)
        // Start once half a second is in, rather than the default 2.5 s.
        .setLoadControl(DefaultLoadControl.Builder().setBufferDurationsMs(20_000, 60_000, 500, 1_500).build())
        // The screen off, the Wi-Fi kept awake while it plays: a stream
        // starved by power saving would stall.
        .setWakeMode(C.WAKE_MODE_NETWORK)
        .build()

    private val _now = MutableStateFlow<Track?>(null)
    private val _playing = MutableStateFlow(false)
    private val _loading = MutableStateFlow(false)

    private val _error = MutableStateFlow<String?>(null)
    val error: StateFlow<String?> = _error.asStateFlow()

    /** What follows, in play order, and what played, most recent first. */
    private val _upcoming = MutableStateFlow<List<Track>>(emptyList())
    private val _history = MutableStateFlow<List<Track>>(emptyList())
    private val _shuffle = MutableStateFlow(false)
    private val _repeat = MutableStateFlow(QueueRepeat.OFF)

    /** The device chosen to play on, when it isn't this phone: its id. */
    private val target = MutableStateFlow<String?>(null)

    /** The device played on, when it isn't this phone, as last heard from. */
    val remote: StateFlow<Peer?> = combine(target, devices.peers) { id, peers ->
        id?.let { peers.find { p -> p.device.id == it && p.connected } }
    }.stateIn(scope, SharingStarted.Eagerly, null)

    /** This phone's `local`, or what the device played on says. */
    private fun <T> shown(local: StateFlow<T>, far: (Peer) -> T): StateFlow<T> =
        combine(local, remote) { l, r -> if (r != null) far(r) else l }.stateIn(scope, SharingStarted.Eagerly, local.value)

    /** The song playing, or on its way. */
    val now: StateFlow<Track?> = shown(_now) { it.playing?.track }
    val playing: StateFlow<Boolean> = shown(_playing) { it.playing?.playing == true }

    /** The current song is still being made playable. */
    val loading: StateFlow<Boolean> = shown(_loading) { false }

    /** This phone's song on its way, read at once, as the player stops for it. */
    val coming: Track? get() = _now.value?.takeIf { _loading.value }
    val upcoming: StateFlow<List<Track>> = shown(_upcoming) { it.upcoming }
    val history: StateFlow<List<Track>> = shown(_history) { emptyList() }
    val shuffleOn: StateFlow<Boolean> = shown(_shuffle) { it.playing?.shuffle == true }
    val repeat: StateFlow<QueueRepeat> = shown(_repeat) { queueRepeat(it.playing?.repeat ?: Repeat.OFF) }

    private val _signal = MutableStateFlow(Signal())
    val signal: StateFlow<Signal> = _signal.asStateFlow()

    /** Where the song is, in ms; read as often as wanted. */
    val position: Long get() = remote.value?.let(::farPosition) ?: herePosition

    /** Where the song is here: as far as it played, or where the one shown since the app opened goes on from. */
    private val herePosition: Long
        get() = resume?.takeIf { player.playbackState == Player.STATE_IDLE }?.second
            ?: player.currentPosition.coerceAtLeast(0)

    /** The song that played when the app last closed, shown paused, and where it was. */
    private var resume: Pair<String, Long>? = null

    /** Where the song the device played on plays is by now. */
    private fun farPosition(peer: Peer): Long {
        val p = peer.playing ?: return 0
        var ms = p.positionMs.toLong()
        if (p.playing) ms += SystemClock.elapsedRealtime() - devices.peersAt
        return p.durationMs?.let { ms.coerceAtMost(it.toLong()) } ?: ms
    }

    /** How long the song is, in ms, once known. */
    val duration: Long?
        get() = remote.value?.let { it.playing?.durationMs?.toLong() } ?: localDuration

    private val localDuration: Long?
        get() = player.duration.takeIf { it != C.TIME_UNSET && it > 0 }
            ?: _now.value?.duration?.toMillis()

    /** Set while carrying out what another device asked: it is for this phone's player. */
    private var obeying = false

    /** The device what is asked goes to, if not this phone's player. */
    private fun far(): String? = if (obeying) null else remote.value?.device?.id

    /** Ask the device played on, or do it `here`. */
    private inline fun either(command: () -> Command, here: () -> Unit) {
        val id = far()
        if (id != null) devices.send(id, command()) else here()
    }

    fun seekTo(ms: Long) = either({ Command.Seek(ms.coerceAtLeast(0).toULong()) }) {
        // Not made playable yet: it goes on from there.
        val r = resume
        if (r != null && player.playbackState == Player.STATE_IDLE) resume = r.first to ms.coerceAtLeast(0) else player.seekTo(ms)
    }

    fun setShuffle(on: Boolean) = either({ Command.SetShuffle(on) }) {
        queue.setShuffle(on)
        changed()
        lineUpNext()
    }

    /** Off, all, one, off. */
    fun cycleRepeat() {
        val next = when (repeat.value) {
            QueueRepeat.OFF -> QueueRepeat.ALL
            QueueRepeat.ALL -> QueueRepeat.ONE
            QueueRepeat.ONE -> QueueRepeat.OFF
        }
        either({ Command.SetRepeat(repeatOf(next)) }) { setRepeat(next) }
    }

    private fun setRepeat(repeat: QueueRepeat) {
        queue.setRepeat(repeat)
        changed()
        lineUpNext()
    }

    /** Play the `k`-th upcoming song now. */
    fun jump(k: Int) = either({ Command.Jump(k.toUInt()) }) {
        val previous = _now.value
        if (queue.jump(k.toUInt()) != null) {
            previous?.let(::played)
            startCurrent()
        }
    }

    fun removeUpcoming(k: Int) = either({ Command.RemoveUpcoming(k.toUInt()) }) {
        queue.removeUpcoming(k.toUInt())
        changed()
        if (k == 0) lineUpNext()
    }

    fun clearUpcoming() = either({ Command.ClearUpcoming }) {
        queue.clearUpcoming()
        changed()
        lineUpNext()
    }

    /** What the volume keys last set the device played on to, and when. */
    private var keyed: Pair<Float, Long>? = null

    private val _remoteVolume = MutableSharedFlow<Float>(extraBufferCapacity = 1)

    /** The volume of the device played on, as the volume keys just set it. */
    val remoteVolume: SharedFlow<Float> = _remoteVolume.asSharedFlow()

    /**
     * A press of a volume key in the app: a twentieth up or down on the
     * device played on. False when none is, or its volume isn't to be set
     * from here: the key is then the phone's.
     */
    fun stepRemoteVolume(up: Boolean): Boolean {
        val r = remote.value ?: return false
        val reported = r.playing?.volume ?: return false
        val now = SystemClock.elapsedRealtime()
        // Pressed again before the device said where it is: on from there.
        val from = keyed?.takeIf { now - it.second < 2_000 }?.first ?: reported
        val to = (Math.round(from * 20 + if (up) 1 else -1) / 20f).coerceIn(0f, 1f)
        keyed = to to now
        devices.send(r.device.id, Command.SetVolume(to))
        _remoteVolume.tryEmit(to)
        return true
    }

    /** The volume of the device played on, from 0 to 1. */
    fun setRemoteVolume(volume: Float) {
        remote.value?.let { devices.send(it.device.id, Command.SetVolume(volume.coerceIn(0f, 1f))) }
    }

    fun clearHistory() {
        _history.value = emptyList()
        keepHistory()
    }

    /** The station of songs playing, which goes on as the queue runs out. */
    private var station: io.github.honeyslatt.music.core.model.ItemId? = null
    private var refilling = false

    /** The queue's state, as the screens and the other devices show it. */
    private fun changed() {
        _upcoming.value = queue.upcoming()
        _shuffle.value = queue.shuffle()
        _repeat.value = queue.repeat()
        devices.publishUpcoming(_upcoming.value.take(100))
        tell()
        refill()
    }

    /** A few songs left of a station: its next ones join the queue. */
    private fun refill() {
        val s = station ?: return
        if (refilling || _upcoming.value.size > 3) return
        refilling = true
        scope.launch {
            runCatching { core.session.stationTracks(s, STATION_SONGS) }.onSuccess { more ->
                if (station == s && more.isNotEmpty()) {
                    more.forEach(queue::addLast)
                    refilling = false
                    changed()
                    lineUpNext()
                }
            }
            refilling = false
        }
    }

    private fun played(track: Track) {
        _history.value = (listOf(track) + _history.value.filter { it.id != track.id }).take(100)
        keepHistory()
    }

    /** Where the songs played are kept, from one launch to the next. */
    private val historyFile = java.io.File(context.filesDir, "history.json").path

    private fun keepHistory() {
        val history = _history.value
        scope.launch(kotlinx.coroutines.Dispatchers.IO) { io.github.honeyslatt.music.core.keepTracks(historyFile, history) }
    }

    /** Bumped when the queue starts over, so late resolutions are dropped. */
    private var generation = 0
    private var lining: Job? = null
    private var failedInRow = 0

    init {
        _history.value = io.github.honeyslatt.music.core.keptTracks(historyFile)
        Http.warm()
        player.addListener(object : Player.Listener {

            override fun onIsPlayingChanged(isPlaying: Boolean) {
                _playing.value = isPlaying
                if (isPlaying) failedInRow = 0
                devices.playing(isPlaying)
                tell()
                if (!isPlaying) keepSession()
            }

            // A seek, or a song that moved on: told at once.
            override fun onPositionDiscontinuity(oldPosition: Player.PositionInfo, newPosition: Player.PositionInfo, reason: Int) {
                tell()
            }

            // Paused at the end of the song, for the sleep timer: it is done.
            override fun onPlayWhenReadyChanged(playWhenReady: Boolean, reason: Int) {
                if (reason == Player.PLAY_WHEN_READY_CHANGE_REASON_END_OF_MEDIA_ITEM && _sleep.value == Sleep.EndOfTrack) {
                    _sleep.value = null
                    player.pauseAtEndOfMediaItems = false
                }
            }

            override fun onMediaItemTransition(item: MediaItem?, reason: Int) {
                // The next song followed on its own: the queue moves on.
                if (reason == Player.MEDIA_ITEM_TRANSITION_REASON_AUTO) queue.trackFinished()
                if (reason == Player.MEDIA_ITEM_TRANSITION_REASON_AUTO ||
                    reason == Player.MEDIA_ITEM_TRANSITION_REASON_SEEK
                ) {
                    // Only the song playing stays, and the next is lined up.
                    while (player.currentMediaItemIndex > 0) player.removeMediaItem(0)
                    _now.value?.let(::played)
                    _now.value = queue.current()
                    changed()
                    failedInRow = 0
                    lineUpNext()
                    keepSession()
                }
            }

            // The song ended with nothing after it in the player: the next
            // one couldn't be made playable in time, or at all. The queue
            // moves on as if it had followed. Not when paused there by the
            // sleep timer.
            override fun onPlaybackStateChanged(state: Int) {
                if (state != Player.STATE_ENDED || !player.playWhenReady) return
                val previous = _now.value
                if (queue.trackFinished() != null) {
                    previous?.let(::played)
                    startCurrent()
                }
            }

            override fun onPlayerError(error: PlaybackException) {
                _error.value = "Couldn't play “${_now.value?.title}”: ${error.errorCodeName}"
                when {
                    // Paused on a station too long: back to what airs now.
                    error.errorCode == PlaybackException.ERROR_CODE_BEHIND_LIVE_WINDOW -> {
                        player.seekToDefaultPosition()
                        player.prepare()
                    }
                    // The network gone (a tunnel, the metro): the song waits
                    // where it was, to go on once it is back, rather than the
                    // queue running through songs that can't load either.
                    error.errorCode in 2000..2999 -> {}
                    else -> skipFailed()
                }
            }
        })
        player.addAnalyticsListener(object : AnalyticsListener {
            override fun onAudioDecoderInitialized(
                eventTime: AnalyticsListener.EventTime,
                decoderName: String,
                initializedTimestampMs: Long,
                initializationDurationMs: Long,
            ) {
                _signal.value = _signal.value.copy(decoder = decoderName)
            }

            override fun onAudioInputFormatChanged(
                eventTime: AnalyticsListener.EventTime,
                format: Format,
                decoderReuseEvaluation: DecoderReuseEvaluation?,
            ) {
                _signal.value = _signal.value.copy(input = format)
            }

            override fun onAudioTrackInitialized(eventTime: AnalyticsListener.EventTime, audioTrackConfig: AudioSink.AudioTrackConfig) {
                _signal.value = _signal.value.copy(output = audioTrackConfig)
            }
        })
    }

    /** When the music stops by itself: at a time, or at the end of the song. */
    sealed interface Sleep {
        /** At [endsAt], in [SystemClock.elapsedRealtime] ms, after [minutes]. */
        data class At(val minutes: Int, val endsAt: Long) : Sleep

        data object EndOfTrack : Sleep
    }

    private val _sleep = MutableStateFlow<Sleep?>(null)
    val sleep: StateFlow<Sleep?> = _sleep.asStateFlow()
    private var sleeping: Job? = null

    /** Pause in `minutes`, or never when null. */
    fun sleepIn(minutes: Int?) {
        sleeping?.cancel()
        player.pauseAtEndOfMediaItems = false
        if (minutes == null) {
            _sleep.value = null
            return
        }
        val endsAt = SystemClock.elapsedRealtime() + minutes * 60_000L
        _sleep.value = Sleep.At(minutes, endsAt)
        sleeping = scope.launch {
            // Checked against the clock: a delay alone stretches while the phone sleeps.
            while (SystemClock.elapsedRealtime() < endsAt) kotlinx.coroutines.delay(1_000)
            fallAsleep()
        }
    }

    /** Pause once the song playing ends. */
    fun sleepAtEndOfTrack() {
        sleeping?.cancel()
        _sleep.value = Sleep.EndOfTrack
        val far = remote.value?.playing?.track?.id
        if (far != null) {
            // The device played on: paused as soon as it moves on.
            sleeping = scope.launch {
                remote.first { it == null || it.playing?.track?.id != far }
                fallAsleep()
            }
        } else {
            player.pauseAtEndOfMediaItems = true
        }
    }

    /** The timer ran out: what plays, here or on the device played on, pauses. */
    private fun fallAsleep() {
        _sleep.value = null
        player.pauseAtEndOfMediaItems = false
        val id = remote.value?.device?.id
        if (id != null) devices.send(id, Command.Pause) else player.pause()
    }

    /** Play `tracks` from `start`. */
    fun play(tracks: List<Track>, start: Int) {
        if (tracks.isEmpty()) return
        either({ Command.PlayList(tracks, start.toUInt()) }) { playHere(tracks, start) }
    }

    private fun playHere(tracks: List<Track>, start: Int) {
        failedInRow = 0
        station = null
        queue.replace(tracks, start.toUInt())
        startCurrent()
    }

    /** Stations playing as the queue's one song, by their id. */
    private val stations = mutableMapOf<String, io.github.honeyslatt.music.core.model.Station>()

    /**
     * Tune in: a station of songs plays them; one on air (or an episode)
     * plays its stream as the queue's one song.
     */
    suspend fun playStation(station: io.github.honeyslatt.music.core.model.Station) {
        far()?.let { devices.send(it, Command.PlayStation(station)); return }
        if (station.kind == io.github.honeyslatt.music.core.model.StationKind.TRACKS) return playStation(station.id)
        stations[station.id.id] = station
        playHere(listOf(stationSong(station)), 0)
    }

    /** A station's stream, as the queue's one song. */
    private fun stationSong(station: io.github.honeyslatt.music.core.model.Station) = Track(
        id = station.id,
        title = station.name,
        artists = listOf(if (station.kind == io.github.honeyslatt.music.core.model.StationKind.LIVE) "Live Radio" else "Radio Episode"),
        album = station.description,
        albumId = null,
        duration = null,
        trackNumber = null,
        discNumber = null,
        artwork = station.artwork,
        explicit = false,
        quality = null,
        inLibrary = false,
        libraryId = null,
    )

    /** A station of songs, going on for as long as it plays. */
    suspend fun playStation(id: io.github.honeyslatt.music.core.model.ItemId) {
        far()?.let {
            val station = io.github.honeyslatt.music.core.model.Station(id, "Station", null, null, io.github.honeyslatt.music.core.model.StationKind.TRACKS)
            devices.send(it, Command.PlayStation(station))
            return
        }
        val tracks = core.session.stationTracks(id, STATION_SONGS)
        if (tracks.isEmpty()) return
        playHere(tracks, 0)
        station = id
    }

    /** What a SoundCloud song plays from, once known: the uploader's file, or an encrypted stream. */
    enum class Source { Original, Encrypted }

    private val sources = androidx.compose.runtime.mutableStateMapOf<String, Source>()

    fun source(track: Track): Source? = sources[track.id.id]

    /** The real bit rates of Apple songs served below the usual 256 kbps, once read. */
    private val lesser = androidx.compose.runtime.mutableStateMapOf<String, UInt>()

    /** The Apple songs playing as Spatial Audio, their Dolby Atmos mix rendered binaurally. */
    private val atmos = androidx.compose.runtime.mutableStateSetOf<String>()

    /**
     * `track`'s quality as it plays: Apple's listing says 256 kbps even
     * when less is served, and AAC when its Dolby Atmos mix is what plays.
     */
    fun quality(track: Track): AudioQuality? = when {
        track.id.id in atmos -> AudioQuality(codec = Codec.Other("Dolby Atmos"), sampleRate = 48_000u, bitDepth = null, bitrateKbps = null)
        else -> lesser[track.id.id]?.let { kbps -> track.quality?.copy(bitrateKbps = kbps) } ?: track.quality
    }

    /** Whether `track` stands for a station's stream, rather than a song. */
    fun isStation(track: Track) = track.id.id in stations

    /** Whether what plays is a station on air, with no end to seek to. */
    fun isLive(track: Track) = stations[track.id.id]?.kind == io.github.honeyslatt.music.core.model.StationKind.LIVE

    /** Play `tracks` right after the current song, in their order. */
    fun playNext(tracks: List<Track>) {
        if (tracks.isEmpty()) return
        far()?.let { devices.send(it, Command.PlayNext(tracks)); return }
        if (_now.value == null) return play(tracks, 0)
        tracks.asReversed().forEach(queue::addNext)
        changed()
        lineUpNext()
    }

    /** Play `tracks` once everything else has. */
    fun addToQueue(tracks: List<Track>) {
        if (tracks.isEmpty()) return
        far()?.let { devices.send(it, Command.AddToQueue(tracks)); return }
        if (_now.value == null) return play(tracks, 0)
        tracks.forEach(queue::addLast)
        changed()
        lineUpNext()
    }

    /** Play `tracks` shuffled, from a random one. */
    fun shuffle(tracks: List<Track>) {
        if (tracks.isEmpty()) return
        far()?.let {
            devices.send(it, Command.SetShuffle(true))
            devices.send(it, Command.PlayList(tracks, tracks.indices.random().toUInt()))
            return
        }
        queue.setShuffle(true)
        play(tracks, tracks.indices.random())
    }

    fun toggle() = either({ Command.Toggle }) {
        when {
            player.isPlaying -> player.pause()
            _now.value == null -> player.play()
            // Stopped by the network: on from where it was.
            player.playerError != null && player.mediaItemCount > 0 -> {
                player.prepare()
                player.play()
            }
            // Not made playable yet (as the app opened), or after a failure.
            player.playbackState == Player.STATE_IDLE || player.mediaItemCount == 0 -> startCurrent()
            // The queue played to its end: its last song again.
            player.playbackState == Player.STATE_ENDED -> {
                player.seekToDefaultPosition()
                player.play()
            }
            else -> player.play()
        }
    }

    /** The network is back: a song stopped for want of it goes on. */
    fun online() {
        if (player.playerError?.errorCode in 2000..2999 && player.mediaItemCount > 0) {
            player.prepare()
            player.play()
        }
    }

    fun next() = either({ Command.Next }) {
        _now.value?.takeIf { !player.hasNextMediaItem() }?.let(::played)
        if (player.hasNextMediaItem() && queue.repeat() != QueueRepeat.ONE) {
            queue.skipNext()
            player.seekToNextMediaItem()
        } else if (queue.skipNext() != null) {
            startCurrent()
        }
    }

    /** Back to the song's start, or to the one before when already there. */
    fun previous() = either({ Command.Previous }) {
        if (player.currentPosition > 3_000) {
            player.seekTo(0)
        } else if (queue.skipPrevious() != null) {
            startCurrent()
        }
    }

    private fun skipFailed() {
        failedInRow++
        if (failedInRow <= MAX_SKIPS && queue.skipNext() != null) startCurrent() else player.stop()
    }

    /** Make the queue's current song playable and play it, at once. */
    private fun startCurrent() {
        val track = queue.current() ?: return
        var from = startAt
        startAt = 0
        resume?.let { (id, at) -> if (id == "${track.id.provider}:${track.id.id}") from = at }
        resume = null
        changed()
        val mine = ++generation
        lining?.cancel()
        // On its way before the player stops, for the session to show it
        // coming rather than nothing (see QueuePlayer).
        _now.value = track
        _loading.value = true
        player.stop()
        player.clearMediaItems()
        tell()
        scope.launch {
            val source = try {
                playable(track, mine)
            } catch (e: kotlinx.coroutines.CancellationException) {
                throw e
            } catch (e: Exception) {
                if (mine != generation) return@launch
                _loading.value = false
                _error.value = "Couldn't play “${track.title}”: ${e.message}"
                // Offline, every song would fail alike: the queue waits on
                // this one, for Play to try it again.
                if ((e as? MusicException.Failed)?.reason?.startsWith("network error") != true) skipFailed()
                return@launch
            }
            if (mine != generation) return@launch
            _loading.value = false
            if (from > 0) player.setMediaSource(source, from) else player.setMediaSource(source)
            player.prepare()
            player.play()
            lineUpNext()
        }
    }

    /** Have the song that follows in the player, so it joins without a gap. */
    private fun lineUpNext() {
        val mine = generation
        lining?.cancel()
        val next = queue.peekFinished()
        // Nothing follows any more (cleared, or repeat turned off on the
        // last song): what the player had lined up mustn't play.
        if (next == null) {
            while (player.mediaItemCount > player.currentMediaItemIndex + 1) player.removeMediaItem(player.mediaItemCount - 1)
            return
        }
        lining = scope.launch {
            val source = runCatching { playable(next, mine) }.getOrNull() ?: return@launch
            // Still what follows, in the same queue.
            if (mine != generation || queue.peekFinished()?.id != next.id) return@launch
            while (player.mediaItemCount > player.currentMediaItemIndex + 1) player.removeMediaItem(player.mediaItemCount - 1)
            player.addMediaSource(source)
        }
    }

    private var keys = 0

    /** Where the next song started goes on from: one handed over midway. */
    private var startAt = 0L

    /**
     * Play on the device `id`, or on this phone when null: what plays here
     * moves there, or what plays there comes here, as with Spotify's
     * devices. A device that plays while this phone doesn't is only
     * controlled from here.
     */
    fun playOn(id: String?) {
        val from = remote.value?.device?.id
        if (id == from) return
        if (id == null) {
            from?.let { devices.send(it, Command.HandOver) }
        } else if (from == null && player.isPlaying && devices.handOff(id, handoff())) {
            player.pause()
        }
        target.value = id
    }

    /** The queue as it stands, for another device to play on from here. */
    private fun handoff(): Handoff {
        val s = queue.snapshot()
        return Handoff(s.tracks, s.order, s.at, queue.shuffle(), repeatOf(queue.repeat()), herePosition.toULong(), player.isPlaying, station)
    }

    /** Where what plays is kept when the app closes. */
    private val sessionFile = java.io.File(context.filesDir, "session.json").path

    /** Keep what plays for the next time the app opens: the queue and where it is. */
    fun keepSession() {
        val session = handoff()
        scope.launch(kotlinx.coroutines.Dispatchers.IO) { io.github.honeyslatt.music.core.keepSession(sessionFile, session) }
    }

    /** Take over a queue as kept or handed over: its songs, its order, its station. */
    private fun adopt(h: Handoff) {
        // A station on air came as the queue's one song: it plays as one here too.
        h.tracks.filter { it.id.id.startsWith("ra.") && it.duration == null }.forEach {
            val kind = if (it.artists.any { a -> a.startsWith("Live", ignoreCase = true) }) {
                io.github.honeyslatt.music.core.model.StationKind.LIVE
            } else {
                io.github.honeyslatt.music.core.model.StationKind.EPISODE
            }
            stations[it.id.id] = io.github.honeyslatt.music.core.model.Station(it.id, it.title, it.album, it.artwork, kind)
        }
        queue.restore(h.tracks, h.order, h.at, h.shuffle)
        queue.setRepeat(queueRepeat(h.repeat))
        station = h.station
    }

    /** Show what played when the app last closed, paused where it was; made playable once played. */
    private fun resumeSession() {
        val h = io.github.honeyslatt.music.core.keptSession(sessionFile) ?: return
        adopt(h)
        queue.current()?.let { t ->
            _now.value = t
            resume = "${t.id.provider}:${t.id.id}" to h.positionMs.toLong()
        }
        // Shuffle and repeat show as they were, even with nothing queued.
        changed()
    }

    /** Play on from where another device left off. */
    private fun take(from: String, h: Handoff) {
        target.value = null
        val previous = _now.value
        adopt(h)
        previous?.let(::played)
        startAt = h.positionMs.toLong()
        startCurrent()
        devices.notice("Playing here, from ${devices.name(from)}")
    }

    /** Do what another device asked of this phone's player. */
    private fun obey(from: String, command: Command) {
        obeying = true
        try {
            when (command) {
                Command.Play -> if (!player.isPlaying) toggle()
                Command.Pause -> player.pause()
                Command.Toggle -> toggle()
                Command.Next -> next()
                Command.Previous -> previous()
                is Command.Seek -> seekTo(command.positionMs.toLong())
                is Command.SetVolume -> {
                    val audio = context.getSystemService(android.media.AudioManager::class.java)
                    val max = audio.getStreamMaxVolume(android.media.AudioManager.STREAM_MUSIC)
                    audio.setStreamVolume(android.media.AudioManager.STREAM_MUSIC, Math.round(command.volume * max), 0)
                }
                is Command.SetShuffle -> setShuffle(command.on)
                is Command.SetRepeat -> setRepeat(queueRepeat(command.repeat))
                is Command.Jump -> jump(command.k.toInt())
                is Command.RemoveUpcoming -> removeUpcoming(command.k.toInt())
                Command.ClearUpcoming -> clearUpcoming()
                is Command.PlayList -> playHere(command.tracks, command.start.toInt())
                is Command.PlayNext -> playNext(command.tracks)
                is Command.AddToQueue -> addToQueue(command.tracks)
                is Command.PlayStation -> {
                    val s = command.station
                    if (s.kind == io.github.honeyslatt.music.core.model.StationKind.TRACKS) {
                        scope.launch {
                            runCatching { core.session.stationTracks(s.id, STATION_SONGS) }.getOrNull()?.takeIf { it.isNotEmpty() }?.let {
                                playHere(it, 0)
                                station = s.id
                            }
                        }
                    } else {
                        stations[s.id.id] = s
                        playHere(listOf(stationSong(s)), 0)
                    }
                }
                Command.HandOver -> if (devices.handOff(from, handoff())) player.pause()
            }
        } finally {
            obeying = false
        }
    }

    /** Tell the other devices what plays here; only a change goes out. */
    private fun tell() {
        val t = _now.value
        val audio = context.getSystemService(android.media.AudioManager::class.java)
        val max = audio.getStreamMaxVolume(android.media.AudioManager.STREAM_MUSIC).coerceAtLeast(1)
        devices.publish(
            Playing(
                track = t,
                playing = player.isPlaying || _loading.value,
                positionMs = player.currentPosition.coerceAtLeast(0).toULong(),
                durationMs = localDuration?.toULong(),
                shuffle = queue.shuffle(),
                repeat = repeatOf(queue.repeat()),
                volume = audio.getStreamVolume(android.media.AudioManager.STREAM_MUSIC).toFloat() / max,
                live = t != null && isLive(t),
            ),
        )
    }

    /** Which devices played, to follow one that starts while this phone is quiet. */
    private val farPlaying = mutableMapOf<String, Boolean>()

    private fun follow(peers: List<Peer>) {
        for (p in peers) {
            val on = p.connected && p.playing?.playing == true
            val before = farPlaying.put(p.device.id, on) ?: false
            if (on && !before && target.value == null && !player.isPlaying && !_loading.value) target.value = p.device.id
        }
        // The device played on left: back to this phone.
        target.value?.let { id -> if (peers.none { it.device.id == id && it.connected }) target.value = null }
    }

    init {
        devices.heard = { event ->
            when (event) {
                is Event.Asked -> obey(event.from, event.command)
                is Event.Handed -> take(event.from, event.handoff)
                else -> {}
            }
        }
        resumeSession()
        scope.launch { devices.peers.collect(::follow) }
        // Changes are told as they happen; while it plays, now and then
        // too, should the music have stalled. Nothing runs while it doesn't.
        scope.launch {
            _playing.collectLatest { on ->
                while (on) {
                    kotlinx.coroutines.delay(5_000)
                    tell()
                }
            }
        }
    }

    /** A song as the player plays it, with what the lock screen shows. */
    private suspend fun playable(track: Track, generation: Int): MediaSource {
        val item = MediaItem.Builder()
            .setMediaId("${track.id.provider}:${track.id.id}")
            .setMediaMetadata(
                MediaMetadata.Builder()
                    .setTitle(track.title)
                    .setArtist(track.artists.joinToString(", "))
                    .setAlbumTitle(track.album)
                    .setArtworkUri(track.artwork?.at(600)?.let(Uri::parse))
                    .build(),
            )
        if (track.id.id in stations) return liveSource(core.session.liveStation(track.id), item)
        return when (track.id.provider) {
            ProviderKind.APPLE_MUSIC -> {
                // A catalog song Apple no longer offers here plays from the
                // library's copy, as Apple Music does.
                val spatial = (context.applicationContext as io.github.honeyslatt.music.MusicApp).settings.spatialAudio
                val protected = try {
                    core.session.protected(track.id, spatial)
                } catch (e: MusicException) {
                    val copy = track.libraryId?.takeIf { it != track.id } ?: throw e
                    core.session.protected(copy, spatial)
                } ?: error("a song you uploaded can't play here yet")
                if (protected.spatial()) atmos += track.id.id else atmos -= track.id.id
                scope.launch { protected.lesserKbps()?.let { lesser[track.id.id] = it } }
                appleSource(protected, "$generation-${keys++}", item, player)
            }
            ProviderKind.SOUND_CLOUD -> {
                val settings = (context.applicationContext as io.github.honeyslatt.music.MusicApp).settings
                val stream = core.session.soundcloudStream(track.id, settings.soundcloudOriginals)
                when {
                    stream.original() -> sources[track.id.id] = Source.Original
                    stream.encrypted() -> sources[track.id.id] = Source.Encrypted
                }
                soundCloudSource(stream, item)
            }
            ProviderKind.NAVIDROME -> navidromeSource(core.session.navidromeStreamUrl(track.id), item)
        }
    }
}

private fun queueRepeat(r: Repeat) = when (r) {
    Repeat.OFF -> QueueRepeat.OFF
    Repeat.ALL -> QueueRepeat.ALL
    Repeat.ONE -> QueueRepeat.ONE
}

private fun repeatOf(r: QueueRepeat) = when (r) {
    QueueRepeat.OFF -> Repeat.OFF
    QueueRepeat.ALL -> Repeat.ALL
    QueueRepeat.ONE -> Repeat.ONE
}
