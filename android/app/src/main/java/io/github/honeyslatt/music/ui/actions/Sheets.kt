package io.github.honeyslatt.music.ui.actions

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
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
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateListOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.SolidColor
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.media3.common.util.UnstableApi
import io.github.honeyslatt.music.core.model.Playlist
import io.github.honeyslatt.music.ui.Icons
import io.github.honeyslatt.music.ui.components.Cover
import io.github.honeyslatt.music.ui.components.fold
import io.github.honeyslatt.music.ui.components.label
import io.github.honeyslatt.music.ui.components.Glyph
import io.github.honeyslatt.music.ui.components.Group
import io.github.honeyslatt.music.ui.components.GroupRow
import io.github.honeyslatt.music.ui.components.Line
import io.github.honeyslatt.music.ui.components.Sheet
import io.github.honeyslatt.music.ui.components.SheetButton
import io.github.honeyslatt.music.ui.components.sheetCell
import io.github.honeyslatt.music.ui.theme.geist
import io.github.honeyslatt.music.ui.theme.roles

/** The sheet an action asked for, over everything. */
@androidx.annotation.OptIn(UnstableApi::class)
@Composable
fun ActionSheets(actions: Actions) {
    val spec = actions.sheet
    // The last one stays drawn while it slides away.
    var shown by remember { mutableStateOf<SheetSpec?>(null) }
    if (spec != null) shown = spec
    val close = { actions.sheet = null }
    when (val s = shown) {
        is SheetSpec.AddToPlaylists -> AddToPlaylists(actions, s, open = spec === s, close)
        is SheetSpec.PlaylistForm -> PlaylistForm(actions, s, open = spec === s, close)
        is SheetSpec.Confirm -> Confirm(s, open = spec === s, close)
        null -> {}
    }
}

/** Add to Playlist(s): pick several, then Add. */
@androidx.annotation.OptIn(UnstableApi::class)
@Composable
private fun AddToPlaylists(actions: Actions, spec: SheetSpec.AddToPlaylists, open: Boolean, close: () -> Unit) {
    val chosen = remember(spec) { mutableStateListOf<Playlist>() }
    var query by remember(spec) { mutableStateOf("") }
    val all = remember(spec) { actions.editablePlaylists(spec.tracks) }
    val recent = remember(spec) { actions.recentPlaylists(spec.tracks) }
    LaunchedEffect(spec) { actions.loadContents(all) }
    val ids = spec.tracks.map { it.id }.toSet()
    fun already(p: Playlist) = actions.contents[p.id]?.containsAll(ids) == true
    Sheet(
        open,
        if (chosen.size > 1) "Add to Playlists" else "Add to Playlist",
        close,
        left = { SheetButton("Cancel", onClick = close) },
        right = { SheetButton("Add", primary = true, enabled = chosen.isNotEmpty()) { actions.addToChosen(spec.tracks, chosen.toList()) } },
    ) {
        Row(Modifier.padding(start = 2.dp, bottom = 10.dp), verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(12.dp)) {
            Cover(spec.artwork, 44.dp, RoundedCornerShape(7.dp), Modifier.size(44.dp))
            Column {
                Line(spec.title, geist(15.sp, FontWeight.SemiBold).copy(color = roles.text))
                if (spec.subtitle.isNotEmpty()) Line(spec.subtitle, geist(13.sp).copy(color = roles.sub))
            }
        }
        SearchField(query, "Search Playlists") { query = it }
        Spacer(Modifier.height(14.dp))
        Group {
            GroupRow(first = true, inset = 72.dp, onClick = { actions.sheet = SheetSpec.PlaylistForm(null, spec.tracks) }) {
                Box(Modifier.size(44.dp).background(roles.surface2, RoundedCornerShape(7.dp)), contentAlignment = Alignment.Center) {
                    Glyph(Icons.Plus, roles.tint, 22.dp)
                }
                BasicText("New Playlist…", style = geist(16.sp).copy(color = roles.tint))
            }
        }
        val folded = fold(query.trim())
        val matching = all.filter { folded.isEmpty() || fold(it.name).contains(folded) }
        @Composable
        fun rows(list: List<Playlist>) = list.forEachIndexed { i, p ->
            val has = already(p)
            val on = p in chosen
            GroupRow(first = i == 0, inset = 72.dp, onClick = if (has) null else ({ if (on) chosen.remove(p) else chosen.add(p) })) {
                io.github.honeyslatt.music.ui.components.PlaylistCover(p, 44.dp, RoundedCornerShape(7.dp), Modifier.size(44.dp))
                Column(Modifier.weight(1f)) {
                    Line(p.name, geist(16.sp).copy(color = roles.text))
                    if (has) Line("Already in playlist", geist(13.sp).copy(color = roles.sub))
                }
                Tick(on = on, done = has)
            }
        }
        if (query.isBlank() && recent.isNotEmpty()) {
            Label("Recent")
            Group { rows(recent) }
        }
        Label("Playlists")
        Group(footer = if (actions.playlistSongsToLibrary) "Songs you add to a playlist join your library if they aren’t there yet." else null) {
            if (matching.isEmpty()) {
                GroupRow(first = true) {
                    BasicText(if (all.isEmpty()) "No playlist of yours takes these songs." else "No playlist of yours matches.", style = geist(16.sp).copy(color = roles.sub))
                }
            } else {
                rows(matching)
            }
        }
    }
}

/** A round tick: empty, chosen (the accent), or grey for a playlist that has the songs. */
@Composable
private fun Tick(on: Boolean, done: Boolean) {
    val fill = when {
        done -> roles.surface3
        on -> roles.accent
        else -> null
    }
    Box(
        Modifier
            .size(24.dp)
            .then(if (fill != null) Modifier.background(fill, CircleShape) else Modifier.border(2.dp, roles.line, CircleShape)),
        contentAlignment = Alignment.Center,
    ) {
        if (fill != null) Glyph(Icons.Check, if (done) roles.sub else roles.onAccent, 15.dp)
    }
}

@Composable
private fun Label(text: String) {
    BasicText(text, Modifier.padding(start = 16.dp, bottom = 7.dp), style = geist(13.sp, FontWeight.Medium).copy(color = roles.sub))
}

@Composable
private fun SearchField(value: String, placeholder: String, onChange: (String) -> Unit) {
    Row(
        Modifier.fillMaxWidth().height(40.dp).background(sheetCell, RoundedCornerShape(12.dp)).padding(horizontal = 12.dp),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(8.dp),
    ) {
        Glyph(Icons.Search, roles.muted, 17.dp)
        Field(value, placeholder, Modifier.weight(1f), onChange = onChange)
    }
}

/** A text field in the design's type, with its placeholder. */
@Composable
private fun Field(value: String, placeholder: String, modifier: Modifier = Modifier, single: Boolean = true, onChange: (String) -> Unit) {
    val style = geist(16.sp).copy(color = roles.text)
    BasicTextField(
        value,
        onChange,
        modifier,
        textStyle = style,
        singleLine = single,
        minLines = if (single) 1 else 3,
        cursorBrush = SolidColor(roles.accent),
        decorationBox = { inner ->
            Box {
                if (value.isEmpty()) BasicText(placeholder, style = style.copy(color = roles.muted))
                inner()
            }
        },
    )
}

/** New Playlist, or Edit Playlist with Delete. Apple takes no cover from an app, so there is no cover to pick. */
@androidx.annotation.OptIn(UnstableApi::class)
@Composable
private fun PlaylistForm(actions: Actions, spec: SheetSpec.PlaylistForm, open: Boolean, close: () -> Unit) {
    val editing = spec.editing
    var name by remember(spec) { mutableStateOf(editing?.name.orEmpty()) }
    var description by remember(spec) { mutableStateOf(editing?.description.orEmpty()) }
    // Where a new one goes, asked when there is more than one choice.
    val services = remember(spec) { if (editing == null) actions.newPlaylistServices(spec.tracks) else emptyList() }
    var service by remember(spec) { mutableStateOf(io.github.honeyslatt.music.core.model.ProviderKind.MUSIC) }
    val ready = name.isNotBlank()
    Sheet(
        open,
        if (editing != null) "Edit Playlist" else "New Playlist",
        close,
        left = { SheetButton("Cancel", onClick = close) },
        right = {
            SheetButton(if (editing != null) "Save" else "Create", primary = true, enabled = ready) {
                if (editing != null) {
                    actions.editPlaylist(editing, name.trim(), description.trim())
                } else {
                    actions.createPlaylist(name.trim(), description.trim(), spec.tracks, service)
                }
            }
        },
    ) {
        Group {
            GroupRow(first = true) { Field(name, "Playlist Name", Modifier.weight(1f)) { name = it.take(100) } }
            GroupRow(first = false) { Field(description, "Description (Optional)", Modifier.weight(1f).padding(vertical = 4.dp), single = false) { description = it } }
        }
        if (services.size > 1) {
            Spacer(Modifier.height(14.dp))
            Label("Create In")
            Group(footer = "Music’s own playlists take songs from every service, and are kept on all your devices.") {
                services.forEachIndexed { i, kind ->
                    GroupRow(first = i == 0, inset = 58.dp, onClick = { service = kind }) {
                        io.github.honeyslatt.music.ui.components.ServiceLogo(kind, 30.dp)
                        BasicText(kind.label(), Modifier.weight(1f), style = geist(16.sp).copy(color = roles.text))
                        Tick(on = service == kind, done = false)
                    }
                }
            }
        }
        if (editing != null) {
            Group(footer = "It goes from your library on every device.") {
                GroupRow(first = true, onClick = { actions.confirmDelete(editing) }) {
                    BasicText("Delete Playlist", style = geist(16.sp).copy(color = roles.red))
                }
            }
        }
    }
}

/** Are you sure: what goes, the red button, and Cancel. */
@Composable
private fun Confirm(spec: SheetSpec.Confirm, open: Boolean, close: () -> Unit) {
    Sheet(open, null, close) {
        Column(Modifier.fillMaxWidth().padding(horizontal = 12.dp, vertical = 8.dp), horizontalAlignment = Alignment.CenterHorizontally) {
            BasicText(spec.title, style = geist(17.sp, FontWeight.SemiBold).copy(color = roles.text, textAlign = TextAlign.Center))
            Spacer(Modifier.height(6.dp))
            BasicText(spec.text, style = geist(14.5.sp).copy(color = roles.sub, textAlign = TextAlign.Center, lineHeight = 20.sp))
        }
        Spacer(Modifier.height(22.dp))
        Column(Modifier.fillMaxWidth().background(sheetCell, RoundedCornerShape(16.dp))) {
            Centred(spec.label, roles.red) {
                close()
                spec.action()
            }
        }
        Spacer(Modifier.height(10.dp))
        Column(Modifier.fillMaxWidth().background(sheetCell, RoundedCornerShape(16.dp))) {
            Centred("Cancel", roles.text, bold = true, onClick = close)
        }
    }
}

@Composable
private fun Centred(label: String, colour: androidx.compose.ui.graphics.Color, bold: Boolean = false, onClick: () -> Unit) {
    GroupRow(first = true, onClick = onClick) {
        BasicText(
            label,
            Modifier.weight(1f),
            style = geist(16.sp, if (bold) FontWeight.SemiBold else FontWeight.Normal).copy(color = colour, textAlign = TextAlign.Center),
        )
    }
}

