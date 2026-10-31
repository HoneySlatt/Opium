package io.github.honeyslatt.music.ui.components

import androidx.compose.animation.core.Animatable
import androidx.compose.animation.core.tween
import androidx.compose.foundation.IndicationNodeFactory
import androidx.compose.foundation.interaction.InteractionSource
import androidx.compose.foundation.interaction.PressInteraction
import androidx.compose.ui.graphics.drawOutline
import androidx.compose.ui.graphics.drawscope.ContentDrawScope
import androidx.compose.ui.graphics.drawscope.scale
import androidx.compose.ui.node.DelegatableNode
import androidx.compose.ui.node.DrawModifierNode
import androidx.compose.ui.node.invalidateDraw
import kotlinx.coroutines.launch
import androidx.compose.foundation.Image
import androidx.compose.foundation.background
import androidx.compose.foundation.combinedClickable
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.text.BasicText
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
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
 * in `shape` and, with `shrink`, a slight scale down while held. Drawn by a
 * node, so a press redraws the row without recomposing it.
 */
@Composable
fun Modifier.press(
    shape: Shape = RectangleShape,
    shrink: Float = 1f,
    onLongClick: (() -> Unit)? = null,
    onClick: () -> Unit,
): Modifier {
    val overlay = roles.press
    // Spotify's: whatever is held gives a little, and springs back.
    val spotify = io.github.honeyslatt.music.ui.theme.look.layout.motion == io.github.honeyslatt.music.core.model.Motion.SPOTIFY
    val give = if (spotify && shrink == 1f) 0.97f else shrink
    val indication = remember(shape, give, overlay, spotify) { Press(shape, give, overlay, spotify) }
    // A long press, felt.
    val haptics = androidx.compose.ui.platform.LocalHapticFeedback.current
    return combinedClickable(
        interactionSource = null,
        indication = indication,
        onLongClick = onLongClick?.let { long ->
            {
                haptics.performHapticFeedback(androidx.compose.ui.hapticfeedback.HapticFeedbackType.LongPress)
                long()
            }
        },
        onClick = onClick,
    )
}

private data class Press(val shape: Shape, val shrink: Float, val overlay: Color, val bounce: Boolean) : IndicationNodeFactory {
    override fun create(interactionSource: InteractionSource): DelegatableNode =
        PressNode(interactionSource, shape, shrink, overlay, bounce)
}

private class PressNode(
    private val source: InteractionSource,
    private val shape: Shape,
    private val shrink: Float,
    private val overlay: Color,
    private val bounce: Boolean,
) : Modifier.Node(), DrawModifierNode {
    private val scale = Animatable(1f)
    private var pressed = false

    override fun onAttach() {
        coroutineScope.launch {
            source.interactions.collect {
                when (it) {
                    is PressInteraction.Press -> held(true)
                    is PressInteraction.Release, is PressInteraction.Cancel -> held(false)
                }
            }
        }
    }

    private fun held(on: Boolean) {
        pressed = on
        if (shrink != 1f) {
            coroutineScope.launch {
                when {
                    on -> scale.animateTo(shrink, tween(if (bounce) 90 else 120))
                    bounce -> scale.animateTo(1f, androidx.compose.animation.core.spring(dampingRatio = 0.45f, stiffness = 700f))
                    else -> scale.animateTo(1f, tween(120))
                }
            }
        }
        invalidateDraw()
    }

    override fun ContentDrawScope.draw() {
        scale(scale.value) {
            this@draw.drawContent()
            if (pressed && shrink == 1f) drawOutline(shape.createOutline(size, layoutDirection, this), overlay)
        }
    }
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
