package io.github.honeyslatt.music.ui.components

import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.core.tween
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.scaleIn
import androidx.compose.animation.scaleOut
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.BasicText
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.shadow
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import io.github.honeyslatt.music.ui.Icons
import io.github.honeyslatt.music.ui.theme.geist
import io.github.honeyslatt.music.ui.theme.roles
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Job
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch

/** A word on what just happened, in the middle of the screen for 1.5 s. */
class Hud(private val scope: CoroutineScope) {
    data class Message(val text: String, val icon: ImageVector)

    var message by mutableStateOf<Message?>(null)
        private set
    var shown by mutableStateOf(false)
        private set
    private var hiding: Job? = null

    fun show(text: String, icon: ImageVector = Icons.Check) {
        message = Message(text, icon)
        shown = true
        hiding?.cancel()
        hiding = scope.launch {
            delay(1_500)
            shown = false
        }
    }
}

/** The HUD: a 156 dp opaque square with an icon and a short label. */
@Composable
fun HudHost(hud: Hud) {
    Box(Modifier.fillMaxSize().padding(bottom = 130.dp), contentAlignment = Alignment.Center) {
        AnimatedVisibility(
            hud.shown,
            enter = fadeIn(tween(200)) + scaleIn(tween(200), initialScale = 0.9f),
            exit = fadeOut(tween(200)) + scaleOut(tween(200), targetScale = 0.9f),
        ) {
            val m = hud.message ?: return@AnimatedVisibility
            val shape = RoundedCornerShape(20.dp)
            Column(
                Modifier
                    .width(156.dp)
                    .shadow(24.dp, shape, ambientColor = Color.Black.copy(alpha = 0.35f), spotColor = Color.Black.copy(alpha = 0.35f))
                    .background(roles.capsule, shape)
                    .border(1.dp, roles.line, shape)
                    .padding(start = 12.dp, end = 12.dp, top = 22.dp, bottom = 18.dp),
                horizontalAlignment = Alignment.CenterHorizontally,
                verticalArrangement = Arrangement.spacedBy(10.dp),
            ) {
                Glyph(m.icon, roles.text, 44.dp)
                BasicText(m.text, style = geist(15.sp, FontWeight.SemiBold).copy(color = roles.text, textAlign = TextAlign.Center))
            }
        }
    }
}
