package io.github.honeyslatt.music.ui.detail

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.text.BasicText
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import io.github.honeyslatt.music.core.model.Track
import io.github.honeyslatt.music.ui.Icons
import io.github.honeyslatt.music.ui.components.ExplicitBadge
import io.github.honeyslatt.music.ui.components.Glyph
import io.github.honeyslatt.music.ui.components.Line
import io.github.honeyslatt.music.ui.components.press
import io.github.honeyslatt.music.ui.components.underline
import io.github.honeyslatt.music.ui.theme.geist
import io.github.honeyslatt.music.ui.theme.roles

/** A song of an album: its number, its title, the E, its menu. */
@Composable
fun AlbumTrackRow(number: Int, track: Track, last: Boolean, onLongClick: () -> Unit = {}, onClick: () -> Unit = {}) {
    Row(
        Modifier.fillMaxWidth().heightIn(min = 52.dp).press(onLongClick = onLongClick, onClick = onClick).padding(start = 20.dp),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        BasicText(
            number.toString(),
            Modifier.width(22.dp),
            style = geist(15.sp).copy(color = roles.sub, textAlign = TextAlign.Center),
        )
        Row(
            Modifier.weight(1f).heightIn(min = 52.dp).then(if (last) Modifier else Modifier.underline(roles.line)),
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.spacedBy(6.dp),
        ) {
            Row(Modifier.weight(1f), verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(6.dp)) {
                Line(track.title, geist(16.sp).copy(color = roles.text), Modifier.weight(1f, fill = false))
                if (track.explicit) ExplicitBadge()
            }
            Box(Modifier.padding(end = 6.dp).size(44.dp).press(onClick = onLongClick), contentAlignment = Alignment.Center) {
                Glyph(Icons.More, roles.text, 20.dp)
            }
        }
    }
}

/** The note under a list: a date, a count, a copyright. */
@Composable
fun Foot(lines: List<String>) {
    if (lines.isEmpty()) return
    BasicText(
        lines.joinToString("\n"),
        Modifier.padding(start = 20.dp, end = 20.dp, top = 16.dp),
        style = geist(13.5.sp).copy(color = roles.sub, lineHeight = 20.sp),
    )
}

/** `N songs, M minutes`. */
fun countAndLength(tracks: List<Track>): String {
    val seconds = tracks.sumOf { it.duration?.seconds ?: 0L }
    val minutes = ((seconds + 30) / 60).toInt()
    val songs = if (tracks.size == 1) "1 song" else "${tracks.size} songs"
    return when {
        minutes >= 60 -> "$songs, ${minutes / 60} h ${minutes % 60} min"
        else -> "$songs, $minutes ${if (minutes == 1) "minute" else "minutes"}"
    }
}

/** `2025-03-14` as `14 March 2025`; a year alone as it is. */
fun releaseDate(released: String?): String? {
    released ?: return null
    return runCatching {
        java.time.LocalDate.parse(released)
            .format(java.time.format.DateTimeFormatter.ofPattern("d MMMM yyyy", java.util.Locale.ENGLISH))
    }.getOrDefault(released)
}
