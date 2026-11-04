package io.github.honeyslatt.opium.ui.nowplaying

import androidx.compose.foundation.background
import androidx.compose.foundation.gestures.detectHorizontalDragGestures
import androidx.compose.foundation.gestures.detectTapGestures
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ColumnScope
import androidx.compose.foundation.layout.RowScope
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.BasicText
import androidx.compose.foundation.text.BasicTextField
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.focus.FocusRequester
import androidx.compose.ui.focus.focusRequester
import androidx.compose.ui.graphics.SolidColor
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.media3.common.util.UnstableApi
import io.github.honeyslatt.opium.core.connect.DeviceKind
import io.github.honeyslatt.opium.core.connect.Peer
import io.github.honeyslatt.opium.data.Devices
import io.github.honeyslatt.opium.data.Pairing
import io.github.honeyslatt.opium.playback.Playback
import io.github.honeyslatt.opium.ui.Icons
import io.github.honeyslatt.opium.ui.components.Glyph
import io.github.honeyslatt.opium.ui.components.Group
import io.github.honeyslatt.opium.ui.components.GroupRow
import io.github.honeyslatt.opium.ui.components.Sheet
import io.github.honeyslatt.opium.ui.components.SheetButton
import io.github.honeyslatt.opium.ui.theme.geist
import io.github.honeyslatt.opium.ui.theme.roles

fun Peer.icon() = if (device.kind == DeviceKind.PHONE) Icons.Phone else Icons.Monitor

/** What a device does, in a few words. */
private fun Peer.status(): String {
    val p = playing
    return when {
        !connected -> "Not on the network"
        p?.track == null -> "Nothing playing"
        p.playing -> "Playing “${p.track!!.title}”"
        else -> "Paused · “${p.track!!.title}”"
    }
}

/**
 * Play On: this phone, then the devices paired and on the Wi-Fi; a tap
 * moves the music there, as with Spotify's devices. Those not paired yet
 * are below, with Pair.
 */
@androidx.annotation.OptIn(UnstableApi::class)
@Composable
fun DevicesSheet(open: Boolean, playback: Playback, onClose: () -> Unit) {
    val context = LocalContext.current
    val peers by playback.devices.peers.collectAsStateWithLifecycle()
    val remote by playback.remote.collectAsStateWithLifecycle()
    val output = remember(open) { Outputs.current(context) }
    Sheet(open, "Play On", onClose) {
        val paired = peers.filter { it.paired && it.connected }
        Group {
            GroupRow(first = true, onClick = { playback.playOn(null); onClose() }) {
                Glyph(output.icon, roles.text, 22.dp)
                Words("This Phone", output.name.takeIf { output.kind != OutputKind.Speaker })
                if (remote == null) Glyph(Icons.Check, roles.accent, 20.dp)
            }
            paired.forEach { peer ->
                GroupRow(first = false, onClick = { playback.playOn(peer.device.id); onClose() }) {
                    Glyph(peer.icon(), roles.text, 22.dp)
                    Words(peer.device.name, peer.status())
                    if (remote?.device?.id == peer.device.id) Glyph(Icons.Check, roles.accent, 20.dp)
                }
            }
        }
        remote?.let { r ->
            r.playing?.volume?.let { volume ->
                Label("Volume on ${r.device.name}")
                Group { Box(Modifier.padding(16.dp)) { Volume(volume) { playback.setRemoteVolume(it) } } }
            }
        }
        val nearby = peers.filter { !it.paired && it.nearby }
        if (nearby.isNotEmpty()) {
            Label("Nearby")
            Group("Pair once: it shows a code to type in here.") {
                nearby.forEachIndexed { i, peer ->
                    GroupRow(first = i == 0) {
                        Glyph(peer.icon(), roles.text, 22.dp)
                        Words(peer.device.name, null)
                        SheetButton("Pair", primary = true) { playback.devices.pair(peer.device.id) }
                    }
                }
            }
        } else if (paired.isEmpty()) {
            Group("Opium on your computer shows here once it is open on the same Wi-Fi.") {}
        }
        Group {
            GroupRow(first = true, onClick = { Outputs.showSwitcher(context) }) {
                Glyph(Icons.Headphones, roles.text, 22.dp)
                Words("This phone's output", "Bluetooth, USB and the speaker, as Android switches them.")
                Glyph(Icons.Right, roles.muted, 18.dp)
            }
        }
    }
}

/** A bar to drag: what it says goes out as it moves. */
@Composable
private fun Volume(volume: Float, onChange: (Float) -> Unit) {
    var dragging by remember { mutableStateOf<Float?>(null) }
    var sent by remember { mutableFloatStateOf(volume) }
    val shown = dragging ?: volume
    fun set(v: Float) {
        dragging = v
        if (kotlin.math.abs(v - sent) >= 0.01f) {
            sent = v
            onChange(v)
        }
    }
    androidx.compose.foundation.layout.Row(verticalAlignment = Alignment.CenterVertically) {
        Glyph(Icons.Vlow, roles.sub, 18.dp)
        BoxWithConstraints(Modifier.weight(1f).padding(horizontal = 12.dp).height(28.dp), contentAlignment = Alignment.CenterStart) {
            val width = constraints.maxWidth.toFloat()
            Box(
                Modifier
                    .fillMaxWidth()
                    .height(28.dp)
                    .pointerInput(Unit) {
                        detectHorizontalDragGestures(
                            onDragStart = { set((it.x / width).coerceIn(0f, 1f)) },
                            onDragEnd = { dragging?.let(onChange); dragging = null },
                            onDragCancel = { dragging = null },
                        ) { change, _ -> set((change.position.x / width).coerceIn(0f, 1f)) }
                    }
                    .pointerInput(Unit) { detectTapGestures { onChange((it.x / width).coerceIn(0f, 1f)) } },
                contentAlignment = Alignment.CenterStart,
            ) {
                Box(Modifier.fillMaxWidth().height(7.dp).background(roles.surface3, RoundedCornerShape(4.dp))) {
                    Box(Modifier.fillMaxWidth(shown.coerceIn(0f, 1f)).height(7.dp).background(roles.accent, RoundedCornerShape(4.dp)))
                }
            }
        }
        Glyph(Icons.Vhigh, roles.sub, 18.dp)
    }
}

/**
 * Pairing, wherever the app is: the code another device shows, to type in
 * here; or the code to type into the device asking.
 */
@Composable
fun PairingHost(devices: Devices) {
    val pairing by devices.pairing.collectAsStateWithLifecycle()
    // Kept while the sheet goes down.
    var last by remember { mutableStateOf<Pairing?>(null) }
    pairing?.let { last = it }
    val shown = last
    Sheet(pairing != null, shown?.let { "Pair with ${it.device.name}" }, onClose = devices::cancelPairing) {
        when (shown) {
            is Pairing.Asking -> CodeEntry(shown, devices)
            is Pairing.Showing -> {
                Note("Type this code on ${shown.device.name}.")
                BasicText(
                    "${shown.code.take(3)} ${shown.code.drop(3)}",
                    Modifier.fillMaxWidth().padding(vertical = 28.dp),
                    style = geist(44.sp, FontWeight.Bold).copy(color = roles.text, textAlign = TextAlign.Center),
                )
            }
            null -> {}
        }
    }
}

@Composable
private fun ColumnScope.CodeEntry(asking: Pairing.Asking, devices: Devices) {
    var code by remember(asking) { mutableStateOf("") }
    val focus = remember { FocusRequester() }
    LaunchedEffect(asking) { runCatching { focus.requestFocus() } }
    Note("Type the code ${asking.device.name} shows.")
    val style = geist(40.sp, FontWeight.Bold, 0.2f).copy(color = roles.text, textAlign = TextAlign.Center)
    BasicTextField(
        code,
        { typed -> code = typed.filter(Char::isDigit).take(6) },
        Modifier.fillMaxWidth().padding(vertical = 24.dp).focusRequester(focus),
        textStyle = style,
        singleLine = true,
        cursorBrush = SolidColor(roles.accent),
        keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.NumberPassword),
        decorationBox = { inner ->
            Box(contentAlignment = Alignment.Center) {
                if (code.isEmpty()) BasicText("000000", style = style.copy(color = roles.muted.copy(alpha = 0.4f)))
                inner()
            }
        },
    )
    Box(Modifier.align(Alignment.CenterHorizontally).width(160.dp)) {
        SheetButton("Pair", primary = true, enabled = code.length == 6) { devices.enterCode(code) }
    }
}

@Composable
private fun Note(text: String) {
    BasicText(
        text,
        Modifier.fillMaxWidth().padding(horizontal = 8.dp),
        style = geist(15.sp).copy(color = roles.sub, textAlign = TextAlign.Center),
    )
}

@Composable
private fun Label(text: String) {
    BasicText(text, Modifier.padding(start = 16.dp, bottom = 7.dp), style = geist(13.sp, FontWeight.Medium).copy(color = roles.sub))
}

/** A row's words: its name, and a line under it. */
@Composable
private fun RowScope.Words(title: String, detail: String?) {
    Column(Modifier.weight(1f)) {
        BasicText(title, style = geist(16.sp).copy(color = roles.text))
        detail?.let { BasicText(it, Modifier.padding(top = 1.dp), style = geist(13.sp).copy(color = roles.sub, lineHeight = 17.5.sp), maxLines = 1) }
    }
}
