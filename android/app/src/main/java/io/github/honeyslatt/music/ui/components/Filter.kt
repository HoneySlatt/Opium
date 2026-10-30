package io.github.honeyslatt.music.ui.components

import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.core.tween
import androidx.compose.animation.expandVertically
import androidx.compose.animation.shrinkVertically
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.BasicText
import androidx.compose.foundation.text.BasicTextField
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.focus.FocusRequester
import androidx.compose.ui.focus.focusRequester
import androidx.compose.ui.geometry.Rect
import androidx.compose.ui.graphics.SolidColor
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import io.github.honeyslatt.music.ui.Icons
import io.github.honeyslatt.music.ui.theme.geist
import io.github.honeyslatt.music.ui.theme.roles
import java.text.Normalizer

/** For matching and sorting names: lower case, without accents. */
fun fold(text: String): String =
    Normalizer.normalize(text, Normalizer.Form.NFD).replace(Regex("\\p{Mn}+"), "").lowercase()

/** Whether any of `fields` holds the folded `query`; an empty one matches all. */
fun matches(query: String, vararg fields: String?): Boolean {
    val q = fold(query.trim())
    return q.isEmpty() || fields.any { it != null && fold(it).contains(q) }
}

/**
 * A list's search field: folded away until its magnifier is tapped (the
 * keyboard comes with it), and kept open while it holds words.
 */
@Composable
fun SearchReveal(open: Boolean, value: String, placeholder: String, onChange: (String) -> Unit) {
    AnimatedVisibility(open || value.isNotEmpty(), enter = expandVertically(tween(250)), exit = shrinkVertically(tween(250))) {
        val focus = remember { FocusRequester() }
        LaunchedEffect(open) { if (open) runCatching { focus.requestFocus() } }
        Row(
            Modifier
                .padding(start = 20.dp, end = 20.dp, top = 6.dp, bottom = 8.dp)
                .fillMaxWidth()
                .height(40.dp)
                .background(roles.surface, RoundedCornerShape(12.dp))
                .padding(horizontal = 12.dp),
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.spacedBy(8.dp),
        ) {
            Glyph(Icons.Search, roles.sub, 17.dp)
            val style = geist(16.sp).copy(color = roles.text)
            BasicTextField(
                value,
                onChange,
                Modifier.weight(1f).focusRequester(focus),
                textStyle = style,
                singleLine = true,
                cursorBrush = SolidColor(roles.accent),
                decorationBox = { inner ->
                    Box {
                        if (value.isEmpty()) BasicText(placeholder, style = style.copy(color = roles.muted))
                        inner()
                    }
                },
            )
            if (value.isNotEmpty()) {
                Box(Modifier.size(22.dp).background(roles.muted, CircleShape).press(CircleShape) { onChange("") }, contentAlignment = Alignment.Center) {
                    Glyph(Icons.X, roles.base, 12.dp)
                }
            }
        }
    }
}

/** A sort menu: each order, the one in use ticked. */
fun <T> Menus.sort(anchor: Rect, orders: List<T>, current: T, label: (T) -> String, pick: (T) -> Unit) {
    show(MenuSpec(anchor, null) { listOf(orders.map { o -> MenuItem(label(o), null, checked = o == current) { pick(o) } }) })
}
