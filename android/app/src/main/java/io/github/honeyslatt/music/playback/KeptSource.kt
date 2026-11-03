package io.github.honeyslatt.music.playback

import android.net.Uri
import androidx.media3.common.C
import androidx.media3.common.MediaItem
import androidx.media3.common.util.UnstableApi
import androidx.media3.datasource.BaseDataSource
import androidx.media3.datasource.DataSpec
import androidx.media3.exoplayer.source.MediaSource
import androidx.media3.exoplayer.source.ProgressiveMediaSource
import io.github.honeyslatt.music.core.KeptAudio

/**
 * A song downloaded: played from its sealed file, unsealed by the core as
 * the player reads; the encoder's priming of an Apple song left out, as
 * when it streams, so that songs follow without a gap.
 */
@UnstableApi
fun keptSource(audio: KeptAudio, item: MediaItem.Builder): MediaSource {
    val priming = audio.priming().toInt()
    val extractors = androidx.media3.extractor.ExtractorsFactory {
        androidx.media3.extractor.DefaultExtractorsFactory().createExtractors().map { if (priming > 0) Primed(it, priming) else it }.toTypedArray()
    }
    return ProgressiveMediaSource.Factory({ KeptDataSource(audio) }, extractors)
        .createMediaSource(item.setUri("music-kept://song.${audio.extension()}").build())
}

/** An extractor whose audio says the encoder's `priming`, which the file doesn't. */
@UnstableApi
private class Primed(private val inner: androidx.media3.extractor.Extractor, private val priming: Int) :
    androidx.media3.extractor.Extractor by inner {
    override fun init(output: androidx.media3.extractor.ExtractorOutput) {
        inner.init(object : androidx.media3.extractor.ExtractorOutput by output {
            override fun track(id: Int, type: Int): androidx.media3.extractor.TrackOutput {
                val track = output.track(id, type)
                return object : androidx.media3.extractor.TrackOutput by track {
                    override fun format(format: androidx.media3.common.Format) {
                        track.format(format.buildUpon().setEncoderDelay(priming).build())
                    }
                }
            }
        })
    }
}

/** Reads the kept song a chunk at a time, as the core unseals it. */
@UnstableApi
private class KeptDataSource(private val audio: KeptAudio) : BaseDataSource(false) {
    private var uri: Uri? = null
    private var position = 0L
    private var left = 0L
    private var chunk = ByteArray(0)
    private var at = 0

    override fun open(spec: DataSpec): Long {
        uri = spec.uri
        transferInitializing(spec)
        val len = audio.len().toLong()
        position = spec.position.coerceAtMost(len)
        left = if (spec.length == C.LENGTH_UNSET.toLong()) len - position else spec.length.coerceAtMost(len - position)
        chunk = ByteArray(0)
        at = 0
        transferStarted(spec)
        return left
    }

    override fun read(buffer: ByteArray, offset: Int, length: Int): Int {
        if (length == 0) return 0
        if (left == 0L) return C.RESULT_END_OF_INPUT
        if (at == chunk.size) {
            chunk = audio.read(position.toULong(), CHUNK.toUInt())
            at = 0
            if (chunk.isEmpty()) return C.RESULT_END_OF_INPUT
        }
        val n = minOf(length.toLong(), (chunk.size - at).toLong(), left).toInt()
        System.arraycopy(chunk, at, buffer, offset, n)
        at += n
        position += n
        left -= n
        bytesTransferred(n)
        return n
    }

    override fun getUri() = uri

    override fun close() {
        if (uri != null) transferEnded()
        uri = null
    }

    private companion object {
        /** As the store seals them. */
        const val CHUNK = 64 * 1024
    }
}
