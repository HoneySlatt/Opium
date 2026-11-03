package io.github.honeyslatt.music.ui.components

import androidx.compose.foundation.Canvas
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.runtime.Composable
import androidx.compose.runtime.compositionLocalOf
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import io.github.honeyslatt.music.core.DownloadState
import io.github.honeyslatt.music.core.model.ItemId
import io.github.honeyslatt.music.core.model.ProviderKind
import io.github.honeyslatt.music.core.model.Track
import io.github.honeyslatt.music.data.downloadable
import io.github.honeyslatt.music.ui.MoreIcons
import io.github.honeyslatt.music.ui.theme.roles

/** The downloads as the lists show them, and the services out of reach. */
data class DownloadsShown(
    val states: Map<ItemId, DownloadState> = emptyMap(),
    val unreachable: Set<ProviderKind> = emptySet(),
    /** Every service signed in to out of reach: the network gone. */
    val offline: Boolean = false,
) {
    /** How `track` stands as a download (or its library's copy); null: not downloaded. */
    fun state(track: Track): DownloadState? = states[track.id] ?: track.libraryId?.let { states[it] }

    fun kept(track: Track) = state(track) is DownloadState.Kept

    /** Its service out of reach and it not downloaded: it can't play, and shows faded. */
    fun unplayable(track: Track) = track.id.provider in unreachable && !kept(track)
}

val LocalDownloads = compositionLocalOf { DownloadsShown() }

/** What can't play shows at 38 %. */
const val FADED = 0.38f

private fun DownloadState.coming() = this is DownloadState.Queued || this is DownloadState.Coming

/** A page's songs as a download: offered, coming (how much of it), or all kept; null when none can be. */
sealed interface PageDownload {
    data object Offer : PageDownload
    data class Coming(val part: Float) : PageDownload
    data object Kept : PageDownload
}

fun DownloadsShown.page(tracks: List<Track>): PageDownload? {
    val own = tracks.filter(::downloadable)
    if (own.isEmpty()) return null
    if (own.all(::kept)) return PageDownload.Kept
    if (own.none { state(it)?.coming() == true }) return PageDownload.Offer
    val done = own.sumOf { t ->
        when (val s = state(t)) {
            is DownloadState.Kept -> 1.0
            is DownloadState.Coming -> s.part.toDouble()
            else -> 0.0
        }
    }
    return PageDownload.Coming((done / own.size).toFloat())
}

/**
 * A song's mark as a download, 14 dp: a ring filling while it comes (grey
 * while it waits), an arrow on a disc once kept (grey when its subscription
 * went unconfirmed), a red ring when it failed. Nothing when not downloaded.
 */
@Composable
fun DownloadBadge(track: Track, playing: Boolean = false) {
    val state = LocalDownloads.current.state(track) ?: return
    val size = 14.dp
    when (state) {
        is DownloadState.Kept -> Disc(size, if (playing) roles.text else roles.accent, if (playing) roles.base else roles.onAccent)
        DownloadState.Expired -> Disc(size, roles.muted, roles.base)
        is DownloadState.Failed -> Box(Modifier.size(size), contentAlignment = Alignment.Center) {
            Ring(size, 1.5.dp, roles.red, null, 0f)
            Glyph(MoreIcons.ArrowDownHeavy, roles.red, 9.dp)
        }
        DownloadState.Queued -> Ring(size, 1.5.dp, roles.muted, null, 0f)
        is DownloadState.Coming -> Ring(size, 1.5.dp, roles.muted.copy(alpha = 0.5f), roles.accent, state.part)
        DownloadState.Gone -> {}
    }
}

@Composable
private fun Disc(size: Dp, fill: Color, arrow: Color) {
    Box(Modifier.size(size).background(fill, CircleShape), contentAlignment = Alignment.Center) {
        Glyph(MoreIcons.ArrowDownHeavy, arrow, size * 0.65f)
    }
}

/** A ring `width` wide, `part` of it from the top in `filled`. */
@Composable
private fun Ring(size: Dp, width: Dp, track: Color, filled: Color?, part: Float) {
    Canvas(Modifier.size(size)) {
        val w = width.toPx()
        val inset = Offset(w / 2, w / 2)
        val arc = Size(this.size.width - w, this.size.height - w)
        drawArc(track, 0f, 360f, false, inset, arc, style = Stroke(w))
        if (filled != null) drawArc(filled, -90f, 360f * part.coerceIn(0.02f, 1f), false, inset, arc, style = Stroke(w))
    }
}

/**
 * A page's download button, as its header draws one: an arrow in a ring
 * (Download); the ring filling with a stop square while it comes (Stop
 * downloading); all kept, an arrow on a disc (Remove Download).
 */
@Composable
fun DownloadGlyph(state: PageDownload, size: Dp = 26.dp, idle: Color = roles.text) {
    when (state) {
        PageDownload.Offer -> Box(Modifier.size(size), contentAlignment = Alignment.Center) {
            Ring(size, 2.dp, idle, null, 0f)
            Glyph(MoreIcons.ArrowDownHeavy, idle, size * 0.54f)
        }
        is PageDownload.Coming -> Box(Modifier.size(size), contentAlignment = Alignment.Center) {
            Ring(size, 2.dp, idle.copy(alpha = 0.35f), roles.accent, state.part)
            Box(Modifier.size(size * 0.34f).background(roles.accent, RoundedCornerShape(2.dp)))
        }
        PageDownload.Kept -> Disc(size, roles.accent, roles.onAccent)
    }
}
