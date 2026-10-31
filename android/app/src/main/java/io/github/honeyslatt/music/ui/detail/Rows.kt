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
import io.github.honeyslatt.music.ui.components.Bars
import io.github.honeyslatt.music.ui.components.anchor
import io.github.honeyslatt.music.ui.components.rememberAnchor
import io.github.honeyslatt.music.ui.actions.LocalActions
import io.github.honeyslatt.music.ui.components.ExplicitBadge
import io.github.honeyslatt.music.ui.components.LocalNowPlaying
import io.github.honeyslatt.music.ui.components.Glyph
import io.github.honeyslatt.music.ui.components.Line
import io.github.honeyslatt.music.ui.components.press
import io.github.honeyslatt.music.ui.components.underline
import io.github.honeyslatt.music.ui.theme.geist
import io.github.honeyslatt.music.ui.theme.roles

/** A song of an album: its number, its title, the E, its menu. */
@Composable
fun AlbumTrackRow(number: Int, track: Track, last: Boolean, onClick: () -> Unit = {}) {
    val mark = LocalNowPlaying.current
    val now = mark.id == track.id
    val actions = LocalActions.current
    val anchor = rememberAnchor()
    val more = rememberAnchor()
    if (io.github.honeyslatt.music.ui.theme.look.layout.rows == io.github.honeyslatt.music.core.model.Rows.STACKED) {
        // Spotify's: no number nor hairline, the E and the artists under
        // the title, the menu's dots upright.
        Row(
            Modifier
                .fillMaxWidth()
                .heightIn(min = 60.dp)
                .anchor(anchor)
                .press(onLongClick = { actions?.trackMenu(track, anchor.rect) }, onClick = onClick)
                .padding(start = 16.dp, end = 4.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            androidx.compose.foundation.layout.Column(Modifier.weight(1f)) {
                Line(track.title, geist(16.sp).copy(color = if (now) roles.tint else roles.text))
                Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(5.dp)) {
                    if (track.explicit) ExplicitBadge()
                    Line(track.artists.joinToString(", "), geist(13.5.sp).copy(color = roles.sub))
                }
            }
            Box(Modifier.size(44.dp).anchor(more).press { actions?.trackMenu(track, more.rect) }, contentAlignment = Alignment.Center) {
                Glyph(io.github.honeyslatt.music.ui.MoreIcons.MoreVertical, roles.sub, 20.dp)
            }
        }
        return
    }
    Row(
        Modifier
            .fillMaxWidth()
            .heightIn(min = 52.dp)
            .anchor(anchor)
            .press(onLongClick = { actions?.trackMenu(track, anchor.rect) }, onClick = onClick)
            .padding(start = 20.dp),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        Box(Modifier.width(22.dp), contentAlignment = Alignment.Center) {
            if (now) {
                Bars(roles.tint, mark.playing)
            } else {
                BasicText(number.toString(), style = geist(15.sp).copy(color = roles.sub, textAlign = TextAlign.Center))
            }
        }
        Row(
            Modifier.weight(1f).heightIn(min = 52.dp).then(if (last) Modifier else Modifier.underline(roles.line)),
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.spacedBy(6.dp),
        ) {
            Row(Modifier.weight(1f), verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(6.dp)) {
                Line(track.title, geist(16.sp).copy(color = if (now) roles.tint else roles.text), Modifier.weight(1f, fill = false))
                if (track.explicit) ExplicitBadge()
            }
            Box(Modifier.padding(end = 6.dp).size(44.dp).anchor(more).press { actions?.trackMenu(track, more.rect) }, contentAlignment = Alignment.Center) {
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
