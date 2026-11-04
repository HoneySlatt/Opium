package io.github.honeyslatt.opium.ui.connect

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.safeDrawing
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.windowInsetsPadding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.text.BasicText
import androidx.compose.foundation.verticalScroll
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.drawBehind
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import io.github.honeyslatt.opium.core.model.ProviderKind
import io.github.honeyslatt.opium.data.Connection
import io.github.honeyslatt.opium.data.Core
import io.github.honeyslatt.opium.ui.components.Group
import io.github.honeyslatt.opium.ui.components.GroupRow
import io.github.honeyslatt.opium.ui.components.ServiceLogo
import io.github.honeyslatt.opium.ui.components.SheetButton
import io.github.honeyslatt.opium.ui.components.label
import io.github.honeyslatt.opium.ui.theme.geist
import io.github.honeyslatt.opium.ui.theme.roles

/**
 * The first launch: a pink glow, the three services, and Continue once
 * one of them is connected.
 */
@Composable
fun Welcome(core: Core, connecting: Connecting, onContinue: () -> Unit) {
    val accent = roles.accent
    val mauve = roles.mauve
    Box(
        Modifier
            .fillMaxSize()
            .background(roles.base)
            .drawBehind {
                val h = 420.dp.toPx()
                drawRect(Brush.radialGradient(listOf(accent.copy(alpha = 0.34f), Color.Transparent), Offset(size.width * 0.18f, 0f), size.width * 0.7f), size = size.copy(height = h))
                drawRect(Brush.radialGradient(listOf(mauve.copy(alpha = 0.26f), Color.Transparent), Offset(size.width * 0.7f, 0f), size.width * 0.6f), size = size.copy(height = h))
            },
    ) {
        Column(
            Modifier
                .fillMaxSize()
                .windowInsetsPadding(WindowInsets.safeDrawing)
                .verticalScroll(rememberScrollState())
                .padding(start = 20.dp, end = 20.dp, top = 60.dp, bottom = 40.dp),
        ) {
            BasicText("All your music,\nin one library.", style = geist(32.sp, FontWeight.Bold, -0.02f).copy(color = roles.text, lineHeight = 35.sp))
            BasicText(
                "Connect the services you use. Songs, albums and playlists from each of them share the same pages, and play from the best copy you have.",
                Modifier.padding(top = 10.dp, bottom = 28.dp),
                style = geist(15.5.sp).copy(color = roles.sub, lineHeight = 22.sp),
            )
            val states = io.github.honeyslatt.opium.data.SERVICES.associateWith { core.connection(it).collectAsStateWithLifecycle().value }
            Group("You can connect or turn off a service later, from your account.") {
                io.github.honeyslatt.opium.data.SERVICES.forEachIndexed { i, kind ->
                    GroupRow(first = i == 0, inset = 58.dp) {
                        ServiceLogo(kind, 30.dp)
                        Column(Modifier.weight(1f)) {
                            BasicText(kind.label(), style = geist(16.sp).copy(color = roles.text))
                            BasicText(
                                when (kind) {
                                    ProviderKind.APPLE_MUSIC -> "Main service · AAC 256"
                                    ProviderKind.SOUND_CLOUD -> "Optional · your likes and the uploaders’ files"
                                    ProviderKind.NAVIDROME -> "Optional · lossless, from your server"
                                    ProviderKind.OPIUM, ProviderKind.LOCAL -> ""
                                },
                                style = geist(13.sp).copy(color = roles.sub),
                            )
                        }
                        when (states.getValue(kind)) {
                            is Connection.Connected -> Row(verticalAlignment = Alignment.CenterVertically) {
                                Box(Modifier.size(7.dp).background(roles.green, CircleShape))
                                BasicText("Connected", Modifier.padding(start = 6.dp), style = geist(14.sp).copy(color = roles.sub))
                            }
                            Connection.Connecting -> BasicText("Connecting…", style = geist(14.sp).copy(color = roles.sub))
                            else -> SheetButton("Connect", primary = kind == ProviderKind.APPLE_MUSIC) { connecting.start(kind) }
                        }
                    }
                }
            }
            Spacer(Modifier.height(8.dp))
            BigButton("Continue", enabled = states.values.any { it is Connection.Connected }, onClick = onContinue)
        }
    }
}
