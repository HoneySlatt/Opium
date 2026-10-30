package io.github.honeyslatt.music.ui.components

import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.statusBars
import androidx.compose.foundation.layout.windowInsetsPadding
import androidx.compose.foundation.layout.windowInsetsTopHeight
import androidx.compose.foundation.lazy.LazyListScope
import androidx.compose.foundation.lazy.LazyListState
import androidx.compose.foundation.lazy.grid.GridCells
import androidx.compose.foundation.lazy.grid.GridItemSpan
import androidx.compose.foundation.lazy.grid.LazyGridScope
import androidx.compose.foundation.lazy.grid.LazyGridState
import androidx.compose.foundation.lazy.grid.LazyVerticalGrid
import androidx.compose.foundation.lazy.grid.rememberLazyGridState
import androidx.compose.foundation.lazy.rememberLazyListState
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.derivedStateOf
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.runtime.snapshotFlow
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.layout.layout
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import io.github.honeyslatt.music.ui.Icons
import io.github.honeyslatt.music.ui.chrome.ChromeHeight
import io.github.honeyslatt.music.ui.nav.Navigator
import io.github.honeyslatt.music.ui.theme.geist
import io.github.honeyslatt.music.ui.theme.roles

/** A button of the navigation bar's right side. */
data class BarAction(val icon: ImageVector, val label: String, val onClick: () -> Unit)

/** Room left under a page for the bottom chrome. */
private val Under = ChromeHeight + 58.dp + 10.dp + 24.dp

/**
 * A page that lists: its large title, then `content`, under the navigation
 * bar, which turns solid and shows the title once the large one is gone.
 */
@Composable
fun ListPage(
    nav: Navigator,
    title: String,
    actions: List<BarAction> = emptyList(),
    account: (@Composable () -> Unit)? = null,
    state: LazyListState = rememberLazyListState(),
    content: LazyListScope.() -> Unit,
) {
    Folding(nav, state.position())
    ScrollsToTop(nav) { state.animateScrollToItem(0) }
    val titleAt = if (nav.canPop) 2 else 1
    val scrolled by remember(titleAt) { derivedStateOf { state.firstVisibleItemIndex > titleAt } }
    Box(Modifier.fillMaxSize().background(roles.base)) {
        LazyColumn(Modifier.fillMaxSize(), state = state) {
            item { Spacer(Modifier.windowInsetsTopHeight(WindowInsets.statusBars)) }
            // A pushed page's title sits under the bar and its back button.
            if (nav.canPop) item { Spacer(Modifier.height(52.dp)) }
            item { LargeTitle(title) }
            content()
            item { Spacer(Modifier.height(Under)) }
        }
        NavBar(nav, title, scrolled, actions, account)
    }
}

/** The same with a two-column grid, for covers. */
@Composable
fun GridPage(
    nav: Navigator,
    title: String,
    actions: List<BarAction> = emptyList(),
    account: (@Composable () -> Unit)? = null,
    state: LazyGridState = rememberLazyGridState(),
    content: LazyGridScope.() -> Unit,
) {
    Folding(nav, state.firstVisibleItemIndex to state.firstVisibleItemScrollOffset)
    ScrollsToTop(nav) { state.animateScrollToItem(0) }
    val titleAt = if (nav.canPop) 2 else 1
    val scrolled by remember(titleAt) { derivedStateOf { state.firstVisibleItemIndex > titleAt } }
    Box(Modifier.fillMaxSize().background(roles.base)) {
        LazyVerticalGrid(
            GridCells.Fixed(2),
            Modifier.fillMaxSize(),
            state = state,
            contentPadding = androidx.compose.foundation.layout.PaddingValues(horizontal = 20.dp),
            horizontalArrangement = Arrangement.spacedBy(14.dp),
        ) {
            full { Spacer(Modifier.windowInsetsTopHeight(WindowInsets.statusBars)) }
            if (nav.canPop) full { Spacer(Modifier.height(52.dp)) }
            full { LargeTitle(title) }
            content()
            full { Spacer(Modifier.height(Under)) }
        }
        NavBar(nav, title, scrolled, actions, account)
    }
}

/** A grid item across both columns, out of the grid's side padding. */
fun LazyGridScope.full(content: @Composable () -> Unit) {
    item(span = { GridItemSpan(maxLineSpan) }) {
        Box(Modifier.bleed(20.dp)) { content() }
    }
}

/** Grid cells, 20 dp apart in height as the design spaces them. */
fun Modifier.gridCell() = padding(bottom = 20.dp)

/** Wider than the parent by `by` on each side: across a list's padding. */
fun Modifier.bleed(by: androidx.compose.ui.unit.Dp) = layout { measurable, constraints ->
    val extra = by.roundToPx() * 2
    val wide = constraints.copy(
        minWidth = constraints.minWidth + extra,
        maxWidth = if (constraints.hasBoundedWidth) constraints.maxWidth + extra else constraints.maxWidth,
    )
    val placeable = measurable.measure(wide)
    layout(placeable.width - extra, placeable.height) { placeable.place(-extra / 2, 0) }
}

private fun LazyListState.position() = firstVisibleItemIndex to firstVisibleItemScrollOffset

/** Fold the bottom bar while the page scrolls down past 80 dp, unfold it going up. */
@Composable
private fun Folding(nav: Navigator, position: Pair<Int, Int>) {
    val threshold = with(LocalDensity.current) { 80.dp.roundToPx() }
    val state = remember { object { var last = 0 to 0 } }
    LaunchedEffect(position) {
        val (index, offset) = position
        val (lastIndex, lastOffset) = state.last
        val down = index > lastIndex || (index == lastIndex && offset > lastOffset + 4)
        val up = index < lastIndex || (index == lastIndex && offset < lastOffset - 4)
        if (down && (index > 0 || offset > threshold)) nav.folded = true
        if (up) nav.folded = false
        state.last = position
    }
}

@Composable
private fun ScrollsToTop(nav: Navigator, scroll: suspend () -> Unit) {
    val seen = remember { intArrayOf(nav.toTop) }
    LaunchedEffect(nav.toTop) {
        if (nav.toTop != seen[0]) scroll()
        seen[0] = nav.toTop
    }
}

/**
 * The 52 dp bar under the status bar: back, then the page's buttons on the
 * right. Solid once the page has scrolled, with the title in its middle.
 */
@Composable
fun NavBar(
    nav: Navigator,
    title: String,
    solid: Boolean,
    actions: List<BarAction>,
    account: (@Composable () -> Unit)? = null,
) {
    val fade by animateFloatAsState(if (solid) 1f else 0f, label = "bar")
    Box(
        Modifier
            .fillMaxWidth()
            .background(roles.base.copy(alpha = fade))
            .then(if (solid) Modifier.underline(roles.line) else Modifier)
            .windowInsetsPadding(WindowInsets.statusBars)
            .height(52.dp)
            .padding(horizontal = 12.dp),
    ) {
        if (nav.canPop) {
            Box(Modifier.align(Alignment.CenterStart)) { RoundButton(Icons.Left, { nav.pop() }) }
        }
        Box(Modifier.align(Alignment.Center).padding(horizontal = 58.dp).alpha(fade)) {
            Line(title, geist(16.sp, FontWeight.SemiBold).copy(color = roles.text))
        }
        Row(Modifier.align(Alignment.CenterEnd), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            actions.forEach { RoundButton(it.icon, it.onClick) }
            account?.invoke()
        }
    }
}

/** The account's avatar on root pages: it opens Settings. */
@Composable
fun Avatar(initial: String, onClick: () -> Unit) {
    Box(
        Modifier
            .size(40.dp)
            .press(CircleShape, shrink = 0.92f, onClick = onClick)
            .background(
                androidx.compose.ui.graphics.Brush.linearGradient(listOf(roles.accent, roles.mauve)),
                CircleShape,
            ),
        contentAlignment = Alignment.Center,
    ) {
        Line(initial, geist(17.sp, FontWeight.SemiBold).copy(color = roles.onAccent))
    }
}
