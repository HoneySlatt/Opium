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
import io.github.honeyslatt.music.core.PlayQueue
import io.github.honeyslatt.music.core.QueueRepeat
import io.github.honeyslatt.music.core.model.ProviderKind
import io.github.honeyslatt.music.core.model.Track
import io.github.honeyslatt.music.data.Core
import io.github.honeyslatt.music.ui.components.at
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Job
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
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

/** How many songs in a row may fail to play before playback stops. */
private const val MAX_SKIPS = 5

/**
 * What plays: the queue (the Rust core's, as on the desktop) and the
 * player. The player only ever holds the current song and the next one,
 * each made playable just before it is needed, so that the next follows
 * without a gap while the whole queue isn't resolved at once.
 */
@UnstableApi
class Playback(context: Context, private val core: Core, private val scope: CoroutineScope) {
    val queue = PlayQueue()

    val player: ExoPlayer = ExoPlayer.Builder(context)
        .setAudioAttributes(
            AudioAttributes.Builder().setUsage(C.USAGE_MEDIA).setContentType(C.AUDIO_CONTENT_TYPE_MUSIC).build(),
            true,
        )
        .setHandleAudioBecomingNoisy(true)
        // Start once half a second is in, rather than the default 2.5 s.
        .setLoadControl(DefaultLoadControl.Builder().setBufferDurationsMs(20_000, 60_000, 500, 1_500).build())
        .build()

    private val _now = MutableStateFlow<Track?>(null)

    /** The song playing, or on its way. */
    val now: StateFlow<Track?> = _now.asStateFlow()

    private val _playing = MutableStateFlow(false)
    val playing: StateFlow<Boolean> = _playing.asStateFlow()

    private val _loading = MutableStateFlow(false)

    /** The current song is still being made playable. */
    val loading: StateFlow<Boolean> = _loading.asStateFlow()

    private val _error = MutableStateFlow<String?>(null)
    val error: StateFlow<String?> = _error.asStateFlow()

    /** What follows, in play order, and what played, most recent first. */
    private val _upcoming = MutableStateFlow<List<Track>>(emptyList())
    val upcoming: StateFlow<List<Track>> = _upcoming.asStateFlow()
    private val _history = MutableStateFlow<List<Track>>(emptyList())
    val history: StateFlow<List<Track>> = _history.asStateFlow()

    private val _shuffle = MutableStateFlow(false)
    val shuffleOn: StateFlow<Boolean> = _shuffle.asStateFlow()
    private val _repeat = MutableStateFlow(QueueRepeat.OFF)
    val repeat: StateFlow<QueueRepeat> = _repeat.asStateFlow()

    private val _signal = MutableStateFlow(Signal())
    val signal: StateFlow<Signal> = _signal.asStateFlow()

    /** Where the song is, in ms; read as often as wanted. */
    val position: Long get() = player.currentPosition.coerceAtLeast(0)

    /** How long the song is, in ms, once known. */
    val duration: Long?
        get() = player.duration.takeIf { it != C.TIME_UNSET && it > 0 }
            ?: _now.value?.duration?.toMillis()

    fun seekTo(ms: Long) = player.seekTo(ms)

    fun setShuffle(on: Boolean) {
        queue.setShuffle(on)
        changed()
        lineUpNext()
    }

    /** Off, all, one, off. */
    fun cycleRepeat() {
        queue.setRepeat(
            when (queue.repeat()) {
                QueueRepeat.OFF -> QueueRepeat.ALL
                QueueRepeat.ALL -> QueueRepeat.ONE
                QueueRepeat.ONE -> QueueRepeat.OFF
            },
        )
        changed()
        lineUpNext()
    }

    /** Play the `k`-th upcoming song now. */
    fun jump(k: Int) {
        val previous = _now.value
        if (queue.jump(k.toUInt()) != null) {
            previous?.let(::played)
            startCurrent()
        }
    }

    fun removeUpcoming(k: Int) {
        queue.removeUpcoming(k.toUInt())
        changed()
        if (k == 0) lineUpNext()
    }

    fun clearUpcoming() {
        queue.clearUpcoming()
        changed()
        lineUpNext()
    }

    fun clearHistory() {
        _history.value = emptyList()
    }

    /** The queue's state, as the screens show it. */
    private fun changed() {
        _upcoming.value = queue.upcoming()
        _shuffle.value = queue.shuffle()
        _repeat.value = queue.repeat()
    }

    private fun played(track: Track) {
        _history.value = (listOf(track) + _history.value.filter { it.id != track.id }).take(100)
    }

    /** Bumped when the queue starts over, so late resolutions are dropped. */
    private var generation = 0
    private var lining: Job? = null
    private var failedInRow = 0

    init {
        Http.warm()
        player.addListener(object : Player.Listener {

            override fun onIsPlayingChanged(isPlaying: Boolean) {
                _playing.value = isPlaying
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
                }
            }

            override fun onPlayerError(error: PlaybackException) {
                _error.value = "Couldn't play “${_now.value?.title}”: ${error.errorCodeName}"
                skipFailed()
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

    /** Play `tracks` from `start`. */
    fun play(tracks: List<Track>, start: Int) {
        if (tracks.isEmpty()) return
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
        if (station.kind == io.github.honeyslatt.music.core.model.StationKind.TRACKS) {
            play(core.session.stationTracks(station.id, 25u), 0)
            return
        }
        stations[station.id.id] = station
        val song = Track(
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
        play(listOf(song), 0)
    }

    /** Whether `track` stands for a station's stream, rather than a song. */
    fun isStation(track: Track) = track.id.id in stations

    /** Whether what plays is a station on air, with no end to seek to. */
    fun isLive(track: Track) = stations[track.id.id]?.kind == io.github.honeyslatt.music.core.model.StationKind.LIVE

    /** Play `tracks` right after the current song, in their order. */
    fun playNext(tracks: List<Track>) {
        if (tracks.isEmpty()) return
        if (_now.value == null) return play(tracks, 0)
        tracks.asReversed().forEach(queue::addNext)
        changed()
        lineUpNext()
    }

    /** Play `tracks` once everything else has. */
    fun addToQueue(tracks: List<Track>) {
        if (tracks.isEmpty()) return
        if (_now.value == null) return play(tracks, 0)
        tracks.forEach(queue::addLast)
        changed()
        lineUpNext()
    }

    /** Play `tracks` shuffled, from a random one. */
    fun shuffle(tracks: List<Track>) {
        if (tracks.isEmpty()) return
        queue.setShuffle(true)
        play(tracks, tracks.indices.random())
    }

    fun toggle() {
        if (player.playbackState == Player.STATE_IDLE && _now.value != null) {
            startCurrent()
        } else if (player.isPlaying) {
            player.pause()
        } else {
            player.play()
        }
    }

    fun next() {
        _now.value?.takeIf { !player.hasNextMediaItem() }?.let(::played)
        if (player.hasNextMediaItem() && queue.repeat() != QueueRepeat.ONE) {
            queue.skipNext()
            player.seekToNextMediaItem()
        } else if (queue.skipNext() != null) {
            startCurrent()
        }
    }

    /** Back to the song's start, or to the one before when already there. */
    fun previous() {
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
        changed()
        val mine = ++generation
        lining?.cancel()
        player.stop()
        player.clearMediaItems()
        _now.value = track
        _loading.value = true
        scope.launch {
            val source = try {
                playable(track, mine)
            } catch (e: Exception) {
                if (mine != generation) return@launch
                _loading.value = false
                _error.value = "Couldn't play “${track.title}”: ${e.message}"
                skipFailed()
                return@launch
            }
            if (mine != generation) return@launch
            _loading.value = false
            player.setMediaSource(source)
            player.prepare()
            player.play()
            lineUpNext()
        }
    }

    /** Have the song that follows in the player, so it joins without a gap. */
    private fun lineUpNext() {
        val next = queue.peekFinished() ?: return
        val mine = generation
        lining?.cancel()
        lining = scope.launch {
            val source = runCatching { playable(next, mine) }.getOrNull() ?: return@launch
            // Still what follows, in the same queue.
            if (mine != generation || queue.peekFinished()?.id != next.id) return@launch
            while (player.mediaItemCount > player.currentMediaItemIndex + 1) player.removeMediaItem(player.mediaItemCount - 1)
            player.addMediaSource(source)
        }
    }

    private var keys = 0

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
                val protected = core.session.protected(track.id) ?: error("a song you uploaded can't play here yet")
                appleSource(protected, "$generation-${keys++}", item, player)
            }
            else -> error("${track.id.provider} plays once it is connected")
        }
    }
}
