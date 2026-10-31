package io.github.honeyslatt.music.ui.components

import androidx.compose.animation.AnimatedContent
import androidx.compose.animation.core.Animatable
import androidx.compose.animation.core.spring
import androidx.compose.animation.core.tween
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.scaleIn
import androidx.compose.animation.scaleOut
import androidx.compose.animation.togetherWith
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.unit.Dp
import io.github.honeyslatt.music.ui.Icons

/** Play and Pause turning into each other, the one shrinking as the other grows. */
@Composable
fun PlayPause(playing: Boolean, color: Color, size: Dp) {
    AnimatedContent(
        playing,
        transitionSpec = {
            (scaleIn(tween(180), 0.6f) + fadeIn(tween(140))) togetherWith (scaleOut(tween(140), 0.6f) + fadeOut(tween(100)))
        },
        label = "play-pause",
    ) { on -> Glyph(if (on) Icons.Pause else Icons.Play, color, size) }
}

/** Grows in with a little overshoot when `on` turns on: a mark just made, as Spotify's saved check. */
@Composable
fun Modifier.pop(on: Boolean): Modifier {
    val scale = remember { Animatable(1f) }
    val was = remember { booleanArrayOf(on) }
    LaunchedEffect(on) {
        if (on && !was[0]) {
            scale.snapTo(0.5f)
            scale.animateTo(1f, spring(dampingRatio = 0.4f, stiffness = 500f))
        }
        was[0] = on
    }
    return graphicsLayer {
        scaleX = scale.value
        scaleY = scale.value
    }
}
