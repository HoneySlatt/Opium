package io.github.honeyslatt.music.ui.chrome

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.navigationBars
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.windowInsetsPadding
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.BasicText
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.lerp
import androidx.compose.ui.text.SpanStyle
import androidx.compose.ui.text.buildAnnotatedString
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.text.withStyle
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import io.github.honeyslatt.music.core.model.FavouriteKind
import io.github.honeyslatt.music.core.model.MiniPlayer as MiniStyle
import io.github.honeyslatt.music.playback.Playback
import io.github.honeyslatt.music.ui.Icons
import io.github.honeyslatt.music.ui.MoreIcons
import io.github.honeyslatt.music.ui.actions.LocalActions
import io.github.honeyslatt.music.ui.components.Cover
import io.github.honeyslatt.music.ui.components.Glyph
import io.github.honeyslatt.music.ui.components.anchor
import io.github.honeyslatt.music.ui.components.press
import io.github.honeyslatt.music.ui.components.rememberAnchor
import io.github.honeyslatt.music.ui.detail.badge
import io.github.honeyslatt.music.ui.detail.rememberPageColour
import io.github.honeyslatt.music.ui.nav.Navigator
import io.github.honeyslatt.music.ui.nav.Tab
import io.github.honeyslatt.music.ui.nowplaying.rememberPosition
import io.github.honeyslatt.music.ui.theme.geist
import io.github.honeyslatt.music.ui.theme.look
import io.github.honeyslatt.music.ui.theme.roles

/** Spotify's three tabs: Home (New and Radio as its chips), Search and Your Library. */
val SpotifyTabs = listOf(Tab.Home, Tab.Search, Tab.Library)

/**
 * Spotify's bottom: the player as a card in the cover's colour, then the
 * three tabs on a shade darkening into black.
 */
@androidx.annotation.OptIn(androidx.media3.common.util.UnstableApi::class)
@Composable
fun SpotifyBottom(nav: Navigator, mini: MiniState, playback: Playback, onDevices: () -> Unit, modifier: Modifier = Modifier) {
    Column(
        modifier
            .fillMaxWidth()
            .background(Brush.verticalGradient(0f to Color.Transparent, 0.35f to Color.Black.copy(alpha = 0.7f), 1f to Color.Black))
            .windowInsetsPadding(WindowInsets.navigationBars),
    ) {
        if (look.mobile.miniPlayer == MiniStyle.CARD) {
            MiniCard(mini, playback, onDevices, Modifier.padding(horizontal = 8.dp))
        } else {
            MiniPlayer(mini, folded = false, Modifier.padding(horizontal = 14.dp).fillMaxWidth())
        }
        Row(Modifier.fillMaxWidth().height(62.dp).padding(top = 6.dp)) {
            SpotifyTabs.forEach { tab ->
                val on = nav.tab == tab
                val color = if (on) roles.text else roles.sub
                Column(
                    Modifier.weight(1f).fillMaxHeight().press(RoundedCornerShape(12.dp), shrink = 0.94f) { nav.select(tab) },
                    horizontalAlignment = Alignment.CenterHorizontally,
                    verticalArrangement = Arrangement.spacedBy(4.dp, Alignment.Top),
                ) {
                    Glyph(if (tab == Tab.Library) MoreIcons.Shelf else tab.icon, color, 25.dp)
                    BasicText(
                        if (tab == Tab.Library) "Your Library" else tab.label,
                        style = geist(11.sp, if (on) FontWeight.Medium else FontWeight.Normal).copy(color = color),
                    )
                }
            }
        }
    }
}

/**
 * Spotify's player: a card in the cover's colour with the song, where it
 * plays and its quality, the devices, the saved mark and Play; the
 * position as a line along its foot.
 */
@androidx.annotation.OptIn(androidx.media3.common.util.UnstableApi::class)
@Composable
fun MiniCard(state: MiniState, playback: Playback, onDevices: () -> Unit, modifier: Modifier = Modifier) {
    val track = state.track ?: return
    val actions = LocalActions.current
    val remote by playback.remote.collectAsStateWithLifecycle()
    val anchor = rememberAnchor()
    val shape = RoundedCornerShape(8.dp)
    // The cover's colour, darker, as Spotify's.
    val fill = lerp(rememberPageColour(track.artwork), Color.Black, 0.35f)
    val green = roles.accent
    val quality = playback.quality(track)?.badge()
    Box(
        modifier
            .fillMaxWidth()
            .height(60.dp)
            .anchor(anchor)
            .clip(shape)
            .background(fill)
            .press(shape, onLongClick = { actions?.trackMenu(track, anchor.rect) }, onClick = state.onOpen),
    ) {
        Row(
            Modifier.fillMaxWidth().height(56.dp).padding(start = 8.dp, end = 4.dp),
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.spacedBy(10.dp),
        ) {
            Cover(track.artwork, 40.dp, RoundedCornerShape(4.dp), Modifier.size(40.dp))
            Column(Modifier.weight(1f)) {
                val artists = track.artists.joinToString(", ")
                val device = remote?.device?.name
                // Elsewhere, the artists join the title and the device takes
                // their line, in the accent.
                BasicText(
                    buildAnnotatedString {
                        withStyle(SpanStyle(fontWeight = FontWeight.Bold)) { append(track.title) }
                        if (device != null) withStyle(SpanStyle(color = Color.White.copy(alpha = 0.7f))) { append(" • $artists") }
                    },
                    style = geist(14.sp).copy(color = Color.White),
                    maxLines = 1,
                    overflow = TextOverflow.Ellipsis,
                )
                BasicText(
                    buildAnnotatedString {
                        if (device != null) {
                            withStyle(SpanStyle(color = green)) { append(device) }
                        } else {
                            append(artists)
                        }
                        if (quality != null) withStyle(SpanStyle(color = green)) { append(" • $quality") }
                    },
                    style = geist(13.sp).copy(color = Color.White.copy(alpha = 0.7f)),
                    maxLines = 1,
                    overflow = TextOverflow.Ellipsis,
                )
            }
            Box(
                Modifier.size(40.dp).press(CircleShape, shrink = 0.9f, onClick = onDevices),
                contentAlignment = Alignment.Center,
            ) { Glyph(MoreIcons.Devices, if (remote != null) green else Color.White, 22.dp) }
            if (!playback.isStation(track)) {
                val saved = actions?.isFavourite(FavouriteKind.SONG, track.id) == true
                Box(
                    Modifier.size(40.dp).press(CircleShape, shrink = 0.9f) { actions?.toggleFavourite(track) },
                    contentAlignment = Alignment.Center,
                ) {
                    if (saved) {
                        Box(Modifier.size(22.dp).background(green, CircleShape), contentAlignment = Alignment.Center) {
                            Glyph(Icons.Check, roles.onAccent, 14.dp)
                        }
                    } else {
                        Glyph(Icons.Pluscircle, Color.White, 24.dp)
                    }
                }
            }
            Box(Modifier.size(40.dp).press(CircleShape, shrink = 0.9f, onClick = state.onToggle), contentAlignment = Alignment.Center) {
                if (state.loading) Spinner() else Glyph(if (state.playing) Icons.Pause else Icons.Play, Color.White, 24.dp)
            }
        }
        // The position along the foot.
        val position by rememberPosition(playback, state.playing, everyMs = 500)
        val duration = playback.duration ?: 0L
        val done = if (duration > 0) (position.toFloat() / duration).coerceIn(0f, 1f) else 0f
        Box(
            Modifier.align(Alignment.BottomStart).padding(horizontal = 8.dp).fillMaxWidth().height(2.dp)
                .background(Color.White.copy(alpha = 0.25f), RoundedCornerShape(1.dp)),
        ) {
            Box(Modifier.fillMaxWidth(done).height(2.dp).background(Color.White, RoundedCornerShape(1.dp)))
        }
    }
}
