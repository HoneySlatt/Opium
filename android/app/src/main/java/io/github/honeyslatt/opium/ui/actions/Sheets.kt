package io.github.honeyslatt.opium.ui.actions

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
import kotlinx.coroutines.launch
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateListOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import io.github.honeyslatt.opium.ui.components.press
import androidx.compose.ui.graphics.asImageBitmap
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.SolidColor
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.media3.common.util.UnstableApi
import io.github.honeyslatt.opium.core.model.Playlist
import io.github.honeyslatt.opium.ui.Icons
import io.github.honeyslatt.opium.ui.components.Cover
import io.github.honeyslatt.opium.ui.components.fold
import io.github.honeyslatt.opium.ui.components.label
import io.github.honeyslatt.opium.ui.components.Glyph
import io.github.honeyslatt.opium.ui.components.Group
import io.github.honeyslatt.opium.ui.components.GroupRow
import io.github.honeyslatt.opium.ui.components.Line
import io.github.honeyslatt.opium.ui.components.Sheet
import io.github.honeyslatt.opium.ui.components.SheetButton
import io.github.honeyslatt.opium.ui.components.sheetCell
import io.github.honeyslatt.opium.ui.theme.geist
import io.github.honeyslatt.opium.ui.theme.roles

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
        is SheetSpec.NavidromePassword -> NavidromePassword(actions, s, open = spec === s, close)
        is SheetSpec.Confirm -> Confirm(s, open = spec === s, close)
        is SheetSpec.Info -> Info(actions, s, open = spec === s, close)
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
                io.github.honeyslatt.opium.ui.components.PlaylistCover(p, 44.dp, RoundedCornerShape(7.dp), Modifier.size(44.dp))
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

/** A file's tags as the Info sheet edits them, in the order shown. */
private val TAGS = listOf("Title", "Artist", "Album", "Album Artist", "Genre", "Track", "Disc", "Year")

private fun io.github.honeyslatt.opium.core.FileTags.values() = listOf(
    title, artist, album, albumArtist, genre,
    track?.toString().orEmpty(), disc?.toString().orEmpty(), year?.toString().orEmpty(),
)

/**
 * Get Info, as on the computer: a file's title, artist, album and the rest,
 * and its cover, written back into the file once Android allows it. An
 * album's songs at once: what they differ in shows as Mixed, left as it is
 * unless typed.
 */
@androidx.annotation.OptIn(UnstableApi::class)
@Composable
private fun Info(actions: Actions, spec: SheetSpec.Info, open: Boolean, close: () -> Unit) {
    val context = androidx.compose.ui.platform.LocalContext.current
    val local = (context.applicationContext as io.github.honeyslatt.opium.OpiumApp).localFiles
    val scope = androidx.compose.runtime.rememberCoroutineScope()
    val several = spec.tracks.size > 1
    // As the files say; for several, empty where they differ.
    var before by remember(spec) { mutableStateOf<List<String>?>(null) }
    var mixed by remember(spec) { mutableStateOf(List(TAGS.size) { false }) }
    val typed = remember(spec) { mutableStateListOf<String>() }
    var cover by remember(spec) { mutableStateOf<CoverChoice>(CoverChoice.Unchanged) }
    var saving by remember(spec) { mutableStateOf(false) }
    var error by remember(spec) { mutableStateOf<String?>(null) }
    LaunchedEffect(spec) {
        val all = spec.tracks.map { local.tags(it)?.values() ?: List(TAGS.size) { "" } }
        val first = all.first()
        val differs = TAGS.indices.map { i -> all.any { it[i] != first[i] } }
        val shown = first.mapIndexed { i, v -> if (differs[i]) "" else v }
        mixed = differs
        before = shown
        typed.clear()
        typed.addAll(shown)
    }
    val edit = {
        val was = before.orEmpty()
        val text = { i: Int -> typed[i].trim().takeIf { it != was[i].trim() } }
        val number = { i: Int -> typed[i].trim() != was[i].trim() }
        io.github.honeyslatt.opium.core.FileTagEdit(
            title = text(0), artist = text(1), album = text(2), albumArtist = text(3), genre = text(4),
            setTrack = number(5), track = typed[5].trim().toUIntOrNull(),
            setDisc = number(6), disc = typed[6].trim().toUIntOrNull(),
            setYear = number(7), year = typed[7].trim().toUShortOrNull(),
            cover = when (val c = cover) {
                is CoverChoice.Chosen -> io.github.honeyslatt.opium.core.FileCover.Set(c.jpeg)
                CoverChoice.Removed -> io.github.honeyslatt.opium.core.FileCover.Removed
                CoverChoice.Unchanged -> io.github.honeyslatt.opium.core.FileCover.Kept
            },
        )
    }
    // Android's window asks the user's leave to change the files; once given, written.
    val allow = androidx.activity.compose.rememberLauncherForActivityResult(androidx.activity.result.contract.ActivityResultContracts.StartIntentSenderForResult()) { result ->
        if (result.resultCode != android.app.Activity.RESULT_OK) {
            saving = false
            return@rememberLauncherForActivityResult
        }
        scope.launch {
            error = local.write(spec.tracks, edit())
            saving = false
            if (error == null) close()
        }
    }
    val pick = androidx.activity.compose.rememberLauncherForActivityResult(androidx.activity.result.contract.ActivityResultContracts.PickVisualMedia()) { uri ->
        uri ?: return@rememberLauncherForActivityResult
        scope.launch {
            val jpeg = io.github.honeyslatt.opium.data.CoverImage.square(context, uri) ?: return@launch
            val shown = android.graphics.BitmapFactory.decodeByteArray(jpeg, 0, jpeg.size)?.asImageBitmap() ?: return@launch
            cover = CoverChoice.Chosen(jpeg, shown)
        }
    }
    val numbersOk = (5..7).all { i -> typed.getOrNull(i)?.trim()?.let { it.isEmpty() || it.toUIntOrNull() != null } ?: true }
    Sheet(
        open,
        if (several) "Album Info" else "Info",
        close,
        left = { SheetButton("Cancel", onClick = close) },
        right = {
            SheetButton(if (saving) "Saving…" else "Save", primary = true, enabled = before != null && !saving && numbersOk) {
                val changed = edit()
                val nothing = listOf(changed.title, changed.artist, changed.album, changed.albumArtist, changed.genre).all { it == null } &&
                    !changed.setTrack && !changed.setDisc && !changed.setYear && changed.cover is io.github.honeyslatt.opium.core.FileCover.Kept
                if (nothing) {
                    close()
                } else {
                    saving = true
                    error = null
                    // A file Android's index hasn't (opened from another app,
                    // or another device's): nobody to ask leave of.
                    val request = local.writeRequest(spec.tracks)
                    if (request == null) {
                        saving = false
                        error = if (several) {
                            "These files aren't in Android's music index: their tags can't be changed here."
                        } else {
                            "This file isn't in Android's music index: its tags can't be changed here."
                        }
                    } else {
                        allow.launch(androidx.activity.result.IntentSenderRequest.Builder(request).build())
                    }
                }
            }
        },
    ) {
        val choose = { pick.launch(androidx.activity.result.PickVisualMediaRequest(androidx.activity.result.contract.ActivityResultContracts.PickVisualMedia.ImageOnly)) }
        val artwork = spec.tracks.firstNotNullOfOrNull { it.artwork }
        Column(Modifier.fillMaxWidth().padding(bottom = 16.dp), horizontalAlignment = Alignment.CenterHorizontally) {
            val shape = RoundedCornerShape(14.dp)
            Box(
                Modifier.size(168.dp).clip(shape).background(roles.surface, shape).press(shape, onClick = choose),
                contentAlignment = Alignment.Center,
            ) {
                when (val c = cover) {
                    is CoverChoice.Chosen -> androidx.compose.foundation.Image(c.shown, null, Modifier.size(168.dp), contentScale = androidx.compose.ui.layout.ContentScale.Crop)
                    CoverChoice.Removed -> Glyph(Icons.Image, roles.muted, 44.dp)
                    CoverChoice.Unchanged -> if (artwork != null) Cover(artwork, 168.dp, shape, Modifier.size(168.dp)) else Glyph(Icons.Image, roles.muted, 44.dp)
                }
            }
            Row(Modifier.padding(top = 10.dp), horizontalArrangement = Arrangement.spacedBy(22.dp)) {
                Box(Modifier.press(RoundedCornerShape(8.dp), onClick = choose).padding(6.dp)) {
                    BasicText(if (cover is CoverChoice.Chosen || artwork != null) "Change Cover" else "Choose Cover", style = geist(15.sp, FontWeight.Medium).copy(color = roles.accent))
                }
                if (cover is CoverChoice.Chosen || (cover == CoverChoice.Unchanged && artwork != null)) {
                    Box(Modifier.press(RoundedCornerShape(8.dp)) { cover = CoverChoice.Removed }.padding(6.dp)) {
                        BasicText("Remove", style = geist(15.sp, FontWeight.Medium).copy(color = roles.red))
                    }
                }
            }
            if (several) BasicText("${spec.tracks.size} songs: what is typed goes in each.", Modifier.padding(top = 6.dp), style = geist(13.sp).copy(color = roles.sub))
        }
        if (typed.size == TAGS.size) {
            Group(footer = error) {
                TAGS.forEachIndexed { i, label ->
                    // A song's own title and number only for one song.
                    if (several && (i == 0 || i == 5)) return@forEachIndexed
                    GroupRow(first = i == 0 || (several && i == 1)) {
                        BasicText(label, Modifier.size(width = 104.dp, height = 22.dp), style = geist(15.sp).copy(color = roles.sub))
                        Field(typed[i], if (mixed[i]) "Mixed" else "", Modifier.weight(1f)) { typed[i] = if (i >= 5) it.filter(Char::isDigit).take(4) else it }
                    }
                }
            }
        }
    }
}

/** What becomes of a playlist's cover as the form is saved. */
private sealed interface CoverChoice {
    data object Unchanged : CoverChoice
    data object Removed : CoverChoice
    class Chosen(val jpeg: ByteArray, val shown: androidx.compose.ui.graphics.ImageBitmap) : CoverChoice
}

/**
 * New Playlist, or Edit Playlist with Delete. Opium's own and Navidrome's
 * take a cover picked among the photos; Apple and SoundCloud's keep theirs.
 */
@androidx.annotation.OptIn(UnstableApi::class)
@Composable
private fun PlaylistForm(actions: Actions, spec: SheetSpec.PlaylistForm, open: Boolean, close: () -> Unit) {
    val editing = spec.editing
    var name by remember(spec) { mutableStateOf(editing?.name.orEmpty()) }
    var description by remember(spec) { mutableStateOf(editing?.description.orEmpty()) }
    // Where a new one goes, asked when there is more than one choice.
    val services = remember(spec) { if (editing == null) actions.newPlaylistServices(spec.tracks) else emptyList() }
    var service by remember(spec) { mutableStateOf(io.github.honeyslatt.opium.core.model.ProviderKind.OPIUM) }
    var cover by remember(spec) { mutableStateOf<CoverChoice>(CoverChoice.Unchanged) }
    val provider = editing?.id?.provider ?: service
    val context = androidx.compose.ui.platform.LocalContext.current
    val scope = androidx.compose.runtime.rememberCoroutineScope()
    val pick = androidx.activity.compose.rememberLauncherForActivityResult(androidx.activity.result.contract.ActivityResultContracts.PickVisualMedia()) { uri ->
        uri ?: return@rememberLauncherForActivityResult
        scope.launch {
            val jpeg = io.github.honeyslatt.opium.data.CoverImage.square(context, uri) ?: return@launch
            val shown = android.graphics.BitmapFactory.decodeByteArray(jpeg, 0, jpeg.size)?.asImageBitmap() ?: return@launch
            cover = CoverChoice.Chosen(jpeg, shown)
        }
    }
    val ready = name.isNotBlank()
    Sheet(
        open,
        if (editing != null) "Edit Playlist" else "New Playlist",
        close,
        left = { SheetButton("Cancel", onClick = close) },
        right = {
            SheetButton(if (editing != null) "Save" else "Create", primary = true, enabled = ready) {
                val chosen = cover
                if (editing != null) {
                    actions.editPlaylist(editing, name.trim(), description.trim())
                    when (chosen) {
                        is CoverChoice.Chosen -> actions.setPlaylistCover(editing.id, chosen.jpeg)
                        CoverChoice.Removed -> actions.setPlaylistCover(editing.id, null)
                        CoverChoice.Unchanged -> {}
                    }
                } else {
                    actions.createPlaylist(name.trim(), description.trim(), spec.tracks, service, (chosen as? CoverChoice.Chosen)?.jpeg)
                }
            }
        },
    ) {
        if (actions.coverable(provider)) {
            val choose = { pick.launch(androidx.activity.result.PickVisualMediaRequest(androidx.activity.result.contract.ActivityResultContracts.PickVisualMedia.ImageOnly)) }
            // Its own cover: one chosen here, Opium's kept as a file, or (Navidrome's) any.
            val custom = when (val c = cover) {
                is CoverChoice.Chosen -> true
                CoverChoice.Removed -> false
                CoverChoice.Unchanged -> editing != null &&
                    (provider == io.github.honeyslatt.opium.core.model.ProviderKind.NAVIDROME || editing.artwork?.urlTemplate?.startsWith("file://") == true)
            }
            Column(Modifier.fillMaxWidth().padding(bottom = 16.dp), horizontalAlignment = Alignment.CenterHorizontally) {
                val shape = RoundedCornerShape(14.dp)
                Box(
                    Modifier.size(168.dp).clip(shape).background(roles.surface, shape).press(shape, onClick = choose),
                    contentAlignment = Alignment.Center,
                ) {
                    when (val c = cover) {
                        is CoverChoice.Chosen -> androidx.compose.foundation.Image(c.shown, null, Modifier.size(168.dp), contentScale = androidx.compose.ui.layout.ContentScale.Crop)
                        else -> {
                            val shown = editing?.artwork.takeIf { c != CoverChoice.Removed }
                            if (shown != null) Cover(shown, 168.dp, shape, Modifier.size(168.dp)) else Glyph(Icons.Image, roles.muted, 44.dp)
                        }
                    }
                }
                Row(Modifier.padding(top = 10.dp), horizontalArrangement = Arrangement.spacedBy(22.dp)) {
                    Box(Modifier.press(RoundedCornerShape(8.dp), onClick = choose).padding(6.dp)) {
                        BasicText(if (custom) "Change Cover" else "Choose Cover", style = geist(15.sp, FontWeight.Medium).copy(color = roles.accent))
                    }
                    if (custom) {
                        Box(Modifier.press(RoundedCornerShape(8.dp)) { cover = CoverChoice.Removed }.padding(6.dp)) {
                            BasicText("Remove", style = geist(15.sp, FontWeight.Medium).copy(color = roles.red))
                        }
                    }
                }
            }
        }
        Group {
            GroupRow(first = true) { Field(name, "Playlist Name", Modifier.weight(1f)) { name = it.take(100) } }
            GroupRow(first = false) { Field(description, "Description (Optional)", Modifier.weight(1f).padding(vertical = 4.dp), single = false) { description = it } }
        }
        if (services.size > 1) {
            Spacer(Modifier.height(14.dp))
            Label("Create In")
            Group(footer = "Opium’s own playlists take songs from every service, and are kept on all your devices.") {
                services.forEachIndexed { i, kind ->
                    GroupRow(first = i == 0, inset = 58.dp, onClick = { service = kind }) {
                        io.github.honeyslatt.opium.ui.components.ServiceLogo(kind, 30.dp)
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

/**
 * Navidrome's own API wants the password: given once, it opens a session
 * the app keeps (never the password), renewed while the app is used.
 */
@Composable
private fun NavidromePassword(actions: Actions, spec: SheetSpec.NavidromePassword, open: Boolean, close: () -> Unit) {
    var password by remember(spec) { mutableStateOf("") }
    var refused by remember(spec) { mutableStateOf(false) }
    val scope = androidx.compose.runtime.rememberCoroutineScope()
    Sheet(
        open,
        "Navidrome",
        close,
        left = { SheetButton("Cancel", onClick = close) },
        right = {
            SheetButton("Continue", primary = true, enabled = password.isNotEmpty()) {
                val typed = password
                password = ""
                scope.launch { refused = !actions.navidromePassword(typed, spec.then) }
            }
        },
    ) {
        Group(footer = if (refused) "That password isn’t the one Navidrome knows." else "Changing a cover needs a session on your server. The password opens it and isn’t kept.") {
            GroupRow(first = true) {
                BasicTextField(
                    password,
                    { password = it },
                    Modifier.weight(1f),
                    textStyle = geist(16.sp).copy(color = roles.text),
                    singleLine = true,
                    cursorBrush = SolidColor(roles.accent),
                    visualTransformation = androidx.compose.ui.text.input.PasswordVisualTransformation(),
                    keyboardOptions = androidx.compose.foundation.text.KeyboardOptions(keyboardType = androidx.compose.ui.text.input.KeyboardType.Password),
                    decorationBox = { inner ->
                        Box {
                            if (password.isEmpty()) BasicText("Password", style = geist(16.sp).copy(color = roles.muted))
                            inner()
                        }
                    },
                )
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

