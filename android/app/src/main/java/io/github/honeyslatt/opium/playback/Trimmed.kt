package io.github.honeyslatt.opium.playback

import android.net.Uri
import androidx.media3.common.Format
import androidx.media3.common.util.TimestampAdjuster
import androidx.media3.common.util.UnstableApi
import androidx.media3.exoplayer.analytics.PlayerId
import androidx.media3.exoplayer.hls.DefaultHlsExtractorFactory
import androidx.media3.exoplayer.hls.HlsExtractorFactory
import androidx.media3.exoplayer.hls.HlsMediaChunkExtractor
import androidx.media3.extractor.ExtractorInput
import androidx.media3.extractor.ExtractorOutput
import androidx.media3.extractor.TrackOutput

/**
 * An HLS song's audio with the encoder's priming and padding said, which
 * Apple's files leave out: the player then drops them, and songs meant to
 * run into each other do so without a gap.
 */
@UnstableApi
class Trimmed(
    private val inner: HlsMediaChunkExtractor,
    private val priming: Int,
    private val padding: () -> Int,
) : HlsMediaChunkExtractor by inner {

    override fun init(output: ExtractorOutput) {
        inner.init(object : ExtractorOutput by output {
            override fun track(id: Int, type: Int): TrackOutput {
                val track = output.track(id, type)
                return object : TrackOutput by track {
                    override fun format(format: Format) {
                        track.format(
                            format.buildUpon()
                                .setEncoderDelay(priming)
                                .setEncoderPadding(padding())
                                .build(),
                        )
                    }
                }
            }
        })
    }

    override fun recreate(): HlsMediaChunkExtractor = Trimmed(inner.recreate(), priming, padding)

    /** `padding` is asked for when the audio's format is read, by then known. */
    class Factory(private val priming: Int, private val padding: () -> Int) : HlsExtractorFactory {
        private val default = DefaultHlsExtractorFactory()

        override fun createExtractor(
            uri: Uri,
            format: Format,
            muxedCaptionFormats: List<Format>?,
            timestampAdjuster: TimestampAdjuster,
            responseHeaders: Map<String, List<String>>,
            sniffingExtractorInput: ExtractorInput,
            playerId: PlayerId,
        ): HlsMediaChunkExtractor = Trimmed(
            default.createExtractor(
                uri, format, muxedCaptionFormats, timestampAdjuster, responseHeaders,
                sniffingExtractorInput, playerId,
            ),
            priming,
            padding,
        )
    }
}
