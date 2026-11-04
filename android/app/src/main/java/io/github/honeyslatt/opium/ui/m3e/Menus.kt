package io.github.honeyslatt.opium.ui.m3e

import androidx.compose.animation.core.Animatable
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.navigationBars
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.layout.windowInsetsPadding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.BasicText
import androidx.compose.foundation.verticalScroll
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.graphics.lerp
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import io.github.honeyslatt.opium.ui.Icons
import io.github.honeyslatt.opium.ui.components.Cover
import io.github.honeyslatt.opium.ui.components.Glyph
import io.github.honeyslatt.opium.ui.components.Line
import io.github.honeyslatt.opium.ui.components.MenuSpec
import io.github.honeyslatt.opium.ui.components.Menus
import io.github.honeyslatt.opium.ui.components.press
import io.github.honeyslatt.opium.ui.theme.geist
import io.github.honeyslatt.opium.ui.theme.roles

/** M3E's surface-container-low, for sheets: between the page and surface-container. */
val containerLow @Composable get() = lerp(roles.base, roles.cell, 0.5f)

/** A segment's corners in an M3E segmented list: 20 dp outside, 4 dp between. */
fun segment(first: Boolean, last: Boolean, outer: Dp = 20.dp, inner: Dp = 4.dp) = RoundedCornerShape(
    topStart = if (first) outer else inner,
    topEnd = if (first) outer else inner,
    bottomStart = if (last) outer else inner,
    bottomEnd = if (last) outer else inner,
)

/**
 * M3E's menu: a bottom sheet on surface-container-low, radius 28 at the
 * top and a 32 × 4 handle, rising on the slow spring over a 40 % scrim;
 * the song or list it is about, then its groups as segmented lists, the
 * icon at the start, what removes in red.
 */
@Composable
fun ExpressiveMenu(spec: MenuSpec, menus: Menus) {
    val rise = remember(spec) { Animatable(1f) }
    LaunchedEffect(spec) { rise.animateTo(0f, Springs.slow()) }
    Box(Modifier.fillMaxSize()) {
        Box(
            Modifier.fillMaxSize().graphicsLayer { alpha = 1f - rise.value }.background(Color.Black.copy(alpha = 0.4f))
                .clickable(remember { MutableInteractionSource() }, indication = null, onClick = menus::close),
        )
        Column(
            Modifier.align(Alignment.BottomCenter).fillMaxWidth()
                .graphicsLayer { translationY = rise.value * size.height }
                .background(containerLow, RoundedCornerShape(topStart = 28.dp, topEnd = 28.dp))
                .clickable(remember { MutableInteractionSource() }, indication = null) {}
                .windowInsetsPadding(WindowInsets.navigationBars)
                .padding(horizontal = 16.dp),
        ) {
            Box(Modifier.fillMaxWidth().padding(top = 12.dp, bottom = 8.dp), contentAlignment = Alignment.Center) {
                Box(Modifier.size(32.dp, 4.dp).background(roles.sub.copy(alpha = 0.5f), RoundedCornerShape(2.dp)))
            }
            spec.head?.let { head ->
                Row(Modifier.padding(horizontal = 4.dp, vertical = 8.dp), verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(14.dp)) {
                    if (head.star) {
                        Box(Modifier.size(56.dp).background(roles.accent, RoundedCornerShape(12.dp)), contentAlignment = Alignment.Center) {
                            Glyph(io.github.honeyslatt.opium.ui.MoreIcons.HeartFill, roles.onAccent, 26.dp)
                        }
                    } else {
                        Cover(head.artwork, 56.dp, RoundedCornerShape(12.dp), Modifier.size(56.dp))
                    }
                    Column {
                        Line(head.title, geist(17.sp, FontWeight.SemiBold).copy(color = roles.text))
                        Line(head.subtitle, geist(14.sp).copy(color = roles.sub))
                    }
                }
            }
            Column(Modifier.verticalScroll(rememberScrollState()).padding(bottom = 16.dp)) {
                spec.groups().filter { it.isNotEmpty() }.forEach { group ->
                    Spacer(Modifier.height(12.dp))
                    group.forEachIndexed { i, item ->
                        if (i > 0) Spacer(Modifier.height(2.dp))
                        val shape = segment(i == 0, i == group.lastIndex)
                        val colour = if (item.danger) roles.red else roles.text
                        Row(
                            Modifier.fillMaxWidth().height(56.dp).background(roles.surface, shape)
                                .press(shape) {
                                    menus.close()
                                    item.onClick()
                                }
                                .padding(horizontal = 16.dp),
                            verticalAlignment = Alignment.CenterVertically,
                            horizontalArrangement = Arrangement.spacedBy(16.dp),
                        ) {
                            item.icon?.let { Glyph(it, if (item.danger) roles.red else roles.sub, 22.dp) } ?: Spacer(Modifier.width(22.dp))
                            BasicText(item.label, Modifier.weight(1f), style = geist(16.sp).copy(color = colour), maxLines = 1)
                            if (item.checked) Glyph(Icons.Check, roles.primary, 20.dp)
                        }
                    }
                }
            }
        }
    }
}
