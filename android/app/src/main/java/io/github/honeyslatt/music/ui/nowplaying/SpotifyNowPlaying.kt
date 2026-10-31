package io.github.honeyslatt.music.ui.nowplaying

import androidx.compose.foundation.background
import androidx.compose.foundation.gestures.detectHorizontalDragGestures
import androidx.compose.foundation.gestures.detectTapGestures
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.aspectRatio
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.safeDrawing
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.windowInsetsPadding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.BasicText
import androidx.compose.foundation.verticalScroll
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.produceState
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.drawBehind
import androidx.compose.ui.draw.shadow
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.lerp
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.media3.common.util.UnstableApi
import io.github.honeyslatt.music.core.QueueRepeat
import io.github.honeyslatt.music.core.model.FavouriteKind
import io.github.honeyslatt.music.core.model.Track
import io.github.honeyslatt.music.playback.Playback
import io.github.honeyslatt.music.ui.Icons
import io.github.honeyslatt.music.ui.MoreIcons
import io.github.honeyslatt.music.ui.actions.LocalActions
import io.github.honeyslatt.music.ui.components.Cover
import io.github.honeyslatt.music.ui.components.ExplicitBadge
import io.github.honeyslatt.music.ui.components.Glyph
import io.github.honeyslatt.music.ui.components.Line
import io.github.honeyslatt.music.ui.components.Sheet
import io.github.honeyslatt.music.ui.components.anchor
import io.github.honeyslatt.music.ui.components.press
import io.github.honeyslatt.music.ui.components.pop
import androidx.compose.ui.graphics.graphicsLayer
import io.github.honeyslatt.music.ui.components.rememberAnchor
import io.github.honeyslatt.music.ui.detail.badge
import io.github.honeyslatt.music.ui.detail.rememberPageColour
import io.github.honeyslatt.music.ui.theme.geist
import io.github.honeyslatt.music.ui.theme.roles

/**
 * Spotify's Now Playing: where it plays from at the top, the cover on its
 * colour, the song with its saved mark, the position, the controls, the
 * device and the quality; under them, the lyrics in a card, which opens
 * them over the whole screen.
 */
@androidx.annotation.OptIn(UnstableApi::class)
@Composable
fun SpotifyNowPlaying(playback: Playback, actions: NpActions) {
    val track by playback.now.collectAsStateWithLifecycle()
    val playing by playback.playing.collectAsStateWithLifecycle()
    val t = track ?: return
    var lyricsOpen by remember { mutableStateOf(false) }
    var queueOpen by remember { mutableStateOf(false) }
    var signalOpen by remember { mutableStateOf(false) }
    var devicesOpen by remember { mutableStateOf(false) }
    val page = rememberPageColour(t.artwork)
    val motion by produceState<String?>(null, t.albumId) {
        // Another album: not the last one's video meanwhile.
        value = null
        value = actions.motion(t)
    }
    val np =NpColours(Color.White, Color.White.copy(alpha = 0.7f), Color.White.copy(alpha = 0.3f), Color.White.copy(alpha = 0.14f))
    CompositionLocalProvider(LocalNp provides np) {
        Box(
            Modifier.fillMaxSize().background(roles.base)
                .background(Brush.verticalGradient(listOf(page, lerp(page, Color.Black, 0.55f), lerp(page, Color.Black, 0.8f)))),
        ) {
            // Like Spotify's Canvas, the animated cover fills the screen behind.
            val m = motion
            if (m != null && !lyricsOpen) Backdrop(t, m, flat = false)
            if (lyricsOpen) {
                FullLyrics(playback, t, page, playing) { lyricsOpen = false }
            } else {
                Column(
                    Modifier.fillMaxSize().windowInsetsPadding(WindowInsets.safeDrawing).verticalScroll(rememberScrollState()).padding(horizontal = 24.dp),
                ) {
                    Header(t, actions)
                    Spacer(Modifier.height(36.dp))
                    val shape = RoundedCornerShape(8.dp)
                    if (m != null) {
                        // The cover's room left empty, the song and controls where they were.
                        Spacer(Modifier.fillMaxWidth().aspectRatio(1f))
                    } else {
                        // Swiped, the cover goes and the next comes from the right.
                        val swipe = rememberSongSwipe(t.id, nextShown = true)
                        val upcoming by playback.upcoming.collectAsStateWithLifecycle()
                        val gap = with(androidx.compose.ui.platform.LocalDensity.current) { 24.dp.toPx() }
                        Box(Modifier.fillMaxWidth().aspectRatio(1f).songSwipe(swipe, gap, playback::next, playback::previousSong)) {
                            upcoming.firstOrNull()?.let { next ->
                                Cover(
                                    next.artwork,
                                    360.dp,
                                    shape,
                                    Modifier.matchParentSize().graphicsLayer { translationX = swipe.offset.value + swipe.span },
                                )
                            }
                            Cover(
                                t.artwork,
                                360.dp,
                                shape,
                                Modifier.matchParentSize()
                                    .graphicsLayer { translationX = swipe.offset.value }
                                    .shadow(24.dp, shape, ambientColor = Color.Black.copy(alpha = 0.4f), spotColor = Color.Black.copy(alpha = 0.4f)),
                            )
                        }
                    }
                    Spacer(Modifier.height(44.dp))
                    TitleAndSaved(t, actions, playback)
                    Position(playback, playing)
                    Transport(playback, playing)
                    Footer(playback, t, onDevices = { devicesOpen = true }, onQueue = { queueOpen = true }, onSignal = { signalOpen = true })
                    Spacer(Modifier.height(20.dp))
                    LyricsCard(playback, t, page) { lyricsOpen = true }
                    Spacer(Modifier.height(24.dp))
                }
            }
            Sheet(queueOpen, "Queue", onClose = { queueOpen = false }) { Box(Modifier.height(560.dp)) { SpotifyQueueView(playback) } }
            Sheet(signalOpen, "Signal Path", onClose = { signalOpen = false }) { SignalPath(playback, t) }
            DevicesSheet(devicesOpen, playback) { devicesOpen = false }
        }
    }
}

/** Close, "Playing from Album" and its name, the song's menu. */
@Composable
private fun Header(t: Track, actions: NpActions) {
    val menu = LocalActions.current
    val anchor = rememberAnchor()
    Row(Modifier.fillMaxWidth().height(56.dp), verticalAlignment = Alignment.CenterVertically) {
        Box(Modifier.size(44.dp).press(CircleShape, onClick = actions.close), contentAlignment = Alignment.Center) {
            Glyph(Icons.Down, Color.White, 26.dp)
        }
        Column(Modifier.weight(1f), horizontalAlignment = Alignment.CenterHorizontally) {
            BasicText("Playing from Album", style = geist(12.sp).copy(color = Color.White.copy(alpha = 0.85f)))
            Line(t.album ?: "", geist(14.sp, FontWeight.Bold).copy(color = Color.White, textAlign = TextAlign.Center))
        }
        Box(
            Modifier.size(44.dp).anchor(anchor).press(CircleShape) { menu?.trackMenu(t, anchor.rect, nowPlaying = true) },
            contentAlignment = Alignment.Center,
        ) { Glyph(MoreIcons.MoreVertical, Color.White, 24.dp) }
    }
}

/** The title, the E badge and the artists, the saved mark on the right. */
@androidx.annotation.OptIn(UnstableApi::class)
@Composable
private fun TitleAndSaved(t: Track, actions: NpActions, playback: Playback) {
    val menu = LocalActions.current
    Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
        Column(Modifier.weight(1f)) {
            Box(Modifier.press(RoundedCornerShape(6.dp)) { actions.openAlbum(t) }) {
                Line(t.title, geist(23.sp, FontWeight.Bold).copy(color = Color.White))
            }
            Row(
                Modifier.padding(top = 2.dp).press(RoundedCornerShape(6.dp)) { actions.openArtist(t) },
                verticalAlignment = Alignment.CenterVertically,
                horizontalArrangement = Arrangement.spacedBy(6.dp),
            ) {
                if (t.explicit) ExplicitBadge()
                Line(t.artists.joinToString(", "), geist(16.sp).copy(color = Color.White.copy(alpha = 0.7f)))
            }
        }
        if (!playback.isStation(t)) {
            val saved = menu?.isFavourite(FavouriteKind.SONG, t.id) == true
            val haptics = androidx.compose.ui.platform.LocalHapticFeedback.current
            Box(
                Modifier.size(48.dp).press(CircleShape) {
                    haptics.performHapticFeedback(if (saved) androidx.compose.ui.hapticfeedback.HapticFeedbackType.ToggleOff else androidx.compose.ui.hapticfeedback.HapticFeedbackType.ToggleOn)
                    menu?.toggleFavourite(t)
                },
                contentAlignment = Alignment.Center,
            ) {
                if (saved) {
                    Box(Modifier.size(28.dp).pop(saved).background(roles.accent, CircleShape), contentAlignment = Alignment.Center) {
                        Glyph(Icons.Check, roles.onAccent, 17.dp)
                    }
                } else {
                    Glyph(Icons.Pluscircle, Color.White, 30.dp)
                }
            }
        }
    }
}

/** The position: a thin white bar with its knob, the times under it. */
@androidx.annotation.OptIn(UnstableApi::class)
@Composable
private fun Position(playback: Playback, playing: Boolean) {
    val position = rememberPosition(playback, playing, everyMs = 33)
    val duration = playback.duration ?: 0L
    var dragging by remember { mutableStateOf<Float?>(null) }
    val shownMs = { dragging?.let { (it * duration).toLong() } ?: position.value }
    BoxWithConstraints(Modifier.fillMaxWidth().padding(top = 18.dp).height(20.dp)) {
        val width = constraints.maxWidth.toFloat()
        Box(
            Modifier.fillMaxWidth().height(20.dp)
                .pointerInput(duration) {
                    detectHorizontalDragGestures(
                        onDragStart = { dragging = (it.x / width).coerceIn(0f, 1f) },
                        onDragEnd = {
                            dragging?.let { playback.seekTo((it * duration).toLong()) }
                            dragging = null
                        },
                        onDragCancel = { dragging = null },
                    ) { change, _ -> dragging = (change.position.x / width).coerceIn(0f, 1f) }
                }
                .pointerInput(duration) { detectTapGestures { playback.seekTo(((it.x / width).coerceIn(0f, 1f) * duration).toLong()) } }
                .drawBehind {
                    val y = size.height / 2
                    val fraction = if (duration > 0) (shownMs().toFloat() / duration).coerceIn(0f, 1f) else 0f
                    val line = 4.dp.toPx()
                    drawLine(Color.White.copy(alpha = 0.3f), androidx.compose.ui.geometry.Offset(0f, y), androidx.compose.ui.geometry.Offset(size.width, y), line, androidx.compose.ui.graphics.StrokeCap.Round)
                    drawLine(Color.White, androidx.compose.ui.geometry.Offset(0f, y), androidx.compose.ui.geometry.Offset(size.width * fraction, y), line, androidx.compose.ui.graphics.StrokeCap.Round)
                    drawCircle(Color.White, 6.dp.toPx(), androidx.compose.ui.geometry.Offset(size.width * fraction, y))
                },
        )
    }
    val shown = shownMs() / 1000 * 1000
    Row(Modifier.fillMaxWidth()) {
        val times = geist(12.sp).copy(color = Color.White.copy(alpha = 0.7f))
        BasicText(clock(shown), Modifier.weight(1f), style = times)
        BasicText(clock(duration), style = times)
    }
}

/** Shuffle, previous, Play, next, repeat: the switches green with a dot while on. */
@androidx.annotation.OptIn(UnstableApi::class)
@Composable
private fun Transport(playback: Playback, playing: Boolean) {
    val shuffle by playback.shuffleOn.collectAsStateWithLifecycle()
    val repeat by playback.repeat.collectAsStateWithLifecycle()
    val loading by playback.loading.collectAsStateWithLifecycle()
    val green = roles.accent
    Row(Modifier.fillMaxWidth().padding(top = 14.dp), verticalAlignment = Alignment.CenterVertically) {
        Switchy(Icons.Shuffle, shuffle, green) { playback.setShuffle(!shuffle) }
        Box(Modifier.weight(1f))
        Box(Modifier.size(56.dp).press(CircleShape) { playback.previous() }, contentAlignment = Alignment.Center) {
            Glyph(MoreIcons.SkipBack, Color.White, 34.dp)
        }
        Box(Modifier.weight(1f))
        Box(
            Modifier.size(72.dp).press(CircleShape, shrink = 0.94f) { playback.toggle() }.background(Color.White, CircleShape),
            contentAlignment = Alignment.Center,
        ) {
            if (loading) Box(Modifier.size(28.dp)) else io.github.honeyslatt.music.ui.components.PlayPause(playing, Color.Black, 32.dp)
        }
        Box(Modifier.weight(1f))
        Box(Modifier.size(56.dp).press(CircleShape) { playback.next() }, contentAlignment = Alignment.Center) {
            Glyph(MoreIcons.SkipForward, Color.White, 34.dp)
        }
        Box(Modifier.weight(1f))
        Switchy(if (repeat == QueueRepeat.ONE) Icons.Repeat1 else Icons.Repeat, repeat != QueueRepeat.OFF, green) { playback.cycleRepeat() }
    }
}

/** A button that is on or off: green with a dot under it when on. */
@Composable
private fun Switchy(icon: androidx.compose.ui.graphics.vector.ImageVector, on: Boolean, green: Color, onClick: () -> Unit) {
    Box(Modifier.size(48.dp).press(CircleShape, onClick = onClick), contentAlignment = Alignment.Center) {
        Glyph(icon, if (on) green else Color.White, 28.dp)
        if (on) Box(Modifier.align(Alignment.BottomCenter).padding(bottom = 1.dp).size(5.dp).background(green, CircleShape))
    }
}

/** The device and the quality on the left, the queue on the right. */
@androidx.annotation.OptIn(UnstableApi::class)
@Composable
private fun Footer(playback: Playback, t: Track, onDevices: () -> Unit, onQueue: () -> Unit, onSignal: () -> Unit) {
    val remote by playback.remote.collectAsStateWithLifecycle()
    val green = roles.accent
    val quality = playback.quality(t)?.badge()
    Row(Modifier.fillMaxWidth().padding(top = 16.dp), verticalAlignment = Alignment.CenterVertically) {
        Row(
            Modifier.weight(1f).press(RoundedCornerShape(8.dp), onClick = onDevices),
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.spacedBy(10.dp),
        ) {
            val device = remote?.device?.name
            Glyph(MoreIcons.Devices, if (device != null) green else Color.White, 22.dp)
            val words = listOfNotNull(device, quality).joinToString(" • ")
            if (words.isNotEmpty()) {
                Box(Modifier.press(RoundedCornerShape(6.dp), onClick = onSignal)) {
                    BasicText(words, style = geist(13.sp, FontWeight.Medium).copy(color = if (device != null) green else Color.White.copy(alpha = 0.85f)))
                }
            }
        }
        Box(Modifier.size(44.dp).press(CircleShape, onClick = onQueue), contentAlignment = Alignment.Center) {
            Glyph(Icons.Queue, Color.White, 24.dp)
        }
    }
}

/** The lyrics in a card of the cover's colour; its button opens them whole. */
@androidx.annotation.OptIn(UnstableApi::class)
@Composable
private fun LyricsCard(playback: Playback, t: Track, page: Color, onOpen: () -> Unit) {
    // Asked once per song: the card shows only once lyrics are known to be there.
    val has by produceState(false, t.id) { value = LyricsSource.find(t)?.lines?.isNotEmpty() == true }
    if (!has) return
    val shape = RoundedCornerShape(12.dp)
    Column(
        Modifier.fillMaxWidth().background(lerp(page, Color.Black, 0.2f), shape).press(shape, onClick = onOpen).padding(16.dp),
    ) {
        Row(verticalAlignment = Alignment.CenterVertically) {
            BasicText("Lyrics", Modifier.weight(1f), style = geist(17.sp, FontWeight.Bold).copy(color = Color.White))
            Box(Modifier.size(36.dp).background(Color.Black.copy(alpha = 0.25f), CircleShape).press(CircleShape, onClick = onOpen), contentAlignment = Alignment.Center) {
                Glyph(MoreIcons.Expand, Color.White, 16.dp)
            }
        }
        Box(Modifier.fillMaxWidth().height(260.dp).padding(top = 8.dp)) {
            Lyrics(playback, t, calm = false) {}
            // A tap anywhere opens them, rather than seeking to a line.
            Box(Modifier.matchParentSize().pointerInput(Unit) { detectTapGestures { onOpen() } })
        }
    }
}

/** The lyrics over the whole screen on the cover's colour, the song at the
 *  top, the position and Play at the bottom. */
@androidx.annotation.OptIn(UnstableApi::class)
@Composable
private fun FullLyrics(playback: Playback, t: Track, page: Color, playing: Boolean, onClose: () -> Unit) {
    androidx.activity.compose.BackHandler(onBack = onClose)
    Column(
        Modifier.fillMaxSize().background(lerp(page, Color.Black, 0.3f)).windowInsetsPadding(WindowInsets.safeDrawing).padding(horizontal = 24.dp),
    ) {
        Row(Modifier.fillMaxWidth().height(60.dp), verticalAlignment = Alignment.CenterVertically) {
            Box(Modifier.size(44.dp).press(CircleShape, onClick = onClose), contentAlignment = Alignment.Center) {
                Glyph(Icons.Down, Color.White, 26.dp)
            }
            Column(Modifier.weight(1f), horizontalAlignment = Alignment.CenterHorizontally) {
                Line(t.title, geist(14.sp, FontWeight.Bold).copy(color = Color.White, textAlign = TextAlign.Center))
                Line(t.artists.joinToString(", "), geist(13.sp).copy(color = Color.White.copy(alpha = 0.85f), textAlign = TextAlign.Center))
            }
            Spacer(Modifier.size(44.dp))
        }
        Box(Modifier.weight(1f).fillMaxWidth()) { Lyrics(playback, t, calm = false) {} }
        Position(playback, playing)
        Box(Modifier.fillMaxWidth().padding(vertical = 12.dp), contentAlignment = Alignment.Center) {
            Box(
                Modifier.size(64.dp).press(CircleShape, shrink = 0.94f) { playback.toggle() }.background(Color.White, CircleShape),
                contentAlignment = Alignment.Center,
            ) { Glyph(if (playing) Icons.Pause else Icons.Play, Color.Black, 28.dp) }
        }
    }
}
