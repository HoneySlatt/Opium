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

/** A sheet's background: the grouped colour, a shade lifted in the dark. */
val sheetColour @Composable get() = if (roles.dark) Color(0xFF131118) else roles.grouped

/** A group's background inside a sheet. */
val sheetCell @Composable get() = if (roles.dark) Color(0xFF1E1B26) else roles.cell

/**
 * A sheet rising from the bottom with 26 dp corners over a scrim, as tall as
 * its content up to under the status bar. A tap on the scrim, the close
 * button or Back closes it.
 */
@Composable
fun Sheet(open: Boolean, title: String, onClose: () -> Unit, content: @Composable ColumnScope.() -> Unit) {
    BackHandler(open, onClose)
    Box(Modifier.fillMaxSize()) {
        AnimatedVisibility(open, enter = fadeIn(tween(300)), exit = fadeOut(tween(300))) {
            val scrim = if (roles.dark) Color.Black.copy(alpha = 0.5f) else Color(0x471E103C)
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
                val shape = RoundedCornerShape(topStart = 26.dp, topEnd = 26.dp)
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
                        .then(if (roles.dark) Modifier.border(1.dp, roles.line, shape) else Modifier)
                        // The sheet keeps its touches from the scrim.
                        .clickable(remember { MutableInteractionSource() }, indication = null) {},
                ) {
                    Box(Modifier.fillMaxWidth().height(60.dp).padding(start = 14.dp, end = 14.dp, top = 8.dp)) {
                        Box(
                            Modifier.align(Alignment.CenterStart).size(40.dp).press(CircleShape, shrink = 0.92f, onClick = onClose).background(roles.surface, CircleShape),
                            contentAlignment = Alignment.Center,
                        ) { Glyph(Icons.X, roles.text, 19.dp) }
                        BasicText(
                            title,
                            Modifier.align(Alignment.Center).padding(horizontal = 56.dp),
                            style = geist(16.5.sp, FontWeight.SemiBold).copy(color = roles.text, textAlign = TextAlign.Center),
                            maxLines = 1,
                        )
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
