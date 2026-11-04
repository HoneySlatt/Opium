package io.github.honeyslatt.music.playback

import androidx.media3.common.C
import androidx.media3.common.MediaItem
import androidx.media3.common.MimeTypes
import androidx.media3.common.util.UnstableApi
import androidx.media3.exoplayer.drm.DefaultDrmSessionManager
import androidx.media3.exoplayer.drm.ExoMediaDrm
import androidx.media3.exoplayer.drm.FrameworkMediaDrm
import androidx.media3.exoplayer.drm.HttpMediaDrmCallback
import androidx.media3.exoplayer.drm.MediaDrmCallback
import androidx.media3.exoplayer.hls.HlsMediaSource
import androidx.media3.exoplayer.source.MediaSource
import androidx.media3.exoplayer.source.ProgressiveMediaSource
import io.github.honeyslatt.music.core.SoundCloudStream
import java.util.UUID
import kotlinx.coroutines.runBlocking

/**
 * A SoundCloud track: the uploader's own file, played as it is; else its
 * best AAC stream over HLS, licensed with the phone's Widevine when it is
 * encrypted (a major label's tracks).
 */
@UnstableApi
fun soundCloudSource(stream: SoundCloudStream, item: MediaItem.Builder): MediaSource {
    if (!stream.hls()) {
        return ProgressiveMediaSource.Factory(Http.sources).createMediaSource(item.setUri(stream.url()).build())
    }
    val factory = HlsMediaSource.Factory(Http.sources)
    if (stream.encrypted()) {
        val licences = object : MediaDrmCallback {
            override fun executeProvisionRequest(uuid: UUID, request: ExoMediaDrm.ProvisionRequest) =
                HttpMediaDrmCallback(null, Http.sources).executeProvisionRequest(uuid, request)

            override fun executeKeyRequest(uuid: UUID, request: ExoMediaDrm.KeyRequest) =
                MediaDrmCallback.Response(runBlocking { stream.license(request.data) })
        }
        val drm = DefaultDrmSessionManager.Builder()
            .setUuidAndExoMediaDrmProvider(C.WIDEVINE_UUID, FrameworkMediaDrm.DEFAULT_PROVIDER)
            .build(licences)
        factory.setDrmSessionManagerProvider { drm }
    }
    return factory.createMediaSource(item.setUri(stream.url()).setMimeType(MimeTypes.APPLICATION_M3U8).build())
}

/** A music file of the phone, as it is, from its address in the media index. */
@UnstableApi
fun localSource(context: android.content.Context, uri: String, item: MediaItem.Builder): MediaSource =
    ProgressiveMediaSource.Factory(androidx.media3.datasource.DefaultDataSource.Factory(context))
        .createMediaSource(item.setUri(uri).build())

/** A song on the user's server: its file as it is (`format=raw`), with no transcoding. */
@UnstableApi
fun navidromeSource(url: String, item: MediaItem.Builder): MediaSource =
    ProgressiveMediaSource.Factory(Http.sources).createMediaSource(item.setUri(url).build())
