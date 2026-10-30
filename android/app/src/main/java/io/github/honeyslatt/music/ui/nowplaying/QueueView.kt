package io.github.honeyslatt.music.ui.nowplaying

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.itemsIndexed
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.BasicText
import androidx.compose.runtime.Composable
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.media3.common.util.UnstableApi
import io.github.honeyslatt.music.core.QueueRepeat
import io.github.honeyslatt.music.core.model.Track
import io.github.honeyslatt.music.ui.actions.LocalActions
import io.github.honeyslatt.music.ui.components.anchor
import io.github.honeyslatt.music.ui.components.rememberAnchor
import io.github.honeyslatt.music.playback.Playback
import io.github.honeyslatt.music.ui.Icons
import io.github.honeyslatt.music.ui.components.Cover
import io.github.honeyslatt.music.ui.components.Glyph
import io.github.honeyslatt.music.ui.components.Line
import io.github.honeyslatt.music.ui.components.press
import io.github.honeyslatt.music.ui.theme.geist
import io.github.honeyslatt.music.ui.theme.roles

/** The queue: shuffle and repeat, then what plays next, or what played. */
@androidx.annotation.OptIn(UnstableApi::class)
@Composable
fun QueueView(playback: Playback) {
    val np = LocalNp.current
    val upcoming by playback.upcoming.collectAsStateWithLifecycle()
    val history by playback.history.collectAsStateWithLifecycle()
    val shuffle by playback.shuffleOn.collectAsStateWithLifecycle()
    val repeat by playback.repeat.collectAsStateWithLifecycle()
    var showHistory by remember { mutableStateOf(false) }
    Column(Modifier.fillMaxSize()) {
        Row(Modifier.fillMaxWidth().padding(bottom = 14.dp), horizontalArrangement = Arrangement.spacedBy(10.dp)) {
            Toggle(Icons.Shuffle, "Shuffle", shuffle, Modifier.weight(1f)) { playback.setShuffle(!shuffle) }
            Toggle(
                if (repeat == QueueRepeat.ONE) Icons.Repeat1 else Icons.Repeat,
                "Repeat",
                repeat != QueueRepeat.OFF,
                Modifier.weight(1f),
            ) { playback.cycleRepeat() }
        }
        Row(
            Modifier.fillMaxWidth().height(38.dp).background(np.button, RoundedCornerShape(11.dp)).padding(3.dp),
        ) {
            listOf(false to "Playing Next", true to "History").forEach { (hist, label) ->
                val on = showHistory == hist
                Box(
                    Modifier
                        .weight(1f)
                        .height(32.dp)
                        .press(RoundedCornerShape(8.dp)) { showHistory = hist }
                        .background(if (on) np.button else androidx.compose.ui.graphics.Color.Transparent, RoundedCornerShape(8.dp)),
                    contentAlignment = Alignment.Center,
                ) {
                    BasicText(label, style = geist(14.sp, FontWeight.Medium).copy(color = if (on) np.fg else np.sub))
                }
            }
        }
        val list = if (showHistory) history else upcoming
        Row(Modifier.fillMaxWidth().padding(top = 12.dp, bottom = 4.dp), verticalAlignment = Alignment.CenterVertically) {
            BasicText(if (showHistory) "Most recent first" else "", Modifier.weight(1f), style = geist(12.sp).copy(color = np.sub))
            if (list.isNotEmpty()) {
                Box(Modifier.press { if (showHistory) playback.clearHistory() else playback.clearUpcoming() }) {
                    BasicText("Clear", style = geist(14.sp, FontWeight.SemiBold).copy(color = np.fg))
                }
            }
        }
        LazyColumn(Modifier.fillMaxWidth().weight(1f)) {
            itemsIndexed(list, key = { i, t -> "$i" + t.id.id }) { i, track ->
                QueueRow(track, removable = !showHistory, onPlay = {
                    if (showHistory) playback.play(listOf(track), 0) else playback.jump(i)
                }) { playback.removeUpcoming(i) }
            }
            item {
                BasicText(
                    when {
                        list.isNotEmpty() && !showHistory -> "The next song joins without a gap."
                        showHistory -> "Songs you finish or skip show up here."
                        else -> "The queue ends here."
                    },
                    Modifier.padding(top = 14.dp, bottom = 8.dp),
                    style = geist(13.sp).copy(color = np.sub),
                )
            }
        }
    }
}

@Composable
private fun Toggle(icon: androidx.compose.ui.graphics.vector.ImageVector, label: String, on: Boolean, modifier: Modifier, onClick: () -> Unit) {
    val np = LocalNp.current
    val shape = RoundedCornerShape(11.dp)
    val fg = if (on) roles.base else np.fg
    Row(
        modifier.height(38.dp).press(shape, shrink = 0.96f, onClick = onClick).background(if (on) np.fg else np.button, shape),
        horizontalArrangement = Arrangement.Center,
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Glyph(icon, fg, 17.dp)
        BasicText(label, Modifier.padding(start = 8.dp), style = geist(14.sp, FontWeight.SemiBold).copy(color = fg))
    }
}

@Composable
private fun QueueRow(track: Track, removable: Boolean, onPlay: () -> Unit, onRemove: () -> Unit) {
    val np = LocalNp.current
    val actions = LocalActions.current
    val anchor = rememberAnchor()
    Row(
        Modifier.fillMaxWidth().height(60.dp).anchor(anchor).press(onLongClick = { actions?.trackMenu(track, anchor.rect) }, onClick = onPlay),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        Cover(track.artwork, 44.dp, RoundedCornerShape(7.dp), Modifier.size(44.dp))
        Column(Modifier.weight(1f)) {
            Line(track.title, geist(15.5.sp).copy(color = np.fg))
            Line(track.artists.joinToString(", "), geist(13.sp).copy(color = np.sub))
        }
        if (removable) {
            Box(Modifier.size(40.dp).press(CircleShape, shrink = 0.9f, onClick = onRemove), contentAlignment = Alignment.Center) {
                Glyph(Icons.Minuscircle, np.sub, 20.dp)
            }
        }
    }
}
