package io.github.honeyslatt.music.ui.chrome

import androidx.compose.animation.core.CubicBezierEasing
import androidx.compose.animation.core.animateDpAsState
import androidx.compose.animation.core.tween
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.navigationBars
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.layout.windowInsetsPadding
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.BasicText
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.shadow
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.Shape
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import io.github.honeyslatt.music.ui.Icons
import io.github.honeyslatt.music.ui.components.Glyph
import io.github.honeyslatt.music.ui.components.press
import io.github.honeyslatt.music.ui.nav.Navigator
import io.github.honeyslatt.music.ui.nav.Tab
import io.github.honeyslatt.music.ui.theme.geist
import io.github.honeyslatt.music.ui.theme.roles

/** iOS's curve for pushes, sheets and the bar's folding. */
val Standard = CubicBezierEasing(0.32f, 0.72f, 0f, 1f)

/** How tall the chrome is over the page, unfolded and with no mini player. */
val ChromeHeight = 64.dp + 22.dp

/**
 * The opaque capsules that stand in for Liquid Glass: a soft shadow and a
 * hairline edge.
 */
@Composable
fun Modifier.capsule(shape: Shape): Modifier = this
    .shadow(16.dp, shape, ambientColor = Color.Black.copy(alpha = 0.18f), spotColor = Color.Black.copy(alpha = 0.18f))
    .background(roles.capsule, shape)
    .border(1.dp, roles.line, shape)

/**
 * The tab bar and the round Search button, floating 22 dp over the bottom
 * edge. Folded, the tab bar shrinks to the current tab.
 */
@Composable
fun BottomChrome(nav: Navigator, mini: MiniState, modifier: Modifier = Modifier) {
    val folded = nav.folded
    BoxWithConstraints(
        modifier
            .fillMaxWidth()
            .windowInsetsPadding(WindowInsets.navigationBars)
            .padding(start = 14.dp, end = 14.dp, bottom = 22.dp),
    ) {
        val full = maxWidth - 64.dp - 10.dp
        val width by animateDpAsState(if (folded) 64.dp else full, tween(320, easing = Standard), label = "tabbar")
        // Unfolded, the mini player rides 10 dp over the tab bar; folded, it
        // slides down between the folded tab and Search.
        val miniInset by animateDpAsState(if (folded) 74.dp else 0.dp, tween(320, easing = Standard), label = "mini-x")
        val miniLift by animateDpAsState(if (folded) (-3).dp else 74.dp, tween(320, easing = Standard), label = "mini-y")
        MiniPlayer(
            mini,
            folded,
            Modifier
                .padding(start = miniInset, end = miniInset)
                .fillMaxWidth()
                .offset(y = -miniLift),
        )
        Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
            val shape = RoundedCornerShape(32.dp)
            Row(
                Modifier.width(width).height(64.dp).capsule(shape).padding(5.dp),
                horizontalArrangement = Arrangement.Center,
            ) {
                val tabs = if (folded) listOf(nav.barTab) else Tab.entries.filter { it != Tab.Search }
                tabs.forEach { tab ->
                    TabButton(tab, on = tab == nav.tab, folded = folded, Modifier.weight(1f)) {
                        if (folded) nav.folded = false else nav.select(tab)
                    }
                }
            }
            Box(Modifier.weight(1f))
            Box(
                Modifier
                    .size(64.dp)
                    .capsule(CircleShape)
                    .press(CircleShape, shrink = 0.94f) { nav.select(Tab.Search) },
                contentAlignment = Alignment.Center,
            ) {
                Glyph(Icons.Search, if (nav.tab == Tab.Search) roles.tint else roles.text, 24.dp)
            }
        }
    }
}

@Composable
private fun TabButton(tab: Tab, on: Boolean, folded: Boolean, modifier: Modifier, onClick: () -> Unit) {
    val shape = RoundedCornerShape(27.dp)
    val color = if (on) roles.tint else roles.text
    Column(
        modifier
            .fillMaxHeight()
            .press(shape, shrink = 0.94f, onClick = onClick)
            .background(if (on && !folded) roles.surface2 else Color.Transparent, shape),
        horizontalAlignment = Alignment.CenterHorizontally,
        verticalArrangement = Arrangement.spacedBy(3.dp, Alignment.CenterVertically),
    ) {
        Glyph(tab.icon, color, 23.dp)
        if (!folded) BasicText(tab.label, style = geist(10.5.sp, FontWeight.Medium).copy(color = color))
    }
}
