package io.github.honeyslatt.music.ui.chrome

import androidx.compose.animation.core.LinearEasing
import androidx.compose.animation.core.animateFloat
import androidx.compose.animation.core.infiniteRepeatable
import androidx.compose.animation.core.rememberInfiniteTransition
import androidx.compose.animation.core.tween
import androidx.compose.foundation.gestures.detectHorizontalDragGestures
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import io.github.honeyslatt.music.core.model.Track
import io.github.honeyslatt.music.ui.Icons
import io.github.honeyslatt.music.ui.components.Cover
import io.github.honeyslatt.music.ui.components.Glyph
import io.github.honeyslatt.music.ui.components.Line
import io.github.honeyslatt.music.ui.components.press
import io.github.honeyslatt.music.ui.theme.geist
import io.github.honeyslatt.music.ui.theme.roles

/** What the mini player shows and does. */
class MiniState(
    val track: Track?,
    val playing: Boolean,
    val loading: Boolean,
    val onToggle: () -> Unit,
    val onNext: () -> Unit,
    val onPrevious: () -> Unit,
    val onOpen: () -> Unit,
)

/**
 * The song playing, as a 58 dp capsule: its cover, its title and artist,
 * play or pause, and next (left out when the bar is folded). A swipe left
 * goes to the next song, right to the previous one (iOS 26.1).
 */
@Composable
fun MiniPlayer(state: MiniState, folded: Boolean, modifier: Modifier = Modifier) {
    val track = state.track ?: return
    val shape = RoundedCornerShape(29.dp)
    val threshold = with(LocalDensity.current) { 56.dp.toPx() }
    Row(
        modifier
            .height(58.dp)
            .capsule(shape)
            .pointerInput(state) {
                var dragged = 0f
                detectHorizontalDragGestures(
                    onDragStart = { dragged = 0f },
                    onDragEnd = {
                        if (dragged < -threshold) state.onNext()
                        if (dragged > threshold) state.onPrevious()
                    },
                ) { _, amount -> dragged += amount }
            }
            .press(shape, onClick = state.onOpen)
            .padding(start = 9.dp, end = 8.dp),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        Cover(track.artwork, 40.dp, RoundedCornerShape(9.dp), Modifier.size(40.dp))
        Column(Modifier.weight(1f)) {
            Line(track.title, geist(14.5.sp, FontWeight.SemiBold).copy(color = roles.text))
            Line(track.artists.joinToString(", "), geist(13.sp).copy(color = roles.sub))
        }
        Box(Modifier.size(44.dp).press(CircleShape, shrink = 0.9f, onClick = state.onToggle), contentAlignment = Alignment.Center) {
            if (state.loading) {
                Spinner()
            } else {
                Glyph(if (state.playing) Icons.Pause else Icons.Play, roles.text, 24.dp)
            }
        }
        if (!folded) {
            Box(Modifier.size(44.dp).press(CircleShape, shrink = 0.9f, onClick = state.onNext), contentAlignment = Alignment.Center) {
                Glyph(Icons.Ff, roles.text, 26.dp)
            }
        }
    }
}

/** A small arc turning while a song is on its way. */
@Composable
fun Spinner(size: androidx.compose.ui.unit.Dp = 20.dp) {
    val turn = rememberInfiniteTransition(label = "spin")
    val angle = turn.animateFloat(0f, 360f, infiniteRepeatable(tween(900, easing = LinearEasing)), label = "angle")
    val colour = roles.text
    androidx.compose.foundation.Canvas(Modifier.size(size)) {
        drawArc(
            colour,
            angle.value,
            270f,
            useCenter = false,
            style = androidx.compose.ui.graphics.drawscope.Stroke(2.dp.toPx(), cap = androidx.compose.ui.graphics.StrokeCap.Round),
        )
    }
}
