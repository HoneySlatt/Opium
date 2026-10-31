package io.github.honeyslatt.music.playback

import android.net.Uri
import androidx.media3.common.C
import androidx.media3.common.MediaItem
import androidx.media3.common.MimeTypes
import androidx.media3.common.util.UnstableApi
import androidx.media3.datasource.ByteArrayDataSource
import androidx.media3.datasource.DataSource
import androidx.media3.datasource.DataSpec
import androidx.media3.datasource.TransferListener
import androidx.media3.exoplayer.drm.DefaultDrmSessionManager
import androidx.media3.exoplayer.drm.ExoMediaDrm
import androidx.media3.exoplayer.drm.FrameworkMediaDrm
import androidx.media3.exoplayer.drm.HttpMediaDrmCallback
import androidx.media3.exoplayer.drm.MediaDrmCallback
import androidx.media3.exoplayer.hls.HlsMediaSource
import io.github.honeyslatt.music.core.ProtectedTrack
import java.util.UUID
import kotlinx.coroutines.runBlocking
import kotlinx.coroutines.withTimeoutOrNull
import androidx.media3.common.DrmInitData
import androidx.media3.common.Format
import androidx.media3.exoplayer.ExoPlayer
import androidx.media3.exoplayer.analytics.PlayerId

/**
 * An Apple song as ExoPlayer plays it: its playlist from the core, its audio
 * from Apple, its licence through the core with the phone's Widevine, and
 * its encoder's priming and padding left out so that songs follow without a
 * gap. Checked on a Pixel 10 (Widevine L1) in step 0.
 */
@UnstableApi
fun appleSource(
    track: ProtectedTrack,
    key: String,
    item: MediaItem.Builder = MediaItem.Builder(),
    player: ExoPlayer? = null,
): HlsMediaSource {
    val playlistUri = Uri.parse("music-playlist://apple/$key/playlist.m3u8")
    val playlist = track.playlist().toByteArray()
    // Spatial Audio's init segment, with its key ids written in, which the
    // playlist names beside it.
    val initUri = Uri.parse("music-playlist://apple/$key/init.mp4")
    val init = track.init()
    val http = Http.sources
    // The playlist comes from the core; the audio from Apple's servers.
    val sources = DataSource.Factory {
        object : DataSource {
            private var inner: DataSource? = null
            private val listeners = mutableListOf<TransferListener>()

            override fun addTransferListener(listener: TransferListener) {
                listeners += listener
            }

            override fun open(spec: DataSpec): Long {
                val source = when {
                    spec.uri == playlistUri -> ByteArrayDataSource(playlist)
                    spec.uri == initUri && init != null -> ByteArrayDataSource(init)
                    else -> http.createDataSource()
                }
                listeners.forEach(source::addTransferListener)
                inner = source
                return source.open(spec)
            }

            override fun read(buffer: ByteArray, offset: Int, length: Int) = inner!!.read(buffer, offset, length)

            override fun getUri(): Uri? = inner?.uri

            override fun close() {
                inner?.close()
                inner = null
            }
        }
    }
    val licences = object : MediaDrmCallback {
        override fun executeProvisionRequest(uuid: UUID, request: ExoMediaDrm.ProvisionRequest) =
            HttpMediaDrmCallback(null, http).executeProvisionRequest(uuid, request)

        override fun executeKeyRequest(uuid: UUID, request: ExoMediaDrm.KeyRequest) =
            MediaDrmCallback.Response(runBlocking { track.license(request.data) })
    }
    val drm = DefaultDrmSessionManager.Builder()
        .setUuidAndExoMediaDrmProvider(C.WIDEVINE_UUID, FrameworkMediaDrm.DEFAULT_PROVIDER)
        // Spatial Audio's file changes key after its first segment: a
        // session for each.
        .setMultiSession(true)
        .build(licences)
    // The licence is asked for at once, with the key already known, rather
    // than once the player has read the file's start.
    if (player != null) {
        drm.setPlayer(player.playbackLooper, PlayerId.UNSET)
        drm.prepare()
        val pssh = DrmInitData(DrmInitData.SchemeData(C.WIDEVINE_UUID, MimeTypes.VIDEO_MP4, track.pssh()))
        val held = drm.preacquireSession(null, Format.Builder().setSampleMimeType(MimeTypes.AUDIO_AAC).setDrmInitData(pssh).build())
        // Let go once this song plays, the player holding its own reference
        // then (a song lined up comes in without a pause and a play), or
        // after a while if it never does: thrown away with the queue.
        val id = item.build().mediaId
        val handler = android.os.Handler(android.os.Looper.getMainLooper())
        val listener = object : androidx.media3.common.Player.Listener {
            var released = false
            val later = Runnable { letGo() }

            fun letGo() {
                if (released) return
                released = true
                held.release()
                drm.release()
                player.removeListener(this)
                handler.removeCallbacks(later)
            }

            fun check() {
                if (player.isPlaying && player.currentMediaItem?.mediaId == id) letGo()
            }

            override fun onIsPlayingChanged(isPlaying: Boolean) = check()

            override fun onMediaItemTransition(mediaItem: androidx.media3.common.MediaItem?, reason: Int) = check()
        }
        player.addListener(listener)
        handler.postDelayed(listener.later, 10 * 60_000L)
    }
    val padding = { runBlocking { withTimeoutOrNull(2_000) { track.padding() } }?.toInt() ?: 0 }
    return HlsMediaSource.Factory(sources)
        .setExtractorFactory(Trimmed.Factory(track.priming().toInt(), padding))
        .setDrmSessionManagerProvider { drm }
        .createMediaSource(item.setUri(playlistUri).setMimeType(MimeTypes.APPLICATION_M3U8).build())
}

/**
 * A station's stream: Apple's HLS playlist as it airs, its Widevine keys
 * licensed through the core (the radio key server names a key by the
 * playlist's key line, which the core reads). Stations from other
 * broadcasters aren't encrypted.
 */
@UnstableApi
fun liveSource(station: io.github.honeyslatt.music.core.LiveStation, item: MediaItem.Builder): HlsMediaSource {
    val init = station.initUri()?.let(Uri::parse)
    val http = Http.sources
    // The init segment goes through the core, which names the key an
    // episode's leaves out; the rest comes from Apple as it is.
    val sources = DataSource.Factory {
        object : DataSource {
            private var inner: DataSource? = null
            private val listeners = mutableListOf<TransferListener>()

            override fun addTransferListener(listener: TransferListener) {
                listeners += listener
            }

            override fun open(spec: DataSpec): Long {
                val source = if (init != null && spec.uri == init) {
                    val fetched = http.createDataSource()
                    val bytes = try {
                        fetched.open(spec.buildUpon().setPosition(0).setLength(C.LENGTH_UNSET.toLong()).build())
                        androidx.media3.datasource.DataSourceUtil.readToEnd(fetched)
                    } finally {
                        fetched.close()
                    }
                    ByteArrayDataSource(station.fixInit(bytes))
                } else {
                    http.createDataSource()
                }
                listeners.forEach(source::addTransferListener)
                inner = source
                return source.open(spec)
            }

            override fun read(buffer: ByteArray, offset: Int, length: Int) = inner!!.read(buffer, offset, length)

            override fun getUri(): Uri? = inner?.uri

            override fun close() {
                inner?.close()
                inner = null
            }
        }
    }
    val factory = HlsMediaSource.Factory(sources)
    if (station.encrypted()) {
        val licences = object : MediaDrmCallback {
            override fun executeProvisionRequest(uuid: UUID, request: ExoMediaDrm.ProvisionRequest) =
                HttpMediaDrmCallback(null, Http.sources).executeProvisionRequest(uuid, request)

            override fun executeKeyRequest(uuid: UUID, request: ExoMediaDrm.KeyRequest) =
                MediaDrmCallback.Response(runBlocking { station.license(request.data) })
        }
        val drm = DefaultDrmSessionManager.Builder()
            .setUuidAndExoMediaDrmProvider(C.WIDEVINE_UUID, FrameworkMediaDrm.DEFAULT_PROVIDER)
            // A live stream changes keys as it goes.
            .setMultiSession(true)
            .build(licences)
        factory.setDrmSessionManagerProvider { drm }
    }
    return factory.createMediaSource(item.setUri(station.master()).setMimeType(MimeTypes.APPLICATION_M3U8).build())
}
