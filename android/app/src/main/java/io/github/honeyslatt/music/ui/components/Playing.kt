package io.github.honeyslatt.music.ui.components

import androidx.compose.animation.core.LinearEasing
import androidx.compose.animation.core.RepeatMode
import androidx.compose.animation.core.animateFloat
import androidx.compose.animation.core.infiniteRepeatable
import androidx.compose.animation.core.rememberInfiniteTransition
import androidx.compose.animation.core.tween
import androidx.compose.foundation.layout.size
import androidx.compose.runtime.Composable
import androidx.compose.runtime.Immutable
import androidx.compose.runtime.compositionLocalOf
import androidx.compose.runtime.getValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.drawBehind
import androidx.compose.ui.geometry.CornerRadius
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import io.github.honeyslatt.music.core.model.ItemId
import kotlin.math.PI
import kotlin.math.cos

/** The song playing, for the rows that show it. */
@Immutable
data class NowPlayingMark(val id: ItemId? = null, val playing: Boolean = false)

val LocalNowPlaying = compositionLocalOf { NowPlayingMark() }

/**
 * Three bars that dance while the song plays and rest at 40 % when it is
 * paused, each a third of a beat behind the one before.
 */
@Composable
fun Bars(colour: Color, playing: Boolean, width: Dp = 14.dp, height: Dp = 13.dp) {
    val phase = if (playing) {
        rememberInfiniteTransition(label = "bars").animateFloat(
            0f,
            1f,
            infiniteRepeatable(tween(1_800, easing = LinearEasing), RepeatMode.Restart),
            label = "phase",
        )
    } else {
        null
    }
    // Read while drawing only: the bars move without recomposing the row.
    androidx.compose.foundation.layout.Box(
        Modifier.size(width, height).drawBehind {
            val bar = 3.dp.toPx()
            val gap = 2.dp.toPx()
            val start = (size.width - (3 * bar + 2 * gap)) / 2
            repeat(3) { i ->
                val share = phase?.let {
                    // Up over 0.9 s and down again, eased at both ends.
                    val t = (it.value + i / 6f) % 1f
                    0.25f + 0.75f * (1 - cos(2 * PI * t).toFloat()) / 2
                } ?: 0.4f
                val h = size.height * share
                drawRoundRect(
                    colour,
                    Offset(start + i * (bar + gap), size.height - h),
                    Size(bar, h),
                    CornerRadius(1.dp.toPx()),
                )
            }
        },
    )
}
