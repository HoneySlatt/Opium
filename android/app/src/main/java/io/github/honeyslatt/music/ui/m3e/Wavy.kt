package io.github.honeyslatt.music.ui.m3e

import androidx.compose.foundation.Canvas
import androidx.compose.foundation.gestures.detectHorizontalDragGestures
import androidx.compose.foundation.gestures.detectTapGestures
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.size
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.runtime.withFrameNanos
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.CornerRadius
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Path
import androidx.compose.ui.graphics.StrokeCap
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import io.github.honeyslatt.music.ui.theme.roles
import kotlin.math.PI
import kotlin.math.cos
import kotlin.math.min
import kotlin.math.sin

/** Whether the system turned animations off: then nothing moves of itself. */
@Composable
fun animationsOff(): Boolean {
    val resolver = LocalContext.current.contentResolver
    return remember { android.provider.Settings.Global.getFloat(resolver, android.provider.Settings.Global.ANIMATOR_DURATION_SCALE, 1f) == 0f }
}

/** A wave's height (0 flat, 1 standing) and where it is along, as the desktop's: it moves while playing and flattens when paused. */
class Wave {
    var amp by mutableFloatStateOf(0f)
    var phase by mutableFloatStateOf(0f)
}

@Composable
fun rememberWave(playing: Boolean): Wave {
    val still = animationsOff()
    val wave = remember { Wave().apply { amp = if (playing) 1f else 0f } }
    LaunchedEffect(playing, still) {
        if (still) {
            wave.amp = if (playing) 1f else 0f
            return@LaunchedEffect
        }
        val target = if (playing) 1f else 0f
        var then = withFrameNanos { it }
        while (playing || kotlin.math.abs(wave.amp - target) > 0.002f) {
            val now = withFrameNanos { it }
            val dt = min((now - then) / 1e9f, 0.1f)
            then = now
            wave.amp += (target - wave.amp) * min(dt * 6f, 1f)
            if (playing) wave.phase = ((wave.phase - dt * 4.2f) % (2 * PI)).toFloat()
        }
        wave.amp = target
    }
    return wave
}

/**
 * The desktop's wavy position (its toolbar's `wavy`): what has played a
 * wave in primary, the rest flat in secondary-container after a gap, a dot
 * at the end and a handle where it is. `big`: Now Playing's. Without
 * `progress` (a station on air), flat all along. Tapped or dragged, it
 * seeks.
 */
@Composable
fun WavyLine(progress: Float?, playing: Boolean, big: Boolean, modifier: Modifier = Modifier, onSeek: ((Float) -> Unit)? = null) {
    val (stroke, amplitude, wavelength) = if (big) Triple(6.dp, 5.dp, 44.dp) else Triple(4.dp, 3.2.dp, 32.dp)
    val (gap, handle) = if (big) 8.dp to 28.dp else 6.dp to 22.dp
    val wave = rememberWave(playing)
    var dragged by remember { mutableStateOf<Float?>(null) }
    val primary = roles.primary
    val rest = roles.secondaryContainer
    val seek = if (onSeek == null) Modifier else Modifier
        .pointerInput(Unit) { detectTapGestures { onSeek((it.x / size.width).coerceIn(0f, 1f)) } }
        .pointerInput(Unit) {
            detectHorizontalDragGestures(
                onDragEnd = { dragged?.let(onSeek); dragged = null },
                onDragCancel = { dragged = null },
            ) { change, _ -> dragged = (change.position.x / size.width).coerceIn(0f, 1f) }
        }
    Canvas(modifier.fillMaxWidth().height(handle).then(seek)) {
        val s = stroke.toPx()
        val cy = size.height / 2
        val (start, end) = s / 2 to size.width - s / 2
        fun flat(from: Float, to: Float) {
            if (to > from) drawRoundRect(rest, Offset(from - s / 2, cy - s / 2), Size(to - from + s, s), CornerRadius(s / 2))
        }
        val p = dragged ?: progress
        if (p == null) {
            flat(start, end)
            return@Canvas
        }
        val x = start + p.coerceIn(0f, 1f) * (end - start)
        val playedEnd = (x - gap.toPx() / 2 - if (dragged != null) 2.dp.toPx() else 0f).coerceAtLeast(start)
        val a = amplitude.toPx() * wave.amp
        val w = wavelength.toPx()
        fun y(px: Float) = cy + a * sin(px / w * 2 * PI.toFloat() + wave.phase)
        if (playedEnd > start) {
            val path = Path().apply {
                moveTo(start, y(start))
                var px = start
                while (px < playedEnd) {
                    lineTo(px, y(px))
                    px += 2.dp.toPx()
                }
                lineTo(playedEnd, y(playedEnd))
            }
            drawPath(path, primary, style = Stroke(s, cap = StrokeCap.Round))
        }
        flat(x + gap.toPx(), end)
        drawCircle(primary, s / 2 - 0.5.dp.toPx(), Offset(end, cy))
        drawRoundRect(primary, Offset(x - 2.dp.toPx(), cy - handle.toPx() / 2), Size(4.dp.toPx(), handle.toPx()), CornerRadius(2.dp.toPx()))
    }
}

/**
 * The mini player's ring round its cover: the position as a wave of
 * `waves` waves going round from the top, the rest of the circle flat in
 * secondary-container after a gap; flattening when paused.
 */
@Composable
fun WavyRing(
    progress: Float?,
    playing: Boolean,
    modifier: Modifier = Modifier,
    radius: Dp = 23.dp,
    stroke: Dp = 3.dp,
    amplitude: Dp = 1.2.dp,
    waves: Int = 12,
) {
    val wave = rememberWave(playing)
    val primary = roles.primary
    val rest = roles.secondaryContainer
    Canvas(modifier.size((radius + amplitude + stroke) * 2)) {
        val c = Offset(size.width / 2, size.height / 2)
        val r = radius.toPx()
        val s = stroke.toPx()
        val gap = 2 * s / r
        val p = progress?.coerceIn(0f, 1f) ?: 0f
        val played = (2 * PI * p).toFloat()
        val a = amplitude.toPx() * wave.amp
        val top = -PI.toFloat() / 2
        if (played > gap) {
            val path = Path()
            val steps = (played / (2 * PI) * 240).toInt().coerceAtLeast(2)
            for (i in 0..steps) {
                val t = top + played * i / steps
                val rr = r + a * sin(waves * (t - top) + wave.phase)
                val pt = Offset(c.x + rr * cos(t), c.y + rr * sin(t))
                if (i == 0) path.moveTo(pt.x, pt.y) else path.lineTo(pt.x, pt.y)
            }
            drawPath(path, primary, style = Stroke(s, cap = StrokeCap.Round))
        }
        val from = if (played > gap) played + gap else 0f
        val to = (2 * PI).toFloat() - if (played > gap) gap else 0f
        if (to > from) {
            drawArc(
                rest,
                Math.toDegrees((top + from).toDouble()).toFloat(),
                Math.toDegrees((to - from).toDouble()).toFloat(),
                false,
                Offset(c.x - r, c.y - r),
                Size(2 * r, 2 * r),
                style = Stroke(s, cap = StrokeCap.Round),
            )
        }
    }
}
