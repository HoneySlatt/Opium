package io.github.honeyslatt.music.ui.components

import androidx.activity.compose.BackHandler
import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.core.tween
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.slideInVertically
import androidx.compose.animation.slideOutVertically
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ColumnScope
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.navigationBars
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.Spacer
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.draw.drawBehind
import androidx.compose.foundation.layout.statusBars
import androidx.compose.foundation.layout.windowInsetsPadding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.BasicText
import androidx.compose.foundation.verticalScroll
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.foundation.gestures.detectVerticalDragGestures
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import io.github.honeyslatt.music.ui.Icons
import io.github.honeyslatt.music.ui.chrome.Standard
import io.github.honeyslatt.music.ui.theme.geist
import io.github.honeyslatt.music.ui.theme.roles

/** A sheet's background: the grouped colour, a shade lifted in the dark; M3E's surface-container-low. */
val sheetColour @Composable get() = when {
    expressiveSheets -> io.github.honeyslatt.music.ui.m3e.containerLow
    roles.dark -> Color(0xFF1C1C1C)
    else -> roles.grouped
}

/** A group's background inside a sheet; M3E's surface-container-high. */
val sheetCell @Composable get() = when {
    expressiveSheets -> roles.surface
    roles.dark -> Color(0xFF262626)
    else -> roles.cell
}

private val expressiveSheets @Composable get() =
    io.github.honeyslatt.music.ui.theme.look.layout.motion == io.github.honeyslatt.music.core.model.Motion.EXPRESSIVE

/**
 * A sheet rising from the bottom with 26 dp corners over a scrim, as tall as
 * its content up to under the status bar. A tap on the scrim, the close
 * button or Back closes it.
 */
@Composable
fun Sheet(
    open: Boolean,
    title: String?,
    onClose: () -> Unit,
    left: (@Composable () -> Unit)? = null,
    right: (@Composable () -> Unit)? = null,
    content: @Composable ColumnScope.() -> Unit,
) {
    BackHandler(open, onClose)
    Box(Modifier.fillMaxSize()) {
        AnimatedVisibility(open, enter = fadeIn(tween(300)), exit = fadeOut(tween(300))) {
            val scrim = when {
                expressiveSheets -> Color.Black.copy(alpha = 0.4f)
                roles.dark -> Color.Black.copy(alpha = 0.5f)
                else -> Color(0x47161616)
            }
            Box(
                Modifier
                    .fillMaxSize()
                    .background(scrim)
                    .clickable(remember { MutableInteractionSource() }, indication = null, onClick = onClose),
            )
        }
        AnimatedVisibility(
            open,
            Modifier.align(Alignment.BottomCenter),
            enter = slideInVertically(tween(400, easing = Standard)) { it },
            exit = slideOutVertically(tween(380, easing = Standard)) { it },
        ) {
            BoxWithConstraints(Modifier.fillMaxSize().windowInsetsPadding(WindowInsets.statusBars), contentAlignment = Alignment.BottomCenter) {
                val m3e = expressiveSheets
                val shape = RoundedCornerShape(topStart = if (m3e) 28.dp else 26.dp, topEnd = if (m3e) 28.dp else 26.dp)
                var drag by remember { mutableFloatStateOf(0f) }
                val shown by animateFloatAsState(drag, label = "sheet")
                val limit = with(LocalDensity.current) { 90.dp.toPx() }
                Column(
                    Modifier
                        .graphicsLayer { translationY = shown }
                        // Pulled down far enough, it closes; Now Playing
                        // behind stays where it is.
                        .pointerInput(Unit) {
                            detectVerticalDragGestures(
                                onDragEnd = { if (drag > limit) onClose() else drag = 0f },
                                onDragCancel = { drag = 0f },
                            ) { change, amount ->
                                drag = (drag + amount).coerceAtLeast(0f)
                                change.consume()
                            }
                        }
                        .fillMaxWidth()
                        .heightIn(max = maxHeight - 46.dp)
                        .background(sheetColour, shape)
                        .then(if (roles.dark && !m3e) Modifier.border(1.dp, roles.line, shape) else Modifier)
                        // The sheet keeps its touches from the scrim.
                        .clickable(remember { MutableInteractionSource() }, indication = null) {},
                ) {
                    // M3E's handle.
                    if (m3e) {
                        Box(Modifier.fillMaxWidth().padding(top = 12.dp), contentAlignment = Alignment.Center) {
                            Box(Modifier.size(32.dp, 4.dp).background(roles.sub.copy(alpha = 0.5f), RoundedCornerShape(2.dp)))
                        }
                    }
                    // No title: a sheet of its own words, as a confirmation.
                    if (title != null) {
                        Box(Modifier.fillMaxWidth().height(60.dp).padding(start = 14.dp, end = 14.dp, top = 8.dp)) {
                            Box(Modifier.align(Alignment.CenterStart)) {
                                if (left != null) {
                                    left()
                                } else {
                                    Box(
                                        Modifier.size(40.dp).press(CircleShape, shrink = 0.92f, onClick = onClose).background(roles.surface, CircleShape),
                                        contentAlignment = Alignment.Center,
                                    ) { Glyph(Icons.X, roles.text, 19.dp) }
                                }
                            }
                            BasicText(
                                title,
                                Modifier.align(Alignment.Center).padding(horizontal = 80.dp),
                                style = geist(16.5.sp, FontWeight.SemiBold).copy(color = roles.text, textAlign = TextAlign.Center),
                                maxLines = 1,
                            )
                            right?.let { Box(Modifier.align(Alignment.CenterEnd)) { it() } }
                        }
                    } else {
                        Spacer(Modifier.height(14.dp))
                    }
                    Column(
                        Modifier
                            .verticalScroll(rememberScrollState())
                            .padding(start = 16.dp, end = 16.dp, top = 8.dp, bottom = 40.dp)
                            .windowInsetsPadding(WindowInsets.navigationBars),
                        content = content,
                    )
                }
            }
        }
    }
}

/** Rows on a rounded card, with an optional note under it. */
@Composable
fun Group(footer: String? = null, content: @Composable ColumnScope.() -> Unit) {
    Column(Modifier.padding(bottom = 26.dp)) {
        Column(Modifier.fillMaxWidth().background(sheetCell, RoundedCornerShape(16.dp)), content = content)
        footer?.let {
            BasicText(
                it,
                Modifier.padding(start = 16.dp, end = 16.dp, top = 7.dp),
                style = geist(12.5.sp).copy(color = roles.muted, lineHeight = 17.5.sp),
            )
        }
    }
}

/** A sheet's text button: grey, or the accent for what it is for. */
@Composable
fun SheetButton(label: String, primary: Boolean = false, enabled: Boolean = true, onClick: () -> Unit) {
    val shape = RoundedCornerShape(20.dp)
    Box(
        Modifier
            .height(40.dp)
            .alpha(if (enabled) 1f else 0.4f)
            .then(if (enabled) Modifier.press(shape, shrink = 0.94f, onClick = onClick) else Modifier)
            .background(if (primary) roles.accent else roles.surface, shape)
            .padding(horizontal = 14.dp),
        contentAlignment = Alignment.Center,
    ) {
        BasicText(label, style = geist(15.sp, FontWeight.SemiBold).copy(color = if (primary) roles.onAccent else roles.text))
    }
}

/** A row of a group: a press, a hairline above all but the first. */
@Composable
fun GroupRow(first: Boolean, inset: androidx.compose.ui.unit.Dp = 16.dp, onClick: (() -> Unit)? = null, content: @Composable androidx.compose.foundation.layout.RowScope.() -> Unit) {
    val line = roles.line
    androidx.compose.foundation.layout.Row(
        Modifier
            .fillMaxWidth()
            .heightIn(min = 52.dp)
            .drawBehind { if (!first) drawLine(line, androidx.compose.ui.geometry.Offset(inset.toPx(), 0f), androidx.compose.ui.geometry.Offset(size.width, 0f), 1.dp.toPx()) }
            .then(if (onClick != null) Modifier.press(onClick = onClick) else Modifier)
            .padding(horizontal = 16.dp, vertical = 8.dp),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = androidx.compose.foundation.layout.Arrangement.spacedBy(12.dp),
        content = content,
    )
}
