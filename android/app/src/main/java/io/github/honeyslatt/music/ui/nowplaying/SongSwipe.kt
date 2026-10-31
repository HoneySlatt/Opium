package io.github.honeyslatt.music.ui.nowplaying

import androidx.compose.animation.core.Animatable
import androidx.compose.animation.core.spring
import androidx.compose.animation.core.tween
import androidx.compose.foundation.gestures.detectHorizontalDragGestures
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.ui.Modifier
import androidx.compose.ui.hapticfeedback.HapticFeedbackType
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.input.pointer.util.VelocityTracker
import androidx.compose.ui.layout.onSizeChanged
import androidx.compose.ui.platform.LocalHapticFeedback
import io.github.honeyslatt.music.core.model.ItemId
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch

/**
 * A swipe through the songs, as Spotify's: what's swiped follows the
 * finger; let go past a quarter of its width (or flung), it leaves, the
 * next or previous song plays, and the new one is in place: the next as
 * it was shown coming from the right, the previous coming in from the
 * left. Not far enough, it springs back.
 */
class SongSwipe internal constructor(private val scope: CoroutineScope) {
    /** How far it is moved, in px. */
    val offset = Animatable(0f)

    /** Its width and the gap to the song beside it, in px. */
    var span = 0f
        internal set

    /** -1 when the next song was asked for, 1 the previous, 0 none. */
    internal var going = 0

    internal fun back() {
        going = 0
        scope.launch { offset.animateTo(0f, spring(dampingRatio = 0.8f, stiffness = 500f)) }
    }

    internal fun drag(amount: Float) {
        going = 0
        scope.launch { offset.snapTo(offset.value + amount) }
    }

    internal fun release(velocity: Float, onNext: () -> Unit, onPrevious: () -> Unit, felt: () -> Unit) {
        val x = offset.value
        val dir = when {
            x < -span / 4 || velocity < -1500f -> -1
            x > span / 4 || velocity > 1500f -> 1
            else -> 0
        }
        scope.launch {
            if (dir == 0) {
                offset.animateTo(0f, spring(dampingRatio = 0.8f, stiffness = 500f))
                return@launch
            }
            going = dir
            offset.animateTo(dir * span, tween(170))
            felt()
            if (dir < 0) onNext() else onPrevious()
            // Nothing came (no song there): back in place.
            delay(1_500)
            if (going == dir) {
                going = 0
                offset.animateTo(0f, spring(dampingRatio = 0.8f, stiffness = 400f))
            }
        }
    }
}

/**
 * A [SongSwipe] for the song `id`, that puts the new one in place when it
 * changes; `nextShown` when the next song is drawn on the right already.
 */
@Composable
fun rememberSongSwipe(id: ItemId, nextShown: Boolean = false): SongSwipe {
    val scope = rememberCoroutineScope()
    val swipe = remember { SongSwipe(scope) }
    LaunchedEffect(id) {
        when {
            // The next, shown on the right, is where the song was.
            swipe.going == -1 && nextShown -> swipe.offset.snapTo(0f)
            swipe.going != 0 -> {
                swipe.offset.snapTo(-swipe.going * swipe.span)
                swipe.offset.animateTo(0f, spring(dampingRatio = 0.85f, stiffness = 380f))
            }
            else -> if (swipe.offset.value != 0f) swipe.offset.animateTo(0f)
        }
        swipe.going = 0
    }
    return swipe
}

/** Takes the horizontal drags for `swipe`; `gap` (px) is between it and the song beside. */
@Composable
fun Modifier.songSwipe(swipe: SongSwipe, gap: Float, onNext: () -> Unit, onPrevious: () -> Unit): Modifier {
    val haptics = LocalHapticFeedback.current
    return onSizeChanged { swipe.span = it.width + gap }
        .pointerInput(swipe) {
            val tracker = VelocityTracker()
            detectHorizontalDragGestures(
                onDragStart = { tracker.resetTracking() },
                onDragEnd = {
                    swipe.release(tracker.calculateVelocity().x, onNext, onPrevious) {
                        haptics.performHapticFeedback(HapticFeedbackType.SegmentTick)
                    }
                },
                onDragCancel = { swipe.back() },
            ) { change, amount ->
                tracker.addPosition(change.uptimeMillis, change.position)
                swipe.drag(amount)
            }
        }
}
