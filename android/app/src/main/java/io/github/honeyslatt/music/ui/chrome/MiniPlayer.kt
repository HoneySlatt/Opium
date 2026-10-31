package io.github.honeyslatt.music.ui.chrome

import androidx.compose.animation.core.LinearEasing
import androidx.compose.animation.core.animateFloat
import androidx.compose.animation.core.infiniteRepeatable
import androidx.compose.animation.core.rememberInfiniteTransition
import androidx.compose.animation.core.tween
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
import androidx.compose.ui.draw.clipToBounds
import androidx.compose.ui.graphics.graphicsLayer
import io.github.honeyslatt.music.ui.nowplaying.songSwipe
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import io.github.honeyslatt.music.core.model.Track
import io.github.honeyslatt.music.ui.actions.LocalActions
import io.github.honeyslatt.music.ui.components.anchor
import io.github.honeyslatt.music.ui.components.rememberAnchor
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
    /** Now Playing shows: Apple Music's cover has flown into it. */
    val npOpen: Boolean = false,
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
    val actions = LocalActions.current
    val anchor = rememberAnchor()
    val swipe = io.github.honeyslatt.music.ui.nowplaying.rememberSongSwipe(track.id)
    Row(
        modifier
            .height(58.dp)
            .anchor(anchor)
            .capsule(shape)
            .press(shape, onLongClick = { actions?.trackMenu(track, anchor.rect) }, onClick = state.onOpen)
            .padding(start = 9.dp, end = 8.dp),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        // The song follows the finger, and the next or previous comes in.
        SwipedSong(swipe, state, Modifier.weight(1f)) {
            Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(12.dp)) {
                // Its room kept while it is in Now Playing.
                Box(Modifier.size(40.dp)) {
                    androidx.compose.animation.AnimatedVisibility(
                        !state.npOpen,
                        enter = androidx.compose.animation.fadeIn(androidx.compose.animation.core.snap()),
                        exit = androidx.compose.animation.fadeOut(androidx.compose.animation.core.snap()),
                    ) {
                        Cover(track.artwork, 40.dp, RoundedCornerShape(9.dp), Modifier.coverFlight(this).size(40.dp))
                    }
                }
                Column(Modifier.weight(1f)) {
                    Line(track.title, geist(14.5.sp, FontWeight.SemiBold).copy(color = roles.text))
                    Line(track.artists.joinToString(", "), geist(13.sp).copy(color = roles.sub))
                }
            }
        }
        Box(Modifier.size(44.dp).press(CircleShape, shrink = 0.9f, onClick = state.onToggle), contentAlignment = Alignment.Center) {
            if (state.loading) {
                Spinner()
            } else {
                io.github.honeyslatt.music.ui.components.PlayPause(state.playing, roles.text, 24.dp)
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

/** The song in a mini player, swiped through the songs, within its own bounds. */
@Composable
internal fun SwipedSong(
    swipe: io.github.honeyslatt.music.ui.nowplaying.SongSwipe,
    state: MiniState,
    modifier: Modifier = Modifier,
    content: @Composable () -> Unit,
) {
    Box(
        modifier
            .clipToBounds()
            .songSwipe(swipe, 0f, state.onNext, state.onPrevious),
    ) {
        Box(Modifier.graphicsLayer { translationX = swipe.offset.value }) { content() }
    }
}
