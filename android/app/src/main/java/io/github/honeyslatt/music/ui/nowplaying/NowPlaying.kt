package io.github.honeyslatt.music.ui.nowplaying

import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.shrinkVertically
import androidx.compose.animation.expandVertically
import androidx.compose.animation.slideInVertically
import androidx.compose.animation.slideOutVertically
import androidx.compose.animation.core.animateDpAsState
import androidx.compose.animation.core.animateFloat
import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.animation.core.tween
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
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.layout.windowInsetsPadding
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.BasicText
import androidx.compose.runtime.Composable
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.derivedStateOf
import androidx.compose.runtime.State
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableLongStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.produceState
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.runtime.staticCompositionLocalOf
import androidx.compose.runtime.withFrameMillis
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import io.github.honeyslatt.music.ui.chrome.coverFlight
import androidx.compose.ui.draw.drawBehind
import androidx.compose.ui.draw.scale
import androidx.compose.ui.draw.shadow
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.lerp
import androidx.compose.ui.input.pointer.PointerEventPass
import androidx.compose.ui.input.pointer.PointerEventType
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.media3.common.util.UnstableApi
import io.github.honeyslatt.music.core.model.Track
import io.github.honeyslatt.music.core.model.FavouriteKind
import io.github.honeyslatt.music.core.model.ProviderKind
import io.github.honeyslatt.music.ui.actions.LocalActions
import io.github.honeyslatt.music.ui.components.anchor
import io.github.honeyslatt.music.ui.components.rememberAnchor
import io.github.honeyslatt.music.playback.Playback
import io.github.honeyslatt.music.ui.Icons
import io.github.honeyslatt.music.ui.components.Cover
import io.github.honeyslatt.music.ui.components.ExplicitBadge
import io.github.honeyslatt.music.ui.components.Glyph
import io.github.honeyslatt.music.ui.components.Line
import io.github.honeyslatt.music.ui.components.at
import io.github.honeyslatt.music.ui.components.press
import io.github.honeyslatt.music.ui.components.Sheet
import io.github.honeyslatt.music.ui.detail.MotionArtwork
import coil3.compose.AsyncImage
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.graphics.BlurEffect
import androidx.compose.ui.graphics.TileMode
import androidx.compose.ui.graphics.graphicsLayer
import io.github.honeyslatt.music.ui.detail.CoverColours
import io.github.honeyslatt.music.ui.detail.DolbyLogo
import io.github.honeyslatt.music.ui.detail.atmos
import io.github.honeyslatt.music.ui.detail.badge
import io.github.honeyslatt.music.ui.detail.losslessMime
import io.github.honeyslatt.music.ui.theme.geist
import io.github.honeyslatt.music.ui.theme.roles
import io.github.honeyslatt.music.ui.theme.tabular

/** Now Playing's colours: white on dark, the text colour on light. */
data class NpColours(val fg: Color, val sub: Color, val dim: Color, val button: Color)

val LocalNp = staticCompositionLocalOf { NpColours(Color.White, Color.White, Color.White, Color.White) }

enum class NpMode { Artwork, Lyrics, Queue }

/** What Now Playing can open, and close. */
class NpActions(
    val close: () -> Unit,
    val openAlbum: (Track) -> Unit,
    val openArtist: (Track) -> Unit,
    /** The song's animated cover, when it has one and they are wanted. */
    val motion: suspend (Track) -> String?,
)

/**
 * Now Playing, full screen: the cover's colours drifting behind, then the
 * artwork, lyrics or queue, and the controls.
 */
@androidx.annotation.OptIn(UnstableApi::class)
@Composable
fun NowPlaying(playback: Playback, actions: NpActions) {
    val track by playback.now.collectAsStateWithLifecycle()
    val playing by playback.playing.collectAsStateWithLifecycle()
    val t = track ?: return
    var mode by remember { mutableStateOf(NpMode.Artwork) }
    var signalOpen by remember { mutableStateOf(false) }
    var devicesOpen by remember { mutableStateOf(false) }
    val motion by produceState<String?>(null, t.albumId) {
        // Another album: not the last one's video meanwhile.
        value = null
        value = actions.motion(t)
    }
    val dark = roles.dark
    // Over the moving cover the text is white in both themes.
    val np = if (dark || motion != null) {
        NpColours(Color.White, Color.White.copy(alpha = 0.62f), Color.White.copy(alpha = 0.28f), Color.White.copy(alpha = 0.14f))
    } else {
        val c = roles.text
        NpColours(c, c.copy(alpha = 0.6f), c.copy(alpha = 0.26f), c.copy(alpha = 0.08f))
    }
    // Lyrics: the header and the controls step aside while the song plays
    // and nobody touches the screen; a touch brings them back.
    var hasLyrics by remember(t.id) { mutableStateOf(false) }
    var touches by remember { mutableIntStateOf(0) }
    var calm by remember { mutableStateOf(false) }
    LaunchedEffect(mode, playing, hasLyrics, touches) {
        calm = false
        if (mode == NpMode.Lyrics && playing && hasLyrics) {
            kotlinx.coroutines.delay(4_000)
            calm = true
        }
    }
    CompositionLocalProvider(LocalNp provides np) {
        Box(
            Modifier
                .fillMaxSize()
                .background(roles.base)
                .pointerInput(Unit) {
                    awaitPointerEventScope {
                        while (true) {
                            if (awaitPointerEvent(PointerEventPass.Initial).type == PointerEventType.Press) touches++
                        }
                    }
                },
        ) {
            val m = motion
            if (m != null) Backdrop(t, m, flat = mode != NpMode.Artwork) else Wash(t, playing)
            Column(
                Modifier
                    .fillMaxSize()
                    .windowInsetsPadding(WindowInsets.safeDrawing)
                    .padding(start = 28.dp, end = 28.dp, bottom = 10.dp),
            ) {
                // The grabber: a tap closes, as the swipe down does.
                Box(Modifier.fillMaxWidth().height(26.dp).press(onClick = actions.close), contentAlignment = Alignment.Center) {
                    Box(Modifier.size(40.dp, 5.dp).background(np.fg.copy(alpha = 0.35f), RoundedCornerShape(3.dp)))
                }
                when (mode) {
                    NpMode.Artwork -> if (m != null) {
                        // The cover moves behind everything instead.
                        Spacer(Modifier.weight(1f))
                        TitleRow(t, actions)
                    } else {
                        BoxWithConstraints(Modifier.fillMaxWidth().padding(top = 26.dp)) {
                            val scale by animateFloatAsState(if (playing) 1f else 0.86f, tween(400, easing = io.github.honeyslatt.music.ui.chrome.Standard), label = "art")
                            val shape = RoundedCornerShape(14.dp)
                            Cover(
                                t.artwork,
                                maxWidth,
                                shape,
                                Modifier
                                    .coverFlight(io.github.honeyslatt.music.ui.chrome.LocalNpVisibility.current)
                                    .fillMaxWidth()
                                    .aspectRatio(1f)
                                    .scale(scale)
                                    .shadow(30.dp, shape, ambientColor = Color.Black.copy(alpha = 0.38f), spotColor = Color.Black.copy(alpha = 0.38f)),
                            )
                        }
                        Spacer(Modifier.weight(1f).height(12.dp))
                        TitleRow(t, actions)
                    }
                    NpMode.Lyrics -> {
                        AnimatedVisibility(!calm, enter = fadeIn(Calm) + expandVertically(tween(CALM_MS)), exit = fadeOut(Calm) + shrinkVertically(tween(CALM_MS))) {
                            Head(t)
                        }
                        Box(Modifier.weight(1f).fillMaxWidth()) { Lyrics(playback, t, calm) { hasLyrics = it } }
                    }
                    NpMode.Queue -> {
                        Head(t)
                        Box(Modifier.weight(1f).fillMaxWidth().padding(top = 12.dp)) { QueueView(playback) }
                    }
                }
                AnimatedVisibility(
                    !calm,
                    enter = fadeIn(Calm) + slideInVertically(tween(CALM_MS)) { it / 12 } + expandVertically(tween(CALM_MS)),
                    exit = fadeOut(Calm) + slideOutVertically(tween(CALM_MS)) { it / 12 } + shrinkVertically(tween(CALM_MS)),
                ) {
                    Column {
                        Controls(playback, t, playing, mode, onSignal = { signalOpen = true }, onDevices = { devicesOpen = true }) { mode = if (mode == it) NpMode.Artwork else it }
                    }
                }
            }
            Sheet(signalOpen, "Signal Path", onClose = { signalOpen = false }) { SignalPath(playback, t) }
            DevicesSheet(devicesOpen, playback) { devicesOpen = false }
        }
    }
}

/** How long the lyrics' header and controls take to step aside. */
internal const val CALM_MS = 350
private val Calm = tween<Float>(CALM_MS)

/**
 * The animated cover filling the player, over its still cover until the
 * video's first frame, under a scrim that keeps the text readable: a
 * gradient with the artwork; blurred and flat with the lyrics or the queue.
 */
@Composable
internal fun Backdrop(t: Track, motion: String, flat: Boolean) {
    val flatness by animateFloatAsState(if (flat) 1f else 0f, tween(400), label = "scrim")
    Box(Modifier.fillMaxSize().background(Color.Black)) {
        Box(
            Modifier.fillMaxSize().graphicsLayer {
                // Read here, the blur changes without recomposing.
                val radius = flatness * 36.dp.toPx()
                renderEffect = if (radius > 0.5f) BlurEffect(radius, radius, TileMode.Clamp) else null
            },
        ) {
            t.artwork?.let {
                AsyncImage(it.at(1200), contentDescription = null, contentScale = ContentScale.Crop, modifier = Modifier.fillMaxSize())
            }
            MotionArtwork(motion, Modifier.fillMaxSize(), crop = true)
        }
        Box(
            Modifier.fillMaxSize().drawBehind {
                val shade = Color.Black
                drawRect(
                    Brush.verticalGradient(
                        0f to shade.copy(alpha = 0.35f),
                        0.2f to Color.Transparent,
                        0.4f to Color.Transparent,
                        0.58f to shade.copy(alpha = 0.62f),
                        0.72f to shade.copy(alpha = 0.8f),
                        1f to shade.copy(alpha = 0.9f),
                    ),
                    alpha = 1f - flatness,
                )
                drawRect(shade.copy(alpha = 0.62f * flatness))
            },
        )
    }
}

/**
 * The cover's three colours as four large blobs drifting over 40 s, under
 * a vignette. They drift while the music plays: paused, the screen stays
 * still and nothing is drawn again.
 */
@Composable
private fun Wash(track: Track, playing: Boolean) {
    val light = !roles.dark
    val base = roles.base
    val url = track.artwork?.at(120)
    // What's known at once, else the page's base until worked out; the
    // colours fade from one song's to the next's.
    val colours by produceState(CoverColours.knownWash(url, light) ?: listOf(base, base, base), url, light) {
        url?.let { CoverColours.wash(it, light) }?.let { value = it }
    }
    // There and back over 40 s, moved on 15 times a second: a few pixels a
    // second needn't redraw the screen at every frame.
    val wash = remember { androidx.compose.runtime.mutableFloatStateOf(0f) }
    var rising by remember { mutableStateOf(true) }
    LaunchedEffect(playing) {
        while (playing) {
            kotlinx.coroutines.delay(66)
            // Waits for a frame: none come with the screen off or the app
            // away, and the drift rests with them.
            withFrameMillis { }
            val next = wash.floatValue + if (rising) 66f / 40_000 else -66f / 40_000
            if (next !in 0f..1f) rising = !rising
            wash.floatValue = next.coerceIn(0f, 1f)
        }
    }
    val fade = tween<Color>(600)
    val c1 by androidx.compose.animation.animateColorAsState(colours[0], fade, label = "wash1")
    val c2 by androidx.compose.animation.animateColorAsState(colours[1], fade, label = "wash2")
    val c3 by androidx.compose.animation.animateColorAsState(colours[2], fade, label = "wash3")
    val fill = lerp(c1, Color.Black, 0.2f)
    Box(
        Modifier.fillMaxSize().drawBehind {
            drawRect(fill)
            val w = size.width
            val h = size.height
            val d = (wash.floatValue - 0.5f) * 0.12f
            fun blob(x: Float, y: Float, r: Float, c: Color) = drawCircle(
                Brush.radialGradient(listOf(c, c.copy(alpha = 0f)), Offset(w * x, h * y), r),
                radius = r,
                center = Offset(w * x, h * y),
            )
            blob(0.22f + d, 0.18f - d, w * 0.95f, c1)
            blob(0.82f - d, 0.30f + d, w * 1.0f, c2)
            blob(0.30f + d, 0.78f, w * 1.1f, c3)
            blob(0.85f, 0.88f - d, w * 0.9f, c1)
            drawRect(
                Brush.radialGradient(
                    0.4f to Color.Transparent,
                    1f to base.copy(alpha = 0.45f),
                    center = Offset(w * 0.5f, h * 0.45f),
                    radius = maxOf(w, h) * 0.8f,
                ),
            )
        },
    )
}

@Composable
private fun RoundSmall(icon: androidx.compose.ui.graphics.vector.ImageVector, onClick: () -> Unit) {
    val np = LocalNp.current
    Box(
        Modifier.size(34.dp).press(CircleShape, shrink = 0.92f, onClick = onClick).background(np.button, CircleShape),
        contentAlignment = Alignment.Center,
    ) { Glyph(icon, np.fg, 18.dp) }
}

/** Favourite (the star fills), and the song's menu. */
@androidx.annotation.OptIn(UnstableApi::class)
@Composable
private fun StarAndMore(t: Track) {
    val actions = LocalActions.current
    // A station's stream is no song to favourite or add.
    if (actions?.isStation(t) == true) return
    if (t.id.provider != ProviderKind.NAVIDROME) {
        val favourite = actions?.isFavourite(FavouriteKind.SONG, t.id) == true
        RoundSmall(if (favourite) Icons.Starf else Icons.Star) { actions?.toggleFavourite(t) }
    }
    val anchor = rememberAnchor()
    Box(Modifier.anchor(anchor)) { RoundSmall(Icons.More) { actions?.trackMenu(t, anchor.rect, nowPlaying = true) } }
}

/** Title (it opens the album) and artist (the artist), with favourite and more. */
@Composable
private fun TitleRow(t: Track, actions: NpActions) {
    val np = LocalNp.current
    Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(10.dp)) {
        Column(Modifier.weight(1f)) {
            Row(
                Modifier.press { actions.openAlbum(t) },
                verticalAlignment = Alignment.CenterVertically,
                horizontalArrangement = Arrangement.spacedBy(7.dp),
            ) {
                Line(t.title, geist(20.sp, FontWeight.SemiBold).copy(color = np.fg), Modifier.weight(1f, fill = false))
                if (t.explicit) ExplicitBadge()
            }
            Box(Modifier.press { actions.openArtist(t) }) {
                Line(t.artists.joinToString(", "), geist(20.sp).copy(color = np.sub))
            }
        }
        StarAndMore(t)
    }
}

/** The small header of the lyrics and queue modes. */
@Composable
private fun Head(t: Track) {
    val np = LocalNp.current
    Row(Modifier.padding(top = 18.dp), verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(12.dp)) {
        val shape = RoundedCornerShape(9.dp)
        Cover(t.artwork, 64.dp, shape, Modifier.size(64.dp).shadow(10.dp, shape))
        Column(Modifier.weight(1f)) {
            Line(t.title, geist(16.sp, FontWeight.SemiBold).copy(color = np.fg))
            Line(t.artists.joinToString(", "), geist(15.sp).copy(color = np.sub))
        }
        StarAndMore(t)
    }
}

/** Times as `m:ss`. */
fun clock(ms: Long): String {
    val s = (ms / 1000).coerceAtLeast(0)
    return "${s / 60}:${(s % 60).toString().padStart(2, '0')}"
}

/**
 * The position, read at every frame while the song plays. A state, to be
 * read where it is used (in drawing, or through `derivedStateOf`), so that
 * only that follows each frame rather than the whole screen.
 */
@androidx.annotation.OptIn(UnstableApi::class)
@Composable
fun rememberPosition(playback: Playback, playing: Boolean, everyMs: Long = 0): State<Long> {
    val position = remember { mutableLongStateOf(playback.position) }
    LaunchedEffect(playing) {
        position.longValue = playback.position
        while (playing) {
            // At every frame, or as often as asked: a bar that moves a few
            // pixels a second needn't redraw the screen 120 times a second.
            if (everyMs > 0) kotlinx.coroutines.delay(everyMs)
            withFrameMillis { position.longValue = playback.position }
        }
        // Paused, it moves only when sought: looked at a few times a second,
        // and only while shown.
        while (true) {
            kotlinx.coroutines.delay(250)
            withFrameMillis { position.longValue = playback.position }
        }
    }
    return position
}

@androidx.annotation.OptIn(UnstableApi::class)
@Composable
private fun Controls(playback: Playback, t: Track, playing: Boolean, mode: NpMode, onSignal: () -> Unit, onDevices: () -> Unit, onMode: (NpMode) -> Unit) {
    val np = LocalNp.current
    val remote by playback.remote.collectAsStateWithLifecycle()
    if (remote?.playing?.live ?: playback.isLive(t)) {
        LiveControls()
    } else {
        // The signal path is this phone's: none to show for another device.
        Scrubber(playback, t, playing, if (remote == null) onSignal else ({}))
    }
    Row(Modifier.fillMaxWidth().padding(top = 26.dp), horizontalArrangement = Arrangement.SpaceEvenly, verticalAlignment = Alignment.CenterVertically) {
        Transport(Icons.Rw, 36) { playback.previous() }
        Box(Modifier.size(84.dp, 72.dp).press(CircleShape, shrink = 0.86f) { playback.toggle() }, contentAlignment = Alignment.Center) {
            io.github.honeyslatt.music.ui.components.PlayPause(playing, np.fg, 46.dp)
        }
        Transport(Icons.Ff, 36) { playback.next() }
    }
    Row(Modifier.fillMaxWidth().padding(top = 30.dp), horizontalArrangement = Arrangement.SpaceAround) {
        Bottom(Icons.Lyrics, mode == NpMode.Lyrics) { onMode(NpMode.Lyrics) }
        Output(playback, onDevices)
        Bottom(Icons.Queue, mode == NpMode.Queue) { onMode(NpMode.Queue) }
    }
}

/** On air: nowhere to seek, so a full bar and LIVE. */
@Composable
private fun LiveControls() {
    val np = LocalNp.current
    Box(Modifier.fillMaxWidth().padding(top = 22.dp).height(18.dp), contentAlignment = Alignment.CenterStart) {
        Box(Modifier.fillMaxWidth().height(7.dp).background(np.fg.copy(alpha = 0.9f), RoundedCornerShape(6.dp)))
    }
    Box(Modifier.fillMaxWidth().padding(top = 4.dp).height(22.dp), contentAlignment = Alignment.Center) {
        BasicText(
            "LIVE",
            Modifier.background(roles.red, RoundedCornerShape(6.dp)).padding(horizontal = 9.dp, vertical = 3.dp),
            style = geist(11.5.sp, FontWeight.Bold, 0.04f).copy(color = Color.White),
        )
    }
}

/** The scrubber and the times, with the quality between them. */
@androidx.annotation.OptIn(UnstableApi::class)
@Composable
private fun Scrubber(playback: Playback, t: Track, playing: Boolean, onSignal: () -> Unit) {
    val np = LocalNp.current
    val position = rememberPosition(playback, playing, everyMs = 33)
    val duration = playback.duration ?: 0L
    var dragging by remember { mutableStateOf<Float?>(null) }
    // Where the bar is, read as it is drawn; the times, once a second.
    val shownMs = { dragging?.let { (it * duration).toLong() } ?: position.value }
    val shown by remember(duration) { derivedStateOf { shownMs() / 1000 * 1000 } }
    // The scrubber: 7 dp, 12 dp while dragged; it seeks on release.
    val thickness by animateDpAsState(if (dragging != null) 12.dp else 7.dp, tween(150), label = "scrub")
    BoxWithConstraints(Modifier.fillMaxWidth().padding(top = 22.dp).height(18.dp)) {
        val width = constraints.maxWidth.toFloat()
        Box(
            Modifier
                .fillMaxWidth()
                .height(18.dp)
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
                .pointerInput(duration) {
                    detectTapGestures { playback.seekTo(((it.x / width).coerceIn(0f, 1f) * duration).toLong()) }
                },
            contentAlignment = Alignment.CenterStart,
        ) {
            val fill = np.fg.copy(alpha = 0.9f)
            Box(
                Modifier.fillMaxWidth().height(thickness).background(np.dim, RoundedCornerShape(6.dp)).drawBehind {
                    val fraction = if (duration > 0) (shownMs().toFloat() / duration).coerceIn(0f, 1f) else 0f
                    val radius = androidx.compose.ui.geometry.CornerRadius(6.dp.toPx())
                    drawRoundRect(fill, size = size.copy(width = size.width * fraction), cornerRadius = radius)
                },
            )
        }
    }
    Row(Modifier.fillMaxWidth().padding(top = 4.dp), verticalAlignment = Alignment.CenterVertically) {
        val times = geist(12.sp, FontWeight.Medium).tabular().copy(color = np.sub)
        BasicText(clock(shown), Modifier.weight(1f), style = times)
        // Only Lossless and Dolby Atmos earn a badge, as in Apple Music. The
        // uploader's own file is lossless once the player reads it so.
        val original = playback.source(t) == Playback.Source.Original
        val signal by playback.signal.collectAsStateWithLifecycle()
        val quality = playback.quality(t)
        val badge = if (original) "Lossless".takeIf { losslessMime(signal.input?.sampleMimeType) } else quality?.badge()
        badge?.let { label ->
            val chip = RoundedCornerShape(6.dp)
            Row(
                Modifier.height(22.dp).press(chip, shrink = 0.94f, onClick = onSignal).background(np.button, chip).padding(horizontal = 9.dp),
                verticalAlignment = Alignment.CenterVertically,
                horizontalArrangement = Arrangement.spacedBy(5.dp),
            ) {
                if (quality?.atmos() == true && !original) DolbyLogo(np.fg, 9.dp)
                BasicText(label, style = geist(11.5.sp, FontWeight.SemiBold, 0.02f).copy(color = np.fg))
            }
        }
        BasicText(
            "-" + clock(duration - shown),
            Modifier.weight(1f),
            style = times.copy(textAlign = androidx.compose.ui.text.style.TextAlign.End),
        )
    }
}

@Composable
private fun Transport(icon: androidx.compose.ui.graphics.vector.ImageVector, size: Int, onClick: () -> Unit) {
    Box(Modifier.size(84.dp, 72.dp).press(CircleShape, shrink = 0.86f, onClick = onClick), contentAlignment = Alignment.Center) {
        Glyph(icon, LocalNp.current.fg, size.dp)
    }
}

@Composable
private fun Bottom(icon: androidx.compose.ui.graphics.vector.ImageVector, on: Boolean, onClick: () -> Unit) {
    val np = LocalNp.current
    val shape = RoundedCornerShape(12.dp)
    Box(
        Modifier.size(52.dp, 44.dp).press(shape, shrink = 0.92f, onClick = onClick).background(if (on) np.button else Color.Transparent, shape),
        contentAlignment = Alignment.Center,
    ) { Glyph(icon, np.fg.copy(alpha = if (on) 1f else 0.78f), 22.dp) }
}

/** Where it plays: this phone's output, or the device played on; a tap opens Play On. */
@androidx.annotation.OptIn(UnstableApi::class)
@Composable
private fun Output(playback: Playback, onDevices: () -> Unit) {
    val context = androidx.compose.ui.platform.LocalContext.current
    val np = LocalNp.current
    val remote by playback.remote.collectAsStateWithLifecycle()
    val device = remember { mutableStateOf(Outputs.current(context)) }
    // Told when an output comes or goes (headphones, Bluetooth), rather
    // than asked every two seconds.
    androidx.compose.runtime.DisposableEffect(Unit) {
        val audio = context.getSystemService(android.media.AudioManager::class.java)
        val callback = object : android.media.AudioDeviceCallback() {
            override fun onAudioDevicesAdded(added: Array<out android.media.AudioDeviceInfo>) {
                device.value = Outputs.current(context)
            }

            override fun onAudioDevicesRemoved(removed: Array<out android.media.AudioDeviceInfo>) {
                device.value = Outputs.current(context)
            }
        }
        audio.registerAudioDeviceCallback(callback, null)
        onDispose { audio.unregisterAudioDeviceCallback(callback) }
    }
    Column(horizontalAlignment = Alignment.CenterHorizontally) {
        val r = remote
        if (r != null) {
            Bottom(r.icon(), true, onDevices)
            BasicText(r.device.name, style = geist(11.sp, FontWeight.SemiBold).copy(color = np.fg), maxLines = 1)
        } else {
            Bottom(device.value.icon, false, onDevices)
            BasicText(device.value.name, style = geist(11.sp).copy(color = np.sub), maxLines = 1)
        }
    }
}
