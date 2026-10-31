package io.github.honeyslatt.music.ui.nowplaying

import androidx.compose.animation.core.Animatable
import androidx.compose.animation.core.CubicBezierEasing
import androidx.compose.animation.core.LinearEasing
import androidx.compose.animation.core.RepeatMode
import androidx.compose.animation.core.animateFloat
import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.animation.core.infiniteRepeatable
import androidx.compose.animation.core.rememberInfiniteTransition
import androidx.compose.animation.core.tween
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.LazyListState
import androidx.compose.foundation.lazy.itemsIndexed
import androidx.compose.foundation.lazy.rememberLazyListState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.text.BasicText
import androidx.compose.runtime.derivedStateOf
import androidx.compose.runtime.State
import androidx.compose.runtime.Composable
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.produceState
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.draw.drawWithContent
import androidx.compose.ui.graphics.BlendMode
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.CompositingStrategy
import androidx.compose.ui.graphics.Shadow
import androidx.compose.ui.graphics.TransformOrigin
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.graphics.lerp
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.media3.common.util.UnstableApi
import io.github.honeyslatt.music.core.model.LyricLine
import io.github.honeyslatt.music.core.model.Lyrics
import io.github.honeyslatt.music.core.model.Track
import io.github.honeyslatt.music.playback.Playback
import io.github.honeyslatt.music.ui.components.press
import io.github.honeyslatt.music.ui.theme.geist

/** Where lyrics come from: the core asks the service, then the sites. */
object LyricsSource {
    var find: suspend (Track) -> Lyrics? = { null }
}

private val LyricLine.startMs get() = start.toMillis()

/**
 * The lyrics at 27 sp bold: the line being sung lights up word by word
 * with a soft edge and a faint glow, the others stay dim. A tap on a line
 * seeks to it; an instrumental gap breathes three dots.
 */
@androidx.annotation.OptIn(UnstableApi::class)
@Composable
fun Lyrics(playback: Playback, track: Track, calm: Boolean, onFound: (Boolean) -> Unit) {
    val np = LocalNp.current
    val playing by playback.playing.collectAsStateWithLifecycle()
    // Whether they were looked for yet, and what was found.
    val lyrics by produceState<Pair<Boolean, Lyrics?>>(false to null, track.id) { value = true to LyricsSource.find(track) }
    val (looked, l) = lyrics
    LaunchedEffect(lyrics) { onFound(l?.lines?.isNotEmpty() == true) }
    if (!looked) {
        Box(Modifier.fillMaxSize(), contentAlignment = Alignment.Center) {
            io.github.honeyslatt.music.ui.components.LyricsLoader(np.fg)
        }
        return
    }
    if (l == null || l.lines.isEmpty()) {
        Column(Modifier.fillMaxSize().padding(horizontal = 16.dp), verticalArrangement = Arrangement.Center, horizontalAlignment = Alignment.CenterHorizontally) {
            BasicText("No lyrics for this song", style = geist(19.sp, FontWeight.Bold).copy(color = np.fg, textAlign = TextAlign.Center))
            BasicText(
                "Neither its service nor the lyrics sites have them.",
                Modifier.padding(top = 6.dp),
                style = geist(15.sp).copy(color = np.sub, textAlign = TextAlign.Center),
            )
        }
        return
    }
    val position = rememberPosition(playback, playing)
    // The line sung changes a few times a minute: only then is the list
    // composed again, the line itself following each frame (`Sung`).
    val current by remember(l) { derivedStateOf { if (l.synced) l.lines.indexOfLast { it.startMs <= position.value }.coerceAtLeast(0) else -1 } }
    val state = rememberLazyListState()
    var glide by remember { mutableStateOf<Glide?>(null) }
    BoxWithConstraints(Modifier.fillMaxSize()) {
        val above = with(LocalDensity.current) { (maxHeight * 0.28f).roundToPx() }
        val anchor by rememberUpdatedState(above)
        val sung by rememberUpdatedState(current)
        LaunchedEffect(current) { if (current >= 0) state.glideTo(current, anchor) { glide = it } }
        // The list grows or shrinks as the controls step aside: the line
        // being sung goes back to its place once they have.
        LaunchedEffect(calm) {
            kotlinx.coroutines.delay(CALM_MS.toLong())
            if (sung >= 0) state.glideTo(sung, anchor) { glide = it }
        }
        LazyColumn(
            Modifier
                .fillMaxSize()
                // The list fades out at its top and its bottom.
                .graphicsLayer { compositingStrategy = CompositingStrategy.Offscreen }
                .drawWithContent {
                    drawContent()
                    drawRect(
                        Brush.verticalGradient(
                            0f to Color.Transparent,
                            0.12f to Color.Black,
                            0.74f to Color.Black,
                            1f to Color.Transparent,
                        ),
                        blendMode = BlendMode.DstIn,
                    )
                },
            state = state,
            contentPadding = PaddingValues(top = 24.dp, bottom = maxHeight * 0.45f),
        ) {
            itemsIndexed(l.lines) { i, line ->
                val on = i == current
                // A line that was sung dims slowly; the next one grows as it lights.
                val lit by animateFloatAsState(if (on) 1f else 0f, tween(if (on) 360 else 520), label = "lit")
                Box(
                    Modifier
                        .fillMaxWidth()
                        .graphicsLayer {
                            translationY = glide?.lag(i) ?: 0f
                            val grown = 1f + 0.02f * lit
                            scaleX = grown
                            scaleY = grown
                            transformOrigin = TransformOrigin(0f, 0.5f)
                        }
                        .press { playback.seekTo(line.startMs) }
                        .padding(vertical = 10.dp),
                ) {
                    when {
                        line.text.isBlank() -> Dots(on)
                        on -> Sung(line, l.lines.getOrNull(i + 1), position)
                        else -> BasicText(line.text, style = lyricStyle().copy(color = if (l.synced) lerp(np.dim, np.fg, lit) else np.fg))
                    }
                }
            }
        }
    }
}

/**
 * The list on its way to a line: it glides there, and the lines below it
 * follow one after the other, each a beat later.
 */
private class Glide(val to: Int, val shift: Float) {
    val clock = Animatable(0f)

    /** How far line `i` trails behind the list. */
    fun lag(i: Int): Float {
        val rank = (i - to).coerceIn(0, STAGGERED)
        if (rank == 0) return 0f
        val t = clock.value
        return shift * (eased(t) - eased(t - rank * STAGGER_MS))
    }
}

private val GlideEasing = CubicBezierEasing(0.3f, 0f, 0.1f, 1f)
private const val GLIDE_MS = 620f
private const val STAGGER_MS = 45f
private const val STAGGERED = 6

private fun eased(ms: Float) = GlideEasing.transform((ms / GLIDE_MS).coerceIn(0f, 1f))

/** Bring line `index` to `anchor` px below the top: a glide if it is in sight, a cut if not. */
private suspend fun LazyListState.glideTo(index: Int, anchor: Int, show: (Glide?) -> Unit) {
    val seen = layoutInfo.visibleItemsInfo.firstOrNull { it.index == index }
    if (seen == null) {
        show(null)
        scrollToItem(index, -anchor)
        return
    }
    val glide = Glide(index, (seen.offset - anchor).toFloat())
    show(glide)
    val total = GLIDE_MS + STAGGERED * STAGGER_MS
    scroll {
        var done = 0f
        glide.clock.animateTo(total, tween(total.toInt(), easing = LinearEasing)) {
            val far = glide.shift * eased(value)
            scrollBy(far - done)
            done = far
        }
    }
}

@Composable
private fun lyricStyle() = geist(27.sp, FontWeight.Bold, -0.01f).copy(lineHeight = 33.sp)

/** The line being sung: each word lit as far as it has been sung. */
@OptIn(ExperimentalLayoutApi::class)
@Composable
private fun Sung(line: LyricLine, next: LyricLine?, positionState: State<Long>) {
    val np = LocalNp.current
    val position = positionState.value
    val words = line.text.split(' ').filter { it.isNotEmpty() }
    val progress: List<Float> = if (line.words.isNotEmpty()) {
        // The source timed each word.
        val timed = line.words.map { it.text.trim() }.filter { it.isNotEmpty() }
        if (timed.size == words.size) {
            line.words.filter { it.text.isNotBlank() }.map { w ->
                val start = w.start.toMillis()
                val end = w.end.toMillis().coerceAtLeast(start + 1)
                ((position - start).toFloat() / (end - start)).coerceIn(0f, 1f)
            }
        } else {
            shares(words, line, next, position)
        }
    } else {
        shares(words, line, next, position)
    }
    val glow = Shadow(np.fg.copy(alpha = 0.22f), blurRadius = 28f)
    FlowRow {
        words.forEachIndexed { i, word ->
            val p = progress.getOrElse(i) { 0f }
            val edge = (p * 1.25f).coerceIn(0f, 1f)
            val lit = (p * 1.25f - 0.25f).coerceIn(0f, 1f)
            val brush = Brush.horizontalGradient(0f to np.fg, lit to np.fg, edge to np.dim, 1f to np.dim)
            BasicText(
                if (i < words.lastIndex) "$word " else word,
                style = lyricStyle().copy(brush = brush, shadow = glow),
            )
        }
    }
}

/** Without word timings: each word gets its share of the line by length. */
private fun shares(words: List<String>, line: LyricLine, next: LyricLine?, position: Long): List<Float> {
    val start = line.startMs
    val end = (line.end?.toMillis() ?: next?.startMs ?: (start + 4_000))
    val sung = ((position - start).toFloat() / ((end - start) * 0.82f).coerceAtLeast(500f)).coerceIn(0f, 1f)
    val total = words.sumOf { it.length + 1 }.toFloat()
    var acc = 0f
    return words.map { w ->
        val a = acc / total
        acc += w.length + 1
        val b = acc / total
        ((sung - a) / (b - a)).coerceIn(0f, 1f)
    }
}

/** Three dots that breathe during an instrumental gap. */
@Composable
private fun Dots(on: Boolean) {
    val np = LocalNp.current
    val breath by rememberInfiniteTransition(label = "dots").animateFloat(
        0.4f,
        1f,
        infiniteRepeatable(tween(800), RepeatMode.Reverse),
        label = "breath",
    )
    Row(Modifier.padding(vertical = 8.dp), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
        repeat(3) { i ->
            Box(
                Modifier
                    .size(11.dp)
                    .alpha(if (on) (breath - i * 0.12f).coerceIn(0.3f, 1f) else 1f)
                    .background(if (on) np.fg else np.dim, CircleShape),
            )
        }
    }
}
