package io.github.honeyslatt.music.ui.nowplaying

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.BasicText
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.drawBehind
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.media3.common.C
import androidx.media3.common.MimeTypes
import androidx.media3.common.util.UnstableApi
import io.github.honeyslatt.music.core.model.ProviderKind
import io.github.honeyslatt.music.core.model.Track
import io.github.honeyslatt.music.playback.Playback
import io.github.honeyslatt.music.ui.Icons
import io.github.honeyslatt.music.ui.components.Glyph
import io.github.honeyslatt.music.ui.components.Group
import io.github.honeyslatt.music.ui.components.ServiceLogo
import io.github.honeyslatt.music.ui.components.label
import io.github.honeyslatt.music.ui.detail.label
import io.github.honeyslatt.music.ui.components.sheetCell
import io.github.honeyslatt.music.ui.theme.geist
import io.github.honeyslatt.music.ui.theme.roles

/** `44100` as `44.1 kHz`. */
private fun khz(hz: Int) = (hz / 1000.0).let { if (it % 1.0 == 0.0) "${it.toInt()} kHz" else "$it kHz" }

private fun pcm(encoding: Int) = when (encoding) {
    C.ENCODING_PCM_16BIT -> "PCM 16-bit"
    C.ENCODING_PCM_24BIT, C.ENCODING_PCM_24BIT_BIG_ENDIAN -> "PCM 24-bit"
    C.ENCODING_PCM_32BIT, C.ENCODING_PCM_32BIT_BIG_ENDIAN -> "PCM 32-bit"
    C.ENCODING_PCM_FLOAT -> "PCM float"
    else -> "PCM"
}

private fun codec(mime: String?) = when (mime) {
    MimeTypes.AUDIO_AAC -> "AAC"
    MimeTypes.AUDIO_FLAC -> "FLAC"
    MimeTypes.AUDIO_ALAC -> "ALAC"
    MimeTypes.AUDIO_MPEG -> "MP3"
    MimeTypes.AUDIO_OPUS -> "Opus"
    MimeTypes.AUDIO_VORBIS -> "Vorbis"
    MimeTypes.AUDIO_RAW, MimeTypes.AUDIO_WAV -> "PCM"
    else -> mime?.substringAfter('/')?.uppercase()
}

/**
 * The way from the service to the ears, as the player and Android report
 * it. Nothing here is bit-perfect yet: Android's mixer takes every sound,
 * resamples it to its own rate and applies the volume.
 */
@androidx.annotation.OptIn(UnstableApi::class)
@Composable
fun SignalPath(playback: Playback, track: Track) {
    val context = LocalContext.current
    val signal by playback.signal.collectAsStateWithLifecycle()
    val device = remember { Outputs.current(context) }
    val mixer = remember { Outputs.mixerRate(context) }
    val volume = remember { Outputs.volume(context) }
    val input = signal.input
    val rate = input?.sampleRate?.takeIf { it > 0 } ?: track.quality?.sampleRate?.toInt()
    val channels = input?.channelCount?.takeIf { it > 0 }
    val resampled = rate != null && mixer != null && rate != mixer
    val bluetooth = device.kind == OutputKind.Bluetooth

    val reason = when {
        bluetooth -> "Bluetooth re-encodes the music for ${device.name}."
        resampled -> "Android resamples ${khz(rate!!)} to ${khz(mixer!!)} and mixes it with other apps."
        else -> "Android mixes it with other apps and applies the volume."
    }
    Status("Not bit-perfect", reason)

    Group(
        "A USB DAC can receive the samples unchanged: Android 14 and later have a bit-perfect mode " +
            "for USB audio, which Music doesn't use yet. Over Bluetooth or in the car, a lossless file " +
            "still sounds better than a stream, but it is not bit-perfect.",
    ) {
        val provider = track.id.provider
        val source = playback.source(track)
        val original = source == Playback.Source.Original
        // An original's quality is the file's: what the service listed is its stream's.
        val listed = playback.quality(track).takeIf { !original }
        val format = listOfNotNull(
            codec(input?.sampleMimeType) ?: listed?.label(),
            rate?.let(::khz),
            channels?.let { if (it == 1) "mono" else "$it ch" },
            (listed?.bitrateKbps?.toInt()?.times(1000) ?: input?.averageBitrate?.takeIf { it > 0 })?.let { "${it / 1000} kb/s" },
        ).joinToString(" · ")
        val transport = when {
            provider == ProviderKind.APPLE_MUSIC -> "Streamed over HLS, decrypted by Widevine"
            original -> "The uploader's own file, as it is"
            source == Playback.Source.Encrypted -> "Streamed over HLS, decrypted by Widevine"
            provider == ProviderKind.SOUND_CLOUD -> "Streamed over HLS"
            else -> "Your server's file, as it is"
        }
        val name = if (original) "Original" else listed?.label() ?: codec(input?.sampleMimeType) ?: ""
        Link({ ServiceLogo(provider, 20.dp) }, "${provider.label()} · $name", "$format\n$transport", first = true)

        val out = signal.output
        val decoded = listOfNotNull(
            out?.let { pcm(it.encoding) } ?: "PCM",
            (out?.sampleRate ?: rate)?.let(::khz),
            channels?.let { if (it == 1) "mono" else "$it ch" },
        ).joinToString(" · ")
        val decoder = when {
            out?.offload == true -> "Decoded by the phone's audio processor"
            signal.decoder != null -> signal.decoder!!
            else -> "Android's decoder"
        }
        Link({ Glyph(Icons.Cpu, roles.sub, 17.dp) }, "Decoder", "$decoder\n$decoded, samples untouched")
        Link({ Glyph(Icons.Vhigh, roles.sub, 17.dp) }, "Volume", "Media volume ${(volume * 100).toInt()} %")
        Link(
            { Glyph(Icons.Mix, roles.yellow, 17.dp) },
            "Android audio",
            buildString {
                append("Mixed with other apps")
                append(
                    when {
                        resampled -> " · resampled to ${khz(mixer!!)}"
                        mixer != null -> " · at ${khz(mixer)}"
                        else -> ""
                    },
                )
            },
        )
        Link(
            { Glyph(if (bluetooth) Icons.Bt else device.icon, roles.yellow, 17.dp) },
            device.name,
            when (device.kind) {
                OutputKind.Speaker -> "Built-in speaker"
                OutputKind.Bluetooth -> "Bluetooth, re-encoded by the phone in the codec it agreed with the device"
                OutputKind.Usb -> "USB audio, after Android's mixer"
                OutputKind.Wired -> "Wired headphones"
                OutputKind.Other -> "Android's output"
            },
        )
    }
}

@Composable
private fun Status(title: String, reason: String, ok: Boolean = false) {
    val colour = if (ok) roles.green else roles.yellow
    Row(
        Modifier.padding(bottom = 22.dp).fillMaxWidth().background(sheetCell, RoundedCornerShape(16.dp)).padding(horizontal = 16.dp, vertical = 14.dp),
        horizontalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        Box(Modifier.size(34.dp).background(colour.copy(alpha = 0.22f), CircleShape), contentAlignment = Alignment.Center) {
            Glyph(if (ok) Icons.Check else Icons.Alert, colour, 18.dp)
        }
        Column {
            BasicText(title, style = geist(16.sp, FontWeight.SemiBold).copy(color = roles.text))
            BasicText(reason, style = geist(13.5.sp).copy(color = roles.sub, lineHeight = 19.sp))
        }
    }
}

/** One link of the chain: its icon on a tile, what it is, what it does. */
@Composable
private fun Link(icon: @Composable () -> Unit, title: String, detail: String, first: Boolean = false) {
    val line = roles.line
    Row(
        Modifier
            .fillMaxWidth()
            .heightIn(min = 52.dp)
            // The hairline between links starts after the icon.
            .drawBehind { if (!first) drawLine(line, Offset(58.dp.toPx(), 0f), Offset(size.width, 0f), 1.dp.toPx()) }
            .padding(horizontal = 16.dp, vertical = 10.dp),
        horizontalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        Box(Modifier.padding(top = 2.dp).size(30.dp).background(roles.surface2, RoundedCornerShape(8.dp)), contentAlignment = Alignment.Center) { icon() }
        Column(Modifier.weight(1f)) {
            BasicText(title, style = geist(16.sp, FontWeight.Medium).copy(color = roles.text))
            BasicText(detail, Modifier.padding(top = 1.dp), style = geist(13.sp).copy(color = roles.sub, lineHeight = 17.5.sp))
        }
    }
}
