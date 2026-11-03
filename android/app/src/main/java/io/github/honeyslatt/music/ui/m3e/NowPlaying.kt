package io.github.honeyslatt.music.ui.m3e

import androidx.compose.animation.core.animateDpAsState
import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.interaction.collectIsPressedAsState
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.safeDrawing
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.layout.windowInsetsPadding
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.BasicText
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.produceState
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import io.github.honeyslatt.music.core.QueueRepeat
import io.github.honeyslatt.music.core.model.FavouriteKind
import io.github.honeyslatt.music.core.model.Track
import io.github.honeyslatt.music.playback.Playback
import io.github.honeyslatt.music.ui.Icons
import io.github.honeyslatt.music.ui.MoreIcons
import io.github.honeyslatt.music.ui.actions.LocalActions
import io.github.honeyslatt.music.ui.chrome.ShapedPlay
import io.github.honeyslatt.music.ui.components.Cover
import io.github.honeyslatt.music.ui.components.ExplicitBadge
import io.github.honeyslatt.music.ui.components.Glyph
import io.github.honeyslatt.music.ui.components.Line
import io.github.honeyslatt.music.ui.components.Sheet
import io.github.honeyslatt.music.ui.components.anchor
import io.github.honeyslatt.music.ui.components.press
import io.github.honeyslatt.music.ui.components.rememberAnchor
import io.github.honeyslatt.music.ui.detail.badge
import io.github.honeyslatt.music.ui.nowplaying.Backdrop
import io.github.honeyslatt.music.ui.nowplaying.DevicesSheet
import io.github.honeyslatt.music.ui.nowplaying.LocalNp
import io.github.honeyslatt.music.ui.nowplaying.Lyrics
import io.github.honeyslatt.music.ui.nowplaying.NpActions
import io.github.honeyslatt.music.ui.nowplaying.NpColours
import io.github.honeyslatt.music.ui.nowplaying.NpMode
import io.github.honeyslatt.music.ui.nowplaying.QueueView
import io.github.honeyslatt.music.ui.nowplaying.SignalPath
import io.github.honeyslatt.music.ui.nowplaying.Wash
import io.github.honeyslatt.music.ui.nowplaying.clock
import io.github.honeyslatt.music.ui.nowplaying.rememberPosition
import io.github.honeyslatt.music.ui.theme.geist
import io.github.honeyslatt.music.ui.theme.roles

/**
 * M3E's Now Playing: over the cover's drifting colours, the chevron, the
 * connected tabs Playing · Lyrics · Queue and More. Playing: the cover at
 * radius 48 (smaller and rounder paused), the title, the tonal heart, the
 * large wavy position, the times about the quality, the button group about
 * the 88 dp shaped Play, the output. An album with an animated cover: it
 * fills the screen behind instead, as on Stock.
 */
@androidx.annotation.OptIn(androidx.media3.common.util.UnstableApi::class)
@Composable
fun ExpressiveNowPlaying(playback: Playback, actions: NpActions) {
    val track by playback.now.collectAsStateWithLifecycle()
    val playing by playback.playing.collectAsStateWithLifecycle()
    val loading by playback.loading.collectAsStateWithLifecycle()
    val t = track ?: return
    var mode by remember { mutableStateOf(NpMode.Artwork) }
    var signalOpen by remember { mutableStateOf(false) }
    var devicesOpen by remember { mutableStateOf(false) }
    val motion by produceState<String?>(null, t.albumId) {
        // Another album: not the last one's video meanwhile.
        value = null
        value = actions.motion(t)
    }
    val m = motion
    // Over the moving cover, as Stock's, the text is white in both appearances.
    val over = if (m != null) roles.copy(base = Color.Black, text = Color.White, sub = Color.White.copy(alpha = 0.7f)) else roles
    val np = NpColours(over.text, over.sub, over.text.copy(alpha = 0.28f), over.text.copy(alpha = 0.1f))
    CompositionLocalProvider(LocalNp provides np) {
        Box(Modifier.fillMaxSize().background(roles.base)) {
            if (m != null) Backdrop(t, m, flat = mode != NpMode.Artwork) else Wash(t, playing)
            CompositionLocalProvider(io.github.honeyslatt.music.ui.theme.LocalRoles provides over) {
                Column(Modifier.fillMaxSize().windowInsetsPadding(WindowInsets.safeDrawing).padding(horizontal = 24.dp, vertical = 8.dp)) {
                    Top(t, mode, actions) { mode = it }
                    when (mode) {
                        NpMode.Artwork -> Playing(playback, t, playing, loading, actions, moving = m != null, { signalOpen = true }, { devicesOpen = true })
                        NpMode.Lyrics -> {
                            Box(Modifier.weight(1f).fillMaxWidth()) { Lyrics(playback, t, false) {} }
                            Docked(playback, t, playing, loading)
                        }
                        NpMode.Queue -> Box(Modifier.weight(1f).fillMaxWidth().padding(top = 12.dp)) { io.github.honeyslatt.music.ui.nowplaying.ExpressiveQueueView(playback) }
                    }
                }
            }
            Sheet(signalOpen, "Signal Path", onClose = { signalOpen = false }) { SignalPath(playback, t) }
            DevicesSheet(devicesOpen, playback) { devicesOpen = false }
        }
    }
}

/** The chevron that closes, the connected tabs, More. */
@Composable
private fun Top(t: Track, mode: NpMode, actions: NpActions, onMode: (NpMode) -> Unit) {
    val menu = LocalActions.current
    val more = rememberAnchor()
    Row(Modifier.fillMaxWidth().height(56.dp), verticalAlignment = Alignment.CenterVertically) {
        Box(Modifier.size(48.dp).press(CircleShape, onClick = actions.close), contentAlignment = Alignment.Center) {
            Glyph(Icons.Down, roles.text, 26.dp)
        }
        Row(Modifier.weight(1f), horizontalArrangement = Arrangement.spacedBy(2.dp, Alignment.CenterHorizontally)) {
            val tabs = listOf(NpMode.Artwork to "Playing", NpMode.Lyrics to "Lyrics", NpMode.Queue to "Queue")
            tabs.forEachIndexed { i, (m, label) ->
                val on = m == mode
                val outer = 20.dp
                val inner = if (on) outer else 8.dp
                val shape = RoundedCornerShape(
                    topStart = if (i == 0 || on) outer else inner,
                    bottomStart = if (i == 0 || on) outer else inner,
                    topEnd = if (i == tabs.lastIndex || on) outer else inner,
                    bottomEnd = if (i == tabs.lastIndex || on) outer else inner,
                )
                Box(
                    Modifier.height(40.dp).press(shape, shrink = 0.96f) { onMode(m) }
                        .background(if (on) roles.text else roles.text.copy(alpha = 0.1f), shape)
                        .padding(horizontal = 14.dp),
                    contentAlignment = Alignment.Center,
                ) { BasicText(label, style = geist(14.sp, FontWeight.SemiBold).copy(color = if (on) roles.base else roles.text)) }
            }
        }
        Box(Modifier.size(48.dp).anchor(more).press(CircleShape) { menu?.trackMenu(t, more.rect, nowPlaying = true) }, contentAlignment = Alignment.Center) {
            Glyph(Icons.More, roles.text, 24.dp)
        }
    }
}

@androidx.annotation.OptIn(androidx.media3.common.util.UnstableApi::class)
@Composable
private fun androidx.compose.foundation.layout.ColumnScope.Playing(
    playback: Playback,
    t: Track,
    playing: Boolean,
    loading: Boolean,
    actions: NpActions,
    moving: Boolean,
    onSignal: () -> Unit,
    onDevices: () -> Unit,
) {
    val menu = LocalActions.current
    // The cover moves behind everything instead.
    if (moving) Spacer(Modifier.weight(1f)) else BoxWithConstraints(Modifier.weight(1f).fillMaxWidth(), contentAlignment = Alignment.Center) {
        val side = minOf(maxWidth, maxHeight)
        val scale by animateFloatAsState(if (playing) 1f else 0.88f, Springs.slow(), label = "np-cover")
        val corner by animateDpAsState(if (playing) 48.dp else 64.dp, Springs.slow(), label = "np-corner")
        Cover(
            t.artwork,
            side,
            RoundedCornerShape(corner),
            Modifier.size(side).graphicsLayer {
                scaleX = scale
                scaleY = scale
            },
        )
    }
    // The title, the artist and album as links, the tonal heart.
    Row(Modifier.fillMaxWidth().padding(top = 20.dp), verticalAlignment = Alignment.CenterVertically) {
        Column(Modifier.weight(1f)) {
            Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(6.dp)) {
                Line(t.title, geist(26.sp, FontWeight.Bold, -0.02f).copy(color = roles.text), Modifier.weight(1f, fill = false))
                if (t.explicit) ExplicitBadge()
            }
            Row(horizontalArrangement = Arrangement.spacedBy(4.dp)) {
                Box(Modifier.press(RoundedCornerShape(6.dp)) { actions.openArtist(t) }) {
                    Line(t.artists.joinToString(", "), geist(16.sp, FontWeight.Medium).copy(color = roles.primary))
                }
                t.album?.let { album ->
                    BasicText("·", style = geist(16.sp).copy(color = roles.sub))
                    Box(Modifier.weight(1f, fill = false).press(RoundedCornerShape(6.dp)) { actions.openAlbum(t) }) {
                        Line(album, geist(16.sp).copy(color = roles.sub))
                    }
                }
            }
        }
        if (!playback.isStation(t)) {
            val saved = menu?.isFavourite(FavouriteKind.SONG, t.id) == true
            Box(
                Modifier.size(48.dp).press(CircleShape) { menu?.toggleFavourite(t) }
                    .background(if (saved) roles.primary else roles.secondaryContainer, CircleShape),
                contentAlignment = Alignment.Center,
            ) { Glyph(if (saved) MoreIcons.HeartFill else MoreIcons.Heart, if (saved) roles.onPrimary else roles.onSecondaryContainer, 22.dp) }
        }
    }
    // The position, the times about the quality.
    val position by rememberPosition(playback, playing, everyMs = 250)
    val duration = playback.duration?.takeIf { it > 0 }
    Box(Modifier.padding(top = 16.dp)) {
        WavyLine(duration?.let { (position.toFloat() / it).coerceIn(0f, 1f) }, playing, big = true) { p ->
            duration?.let { playback.seekTo((p * it).toLong()) }
        }
    }
    Row(Modifier.fillMaxWidth().padding(top = 6.dp), verticalAlignment = Alignment.CenterVertically) {
        BasicText(clock(position), style = geist(13.sp).copy(color = roles.sub, fontFeatureSettings = "tnum"))
        Spacer(Modifier.weight(1f))
        playback.quality(t)?.badge()?.let { q ->
            Box(
                Modifier.press(RoundedCornerShape(8.dp), onClick = onSignal).background(roles.tertiaryContainer, RoundedCornerShape(8.dp))
                    .padding(horizontal = 10.dp, vertical = 4.dp),
            ) { BasicText(q, style = geist(12.sp, FontWeight.SemiBold).copy(color = roles.onTertiaryContainer)) }
        }
        Spacer(Modifier.weight(1f))
        BasicText(duration?.let(::clock) ?: "", style = geist(13.sp).copy(color = roles.sub, fontFeatureSettings = "tnum"))
    }
    // The button group about the shaped Play.
    val shuffling by playback.shuffleOn.collectAsStateWithLifecycle()
    val repeat by playback.repeat.collectAsStateWithLifecycle()
    Row(
        Modifier.fillMaxWidth().padding(top = 18.dp),
        horizontalArrangement = Arrangement.SpaceBetween,
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Pill(Icons.Shuffle, shuffling) { playback.setShuffle(!shuffling) }
        Pill(MoreIcons.SkipBack, false) { playback.previous() }
        ShapedPlay(playing, loading, 88.dp, { playback.toggle() })
        Pill(MoreIcons.SkipForward, false) { playback.next() }
        Pill(if (repeat == QueueRepeat.ONE) Icons.Repeat1 else Icons.Repeat, repeat != QueueRepeat.OFF) { playback.cycleRepeat() }
    }
    Output(playback, onDevices)
}

/** A button of the group: a 64 × 56 pill widening to 76 while pressed; on, in secondary-container. */
@Composable
private fun Pill(icon: ImageVector, on: Boolean, onClick: () -> Unit) {
    val touches = remember { MutableInteractionSource() }
    val pressed by touches.collectIsPressedAsState()
    val width by animateDpAsState(if (pressed) 76.dp else 64.dp, Springs.fast(), label = "pill")
    Box(
        Modifier.width(width).height(56.dp).background(if (on) roles.secondaryContainer else roles.text.copy(alpha = 0.08f), RoundedCornerShape(28.dp))
            .clickable(touches, indication = null, onClick = onClick),
        contentAlignment = Alignment.Center,
    ) { Glyph(icon, if (on) roles.onSecondaryContainer else roles.text, 24.dp) }
}

/** Where it plays: this phone's output, or the device playing it. */
@androidx.annotation.OptIn(androidx.media3.common.util.UnstableApi::class)
@Composable
private fun Output(playback: Playback, onDevices: () -> Unit) {
    val context = androidx.compose.ui.platform.LocalContext.current
    val remote by playback.remote.collectAsStateWithLifecycle()
    val here = remember { io.github.honeyslatt.music.ui.nowplaying.Outputs.current(context) }
    val r = remote
    Box(Modifier.fillMaxWidth().padding(top = 16.dp), contentAlignment = Alignment.Center) {
        Row(
            Modifier.height(40.dp).press(RoundedCornerShape(20.dp), onClick = onDevices)
                .background(if (r != null) roles.secondaryContainer else roles.text.copy(alpha = 0.08f), RoundedCornerShape(20.dp))
                .padding(horizontal = 16.dp),
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.spacedBy(8.dp),
        ) {
            val fg = if (r != null) roles.onSecondaryContainer else roles.text
            Glyph(if (r != null) Icons.Monitor else here.icon, fg, 18.dp)
            BasicText(r?.device?.name ?: here.name, style = geist(14.sp, FontWeight.Medium).copy(color = fg), maxLines = 1)
        }
    }
}

/** Under the lyrics: the small wavy position, previous, the 72 dp Play, next. */
@androidx.annotation.OptIn(androidx.media3.common.util.UnstableApi::class)
@Composable
private fun Docked(playback: Playback, t: Track, playing: Boolean, loading: Boolean) {
    val position by rememberPosition(playback, playing, everyMs = 250)
    val duration = playback.duration?.takeIf { it > 0 }
    Column(Modifier.fillMaxWidth().padding(top = 8.dp)) {
        WavyLine(duration?.let { (position.toFloat() / it).coerceIn(0f, 1f) }, playing, big = false) { p ->
            duration?.let { playback.seekTo((p * it).toLong()) }
        }
        Row(Modifier.fillMaxWidth().padding(top = 10.dp), horizontalArrangement = Arrangement.SpaceEvenly, verticalAlignment = Alignment.CenterVertically) {
            Pill(MoreIcons.SkipBack, false) { playback.previous() }
            ShapedPlay(playing, loading, 72.dp, { playback.toggle() })
            Pill(MoreIcons.SkipForward, false) { playback.next() }
        }
    }
}
