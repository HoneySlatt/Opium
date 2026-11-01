package io.github.honeyslatt.music.ui.components

import androidx.compose.animation.core.animate
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.statusBars
import androidx.compose.foundation.layout.windowInsetsPadding
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.input.nestedscroll.NestedScrollConnection
import androidx.compose.ui.input.nestedscroll.NestedScrollSource
import androidx.compose.ui.input.nestedscroll.nestedScroll
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.IntOffset
import androidx.compose.ui.unit.Velocity
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.launch
import kotlin.math.roundToInt

/** How far a page is pulled for it to refresh, and the room the spinner keeps while it does. */
private val Far = 90.dp
private val Rest = 56.dp
private val SpinnerSize = 28.dp

/**
 * Apple Music's pull to refresh: the page pulled down from its top, the
 * spinner shows spoke by spoke over it, `under` the bar; let go far enough,
 * `refresh` runs, the spinner turning in the room left until it is done.
 */
@Composable
fun PullRefresh(refresh: (suspend () -> Unit)?, under: Dp, content: @Composable () -> Unit) {
    if (refresh == null) return content()
    val density = LocalDensity.current
    val far = with(density) { Far.toPx() }
    val rest = with(density) { Rest.toPx() }
    val scope = rememberCoroutineScope()
    var pulled by remember { mutableFloatStateOf(0f) }
    var refreshing by remember { mutableStateOf(false) }
    val latest by rememberUpdatedState(refresh)
    val connection = remember(far, rest) {
        object : NestedScrollConnection {
            override fun onPreScroll(available: Offset, source: NestedScrollSource): Offset {
                // Pushed back up while pulled: the pull goes before the page scrolls.
                if (refreshing || source != NestedScrollSource.UserInput || available.y >= 0f || pulled <= 0f) return Offset.Zero
                val to = (pulled + available.y).coerceAtLeast(0f)
                val used = to - pulled
                pulled = to
                return Offset(0f, used)
            }

            override fun onPostScroll(consumed: Offset, available: Offset, source: NestedScrollSource): Offset {
                // At the top and pulled further: the page follows the finger, slower.
                if (refreshing || source != NestedScrollSource.UserInput || available.y <= 0f) return Offset.Zero
                pulled += available.y * 0.5f
                return Offset(0f, available.y)
            }

            override suspend fun onPreFling(available: Velocity): Velocity {
                if (refreshing || pulled <= 0f) return Velocity.Zero
                if (pulled >= far) {
                    refreshing = true
                    scope.launch {
                        animate(pulled, rest) { v, _ -> pulled = v }
                        try {
                            latest()
                        } finally {
                            refreshing = false
                        }
                        animate(pulled, 0f) { v, _ -> pulled = v }
                    }
                } else {
                    scope.launch { animate(pulled, 0f) { v, _ -> pulled = v } }
                }
                return available
            }
        }
    }
    Box(Modifier.fillMaxSize().nestedScroll(connection)) {
        Box(Modifier.fillMaxSize().graphicsLayer { translationY = pulled }) { content() }
        Spinner({ pulled }, far, refreshing, under)
    }
}

/** In the middle of the room the pull leaves: still while pulled, turning while it refreshes. */
@Composable
private fun Spinner(pulled: () -> Float, far: Float, refreshing: Boolean, under: Dp) {
    val now = pulled()
    if (now <= 0f) return
    val half = with(LocalDensity.current) { SpinnerSize.toPx() / 2 }
    Box(
        Modifier.fillMaxWidth().windowInsetsPadding(WindowInsets.statusBars).padding(top = under)
            .offset { IntOffset(0, (pulled() / 2 - half).roundToInt()) },
        contentAlignment = Alignment.TopCenter,
    ) {
        ActivitySpinner(SpinnerSize, drawn = if (refreshing) null else now / far)
    }
}
