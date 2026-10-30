package io.github.honeyslatt.music.ui.components

import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.animation.core.tween
import androidx.compose.foundation.Image
import androidx.compose.foundation.background
import androidx.compose.foundation.combinedClickable
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.interaction.collectIsPressedAsState
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.text.BasicText
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
import androidx.compose.ui.composed
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.drawWithContent
import androidx.compose.ui.draw.scale
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.ColorFilter
import androidx.compose.ui.graphics.RectangleShape
import androidx.compose.ui.graphics.Shape
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.graphics.vector.rememberVectorPainter
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import io.github.honeyslatt.music.ui.theme.roles

/** An icon of the set, in one colour. */
@Composable
fun Glyph(icon: ImageVector, tint: Color, size: Dp = 20.dp, modifier: Modifier = Modifier) {
    Image(
        rememberVectorPainter(icon),
        contentDescription = null,
        modifier = modifier.size(size),
        colorFilter = ColorFilter.tint(tint),
    )
}

/** One line of text, cut with an ellipsis. */
@Composable
fun Line(text: String, style: TextStyle, modifier: Modifier = Modifier) {
    BasicText(text, modifier, style = style, maxLines = 1, overflow = TextOverflow.Ellipsis)
}

/**
 * A tap target in the design's manner: no Material ripple, a press overlay
 * in `shape` and, with `shrink`, a slight scale down while held.
 */
fun Modifier.press(
    shape: Shape = RectangleShape,
    shrink: Float = 1f,
    onLongClick: (() -> Unit)? = null,
    onClick: () -> Unit,
): Modifier = composed {
    val source = remember { MutableInteractionSource() }
    val pressed by source.collectIsPressedAsState()
    val scale by animateFloatAsState(if (pressed) shrink else 1f, tween(120), label = "press")
    val overlay = roles.press
    this
        .scale(scale)
        .clip(shape)
        .drawWithContent {
            drawContent()
            if (pressed && shrink == 1f) drawRect(overlay)
        }
        .combinedClickable(
            interactionSource = source,
            indication = null,
            onLongClick = onLongClick,
            onClick = onClick,
        )
}

/** A round 40 dp button of the navigation bar. */
@Composable
fun RoundButton(icon: ImageVector, onClick: () -> Unit, fill: Color = roles.surface, size: Dp = 40.dp) {
    Box(
        Modifier
            .size(size)
            .press(androidx.compose.foundation.shape.CircleShape, shrink = 0.92f, onClick = onClick)
            .background(fill, androidx.compose.foundation.shape.CircleShape),
        contentAlignment = androidx.compose.ui.Alignment.Center,
    ) {
        Glyph(icon, roles.text, 20.dp)
    }
}
