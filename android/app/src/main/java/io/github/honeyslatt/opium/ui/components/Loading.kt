package io.github.honeyslatt.opium.ui.components

import androidx.compose.animation.core.LinearEasing
import androidx.compose.animation.core.RepeatMode
import androidx.compose.animation.core.animateFloat
import androidx.compose.animation.core.infiniteRepeatable
import androidx.compose.animation.core.rememberInfiniteTransition
import androidx.compose.animation.core.tween
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.aspectRatio
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.StrokeCap
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import io.github.honeyslatt.opium.core.model.Motion
import io.github.honeyslatt.opium.ui.theme.look
import io.github.honeyslatt.opium.ui.theme.roles
import kotlin.math.cos
import kotlin.math.sin

/** What's loading looks like, for the grey shapes Spotify draws in its place. */
enum class Shapes { Rows, Grid, Shelves }

/**
 * Something on its way: Apple Music's spinner in the middle, or Spotify's
 * grey shapes of what comes, pulsing.
 */
@Composable
fun Loader(shapes: Shapes = Shapes.Rows, modifier: Modifier = Modifier) {
    if (look.layout.motion == Motion.SPOTIFY) {
        Placeholder(shapes, modifier)
    } else {
        Box(modifier.fillMaxWidth().padding(vertical = 48.dp), contentAlignment = Alignment.Center) {
            ActivitySpinner()
        }
    }
}

/**
 * iOS's: eight spokes, the lit one going round, the others fading behind;
 * or, still, as many of them as `drawn` of eight, as when a page is pulled.
 */
@Composable
fun ActivitySpinner(size: Dp = 28.dp, color: Color = roles.sub, drawn: Float? = null) {
    val turn = rememberInfiniteTransition(label = "activity")
    val step by turn.animateFloat(0f, 8f, infiniteRepeatable(tween(800, easing = LinearEasing)), label = "step")
    Canvas(Modifier.size(size)) {
        val lit = step.toInt()
        val r = this.size.minDimension / 2
        val stroke = r * 0.22f
        val spokes = if (drawn == null) 8 else (drawn * 8).toInt().coerceIn(0, 8)
        for (k in 0 until spokes) {
            val a = Math.toRadians(k * 45.0 - 90).toFloat()
            val behind = (lit - k + 8) % 8
            val c = Offset(center.x, center.y)
            drawLine(
                color.copy(alpha = if (drawn == null) 1f - behind * 0.11f else 0.7f),
                c + Offset(cos(a), sin(a)) * (r * 0.45f),
                c + Offset(cos(a), sin(a)) * (r - stroke / 2),
                stroke,
                StrokeCap.Round,
            )
        }
    }
}

/** The lyrics on their way: the spinner, or Spotify's lines breathing, in `color` over the cover's. */
@Composable
fun LyricsLoader(color: Color) {
    if (look.layout.motion != Motion.SPOTIFY) return ActivitySpinner(color = color.copy(alpha = 0.7f))
    val pulse = rememberInfiniteTransition(label = "pulse")
    val alpha by pulse.animateFloat(0.22f, 0.1f, infiniteRepeatable(tween(900), RepeatMode.Reverse), label = "alpha")
    Column(Modifier.fillMaxWidth().padding(horizontal = 24.dp), verticalArrangement = Arrangement.spacedBy(18.dp)) {
        listOf(0.9f, 0.7f, 0.82f, 0.55f, 0.75f).forEach { w ->
            Box(Modifier.fillMaxWidth(w).height(22.dp).background(color.copy(alpha = alpha), RoundedCornerShape(6.dp)))
        }
    }
}

/** Spotify's: the shapes of what loads, in grey, breathing. */
@Composable
private fun Placeholder(shapes: Shapes, modifier: Modifier) {
    val pulse = rememberInfiniteTransition(label = "pulse")
    val alpha by pulse.animateFloat(1f, 0.45f, infiniteRepeatable(tween(900), RepeatMode.Reverse), label = "alpha")
    val grey = roles.surface2
    val block = Modifier.background(grey, RoundedCornerShape(4.dp))
    Column(modifier.fillMaxWidth().graphicsLayer { this.alpha = alpha }.padding(16.dp), verticalArrangement = Arrangement.spacedBy(16.dp)) {
        when (shapes) {
            Shapes.Rows -> repeat(8) {
                Row(verticalAlignment = Alignment.CenterVertically) {
                    Box(Modifier.size(48.dp).then(block))
                    Spacer(Modifier.width(12.dp))
                    Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                        Box(Modifier.width(180.dp).height(12.dp).then(block))
                        Box(Modifier.width(110.dp).height(10.dp).then(block))
                    }
                }
            }
            Shapes.Grid -> repeat(3) {
                Row(horizontalArrangement = Arrangement.spacedBy(16.dp)) {
                    repeat(2) {
                        Column(Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(8.dp)) {
                            Box(Modifier.fillMaxWidth().aspectRatio(1f).then(block))
                            Box(Modifier.width(110.dp).height(12.dp).then(block))
                        }
                    }
                }
            }
            Shapes.Shelves -> repeat(3) {
                Column(verticalArrangement = Arrangement.spacedBy(12.dp)) {
                    Box(Modifier.width(160.dp).height(18.dp).then(block))
                    Row(horizontalArrangement = Arrangement.spacedBy(12.dp)) {
                        repeat(3) {
                            Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                                Box(Modifier.size(140.dp).then(block))
                                Box(Modifier.width(100.dp).height(12.dp).then(block))
                            }
                        }
                    }
                }
            }
        }
    }
}
