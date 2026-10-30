package io.github.honeyslatt.music.ui.components

import androidx.activity.compose.BackHandler
import androidx.compose.animation.core.Animatable
import androidx.compose.animation.core.tween
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.BasicText
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.runtime.staticCompositionLocalOf
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.drawBehind
import androidx.compose.ui.draw.shadow
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Rect
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.TransformOrigin
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.layout.Layout
import androidx.compose.ui.layout.LayoutCoordinates
import androidx.compose.ui.layout.boundsInRoot
import androidx.compose.ui.layout.onGloballyPositioned
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.Constraints
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import io.github.honeyslatt.music.core.model.Artwork
import io.github.honeyslatt.music.ui.Icons
import io.github.honeyslatt.music.ui.theme.geist
import io.github.honeyslatt.music.ui.theme.roles

/** A choice of a menu; destructive ones are red, the one in use ticked. */
class MenuItem(val label: String, val icon: ImageVector?, val danger: Boolean = false, val checked: Boolean = false, val onClick: () -> Unit)

/** What a menu is about, above its choices. */
class MenuHead(val artwork: Artwork?, val title: String, val subtitle: String, val star: Boolean = false)

/**
 * A menu by what it was opened from. Its groups are read as it shows, so a
 * state they read (a favourite found out late) changes them.
 */
class MenuSpec(val anchor: Rect, val head: MenuHead?, val groups: () -> List<List<MenuItem>>)

/** The menu open, if one is. */
class Menus {
    var open by mutableStateOf<MenuSpec?>(null)
        private set

    fun show(spec: MenuSpec) {
        open = spec
    }

    fun close() {
        open = null
    }
}

val LocalMenus = staticCompositionLocalOf { Menus() }

/** Where something is on screen, for a menu to open by it. */
class Anchor {
    internal var coordinates: LayoutCoordinates? = null
    val rect: Rect get() = coordinates?.takeIf { it.isAttached }?.boundsInRoot() ?: Rect.Zero
}

@Composable
fun rememberAnchor() = remember { Anchor() }

fun Modifier.anchor(anchor: Anchor) = onGloballyPositioned { anchor.coordinates = it }

/**
 * The menu: 270 dp wide, under what opened it (over it when there's no
 * room), popping out of it over a light scrim.
 */
@Composable
fun MenuHost(menus: Menus) {
    val spec = menus.open ?: return
    BackHandler(onBack = menus::close)
    val pop = remember(spec) { Animatable(0f) }
    LaunchedEffect(spec) { pop.animateTo(1f, tween(200, easing = io.github.honeyslatt.music.ui.chrome.Standard)) }
    Box(
        Modifier
            .fillMaxSize()
            .background(Color.Black.copy(alpha = 0.18f))
            .clickable(remember { MutableInteractionSource() }, indication = null, onClick = menus::close),
    )
    var origin by remember(spec) { mutableStateOf(TransformOrigin.Center) }
    Layout(
        content = {
            MenuCard(
                spec,
                menus,
                Modifier.graphicsLayer {
                    alpha = pop.value
                    val s = 0.88f + 0.12f * pop.value
                    scaleX = s
                    scaleY = s
                    transformOrigin = origin
                },
            )
        },
    ) { measurables, constraints ->
        val placeable = measurables.first().measure(Constraints(maxWidth = 270.dp.roundToPx(), maxHeight = constraints.maxHeight))
        val w = placeable.width
        val h = placeable.height
        val a = spec.anchor
        val gap = 6.dp.toPx()
        var top = a.bottom + gap
        if (top + h > constraints.maxHeight - 30.dp.toPx()) top = maxOf(44.dp.toPx(), a.top - h - gap)
        val left = (a.right - w).coerceIn(12.dp.toPx(), constraints.maxWidth - w - 12.dp.toPx())
        origin = TransformOrigin(
            ((a.center.x - left) / w).coerceIn(0f, 1f),
            if (top > a.top) 0f else 1f,
        )
        layout(constraints.maxWidth, constraints.maxHeight) { placeable.place(left.toInt(), top.toInt()) }
    }
}

@Composable
private fun MenuCard(spec: MenuSpec, menus: Menus, modifier: Modifier) {
    val shape = RoundedCornerShape(18.dp)
    val separator = if (roles.dark) Color(0xFF101010) else roles.surface
    Column(
        modifier
            .width(270.dp)
            .shadow(30.dp, shape, ambientColor = Color.Black.copy(alpha = 0.4f), spotColor = Color.Black.copy(alpha = 0.4f))
            .background(roles.capsule, shape)
            .border(1.dp, roles.line, shape),
    ) {
        spec.head?.let { head ->
            Row(Modifier.padding(horizontal = 14.dp, vertical = 12.dp), verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(12.dp)) {
                if (head.star) {
                    Box(Modifier.size(44.dp).background(roles.surface, RoundedCornerShape(7.dp)), contentAlignment = Alignment.Center) {
                        Glyph(Icons.Starf, roles.tint, 22.dp)
                    }
                } else {
                    Cover(head.artwork, 44.dp, RoundedCornerShape(7.dp), Modifier.size(44.dp))
                }
                Column {
                    Line(head.title, geist(15.sp, FontWeight.SemiBold).copy(color = roles.text))
                    Line(head.subtitle, geist(13.sp).copy(color = roles.sub))
                }
            }
            Box(Modifier.fillMaxWidth().height(7.dp).background(separator))
        }
        val line = roles.line
        spec.groups().filter { it.isNotEmpty() }.forEachIndexed { g, group ->
            if (g > 0) Box(Modifier.fillMaxWidth().height(7.dp).background(separator))
            group.forEachIndexed { i, item ->
                val colour = if (item.danger) roles.red else roles.text
                Row(
                    Modifier
                        .fillMaxWidth()
                        .height(46.dp)
                        .drawBehind { if (i > 0) drawLine(line, Offset(0f, 0f), Offset(size.width, 0f), 1.dp.toPx()) }
                        .press {
                            menus.close()
                            item.onClick()
                        }
                        .padding(horizontal = 16.dp),
                    verticalAlignment = Alignment.CenterVertically,
                    horizontalArrangement = Arrangement.spacedBy(12.dp),
                ) {
                    BasicText(item.label, Modifier.weight(1f), style = geist(16.sp).copy(color = colour), maxLines = 1)
                    item.icon?.let { Glyph(it, colour, 19.dp) }
                    if (item.checked) Glyph(Icons.Check, roles.tint, 18.dp)
                }
            }
        }
    }
}
