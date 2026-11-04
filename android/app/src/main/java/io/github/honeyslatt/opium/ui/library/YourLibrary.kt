package io.github.honeyslatt.opium.ui.library

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.statusBars
import androidx.compose.foundation.layout.windowInsetsTopHeight
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.BasicText
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.produceState
import androidx.compose.runtime.remember
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Rect
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import io.github.honeyslatt.opium.core.albumsLeftOut
import io.github.honeyslatt.opium.core.artistsLeftOut
import io.github.honeyslatt.opium.core.model.Artwork
import io.github.honeyslatt.opium.data.downloadable
import io.github.honeyslatt.opium.ui.Icons
import io.github.honeyslatt.opium.ui.MoreIcons
import io.github.honeyslatt.opium.ui.actions.LocalActions
import io.github.honeyslatt.opium.ui.actions.SheetSpec
import io.github.honeyslatt.opium.ui.chrome.ChromeHeight
import io.github.honeyslatt.opium.ui.components.Cover
import io.github.honeyslatt.opium.ui.components.Glyph
import io.github.honeyslatt.opium.ui.components.SearchReveal
import io.github.honeyslatt.opium.ui.components.anchor
import io.github.honeyslatt.opium.ui.components.matches
import io.github.honeyslatt.opium.ui.components.press
import io.github.honeyslatt.opium.ui.components.rememberAnchor
import io.github.honeyslatt.opium.ui.nav.Route
import io.github.honeyslatt.opium.ui.theme.geist
import io.github.honeyslatt.opium.ui.theme.roles
import io.github.honeyslatt.opium.ui.theme.expressive
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext

/** Spotify's Liked Songs violet, for the library's songs. */
val Loved = listOf(Color(0xFF450AF5), Color(0xFFC4EFD9))

/** What Your Library's chips narrow it to. */
private enum class Kind(val label: String) { Playlists("Playlists"), Albums("Albums"), Artists("Artists"), Downloaded("Downloaded") }

/**
 * Spotify's Your Library: the account and the title, search and New
 * Playlist; chips for playlists, albums or artists; the library's songs
 * pinned over the playlists as Liked Songs; then the list, covers first.
 */
@Composable
fun YourLibrary(c: LibraryContext) {
    val actions = LocalActions.current
    var kind by rememberSaveable { mutableStateOf(Kind.Playlists) }
    var searching by rememberSaveable { mutableStateOf(false) }
    var query by rememberSaveable { mutableStateOf("") }
    val profile by c.core.appleProfile.collectAsStateWithLifecycle()
    val me = profile?.name ?: "You"
    // What another service has too is left out once.
    val albums by produceState(c.lists.albums.items.distinctBy { it.id }, c.lists.albums.items) {
        val all = c.lists.albums.items
        value = withContext(Dispatchers.Default) {
            val out = albumsLeftOut(all).toSet()
            all.filterIndexed { i, _ -> i.toUInt() !in out }.distinctBy { it.id }
        }
    }
    val artists by produceState(c.lists.artists.items.distinctBy { it.id }, c.lists.artists.items) {
        val all = c.lists.artists.items
        value = withContext(Dispatchers.Default) {
            val out = artistsLeftOut(all).toSet()
            all.filterIndexed { i, _ -> i.toUInt() !in out }.distinctBy { it.id }
        }
    }
    val downloads = (androidx.compose.ui.platform.LocalContext.current.applicationContext as io.github.honeyslatt.opium.OpiumApp).downloads
    val keptSongs by downloads.kept.collectAsStateWithLifecycle()
    val shownDownloads = io.github.honeyslatt.opium.ui.components.LocalDownloads.current
    // The albums and playlists downloaded, each with its songs, for Downloaded.
    val keptAlbums = remember(keptSongs, albums) {
        val byAlbum = keptSongs.groupBy { it.track.albumId }
        albums.mapNotNull { a -> byAlbum[a.id]?.let { a to it.size } }
    }
    val keptPlaylists by produceState(emptyList<Pair<io.github.honeyslatt.opium.core.model.Playlist, List<io.github.honeyslatt.opium.core.model.Track>>>(), keptSongs, c.lists.playlists.items) {
        value = c.lists.playlists.items.distinctBy { it.id }.mapNotNull { p ->
            downloads.keptPlaylist(p.id).takeIf { songs -> songs.any { it.id in shownDownloads.states } }?.let { p to it }
        }
    }
    Box(Modifier.fillMaxSize().background(roles.base)) {
        val refresh = c.refresh {
            when (kind) {
                Kind.Playlists, Kind.Downloaded -> it.playlists
                Kind.Albums -> it.albums
                Kind.Artists -> it.artists
            }
        }
        val expressive = io.github.honeyslatt.opium.ui.theme.look.expressive
        val listState = androidx.compose.foundation.lazy.rememberLazyListState()
        io.github.honeyslatt.opium.ui.components.PullRefresh(refresh, 0.dp) {
            LazyColumn(Modifier.fillMaxSize(), state = listState) {
                item { Spacer(Modifier.windowInsetsTopHeight(WindowInsets.statusBars)) }
                // M3E's: the search bar over it, its title at 36 sp with a tonal +.
                if (expressive) {
                    item { Spacer(Modifier.height(64.dp)) }
                    item {
                        Row(
                            Modifier.fillMaxWidth().padding(start = 20.dp, end = 16.dp, top = 8.dp, bottom = 12.dp),
                            verticalAlignment = Alignment.CenterVertically,
                        ) {
                            BasicText("Library", Modifier.weight(1f), style = geist(36.sp, FontWeight.Bold, -0.02f).copy(color = roles.text))
                            io.github.honeyslatt.opium.ui.m3e.Tonal(Icons.Plus, 48.dp) { actions?.sheet = SheetSpec.PlaylistForm(null) }
                        }
                    }
                } else item {
                    Row(
                        Modifier.fillMaxWidth().padding(start = 16.dp, end = 8.dp, top = 10.dp, bottom = 12.dp),
                        verticalAlignment = Alignment.CenterVertically,
                        horizontalArrangement = Arrangement.spacedBy(12.dp),
                    ) {
                        c.account()
                        BasicText("Your Library", Modifier.weight(1f), style = geist(24.sp, FontWeight.Bold).copy(color = roles.text))
                        Box(Modifier.size(44.dp).press(CircleShape) { searching = !searching }, contentAlignment = Alignment.Center) {
                            Glyph(Icons.Search, roles.text, 24.dp)
                        }
                        Box(
                            Modifier.size(44.dp).press(CircleShape) { actions?.sheet = SheetSpec.PlaylistForm(null) },
                            contentAlignment = Alignment.Center,
                        ) { Glyph(Icons.Plus, roles.text, 26.dp) }
                    }
                }
                item {
                    Row(Modifier.padding(horizontal = 16.dp), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                        Kind.entries.filter { it != Kind.Downloaded || keptSongs.isNotEmpty() || kind == it }.forEach { k ->
                            val on = k == kind
                            Box(
                                Modifier.height(34.dp)
                                    .press(RoundedCornerShape(17.dp)) { kind = k }
                                    .background(
                                        when {
                                            expressive && on -> roles.secondaryContainer
                                            on -> roles.accent
                                            else -> roles.surface2
                                        },
                                        RoundedCornerShape(if (expressive) 8.dp else 17.dp),
                                    )
                                    .padding(horizontal = 14.dp),
                                contentAlignment = Alignment.Center,
                            ) {
                                BasicText(
                                    k.label,
                                    style = geist(14.sp, if (expressive) FontWeight.Medium else FontWeight.Normal)
                                        .copy(color = if (on) (if (expressive) roles.onSecondaryContainer else roles.onAccent) else roles.text),
                                )
                            }
                        }
                    }
                }
                item { SearchReveal(searching, query, "Search ${kind.label}") { query = it } }
                item { Spacer(Modifier.height(8.dp)) }
                when (kind) {
                    Kind.Playlists -> {
                        if (matches(query, "Songs")) {
                            item {
                                Entry(
                                    null,
                                    "Songs",
                                    if (c.lists.songs.items.isEmpty() && !c.lists.songs.done) "Playlist" else "Playlist • ${songs(c.lists.songs.items.size)}",
                                    pinned = true,
                                    tile = {
                                        // M3E's: a tertiary tile, the note on it.
                                        if (expressive) {
                                            Box(Modifier.size(56.dp).background(roles.tertiaryContainer, RoundedCornerShape(12.dp)), contentAlignment = Alignment.Center) {
                                                Glyph(Icons.Note, roles.onTertiaryContainer, 26.dp)
                                            }
                                        } else {
                                            Box(Modifier.size(64.dp).background(Brush.linearGradient(Loved), RoundedCornerShape(4.dp)), contentAlignment = Alignment.Center) {
                                                Glyph(MoreIcons.HeartFill, Color.White, 26.dp)
                                            }
                                        }
                                    },
                                ) { c.nav.push(Route.Songs) }
                            }
                        }
                        val playlists = c.lists.playlists.items.distinctBy { it.id }
                            .filter { matches(query, if (it.favourites) "Favourite Songs" else it.name, it.curator) }
                        items(playlists, key = { it.id.provider.name + it.id.id }) { p ->
                            val by = if (p.editable) me else p.curator
                            Entry(
                                p.artwork,
                                if (p.favourites) "Favourite Songs" else p.name,
                                listOfNotNull("Playlist", by).joinToString(" • "),
                                tile = if (p.artwork == null) {
                                    { io.github.honeyslatt.opium.ui.components.PlaylistCover(p, 64.dp, RoundedCornerShape(4.dp), Modifier.size(64.dp)) }
                                } else {
                                    null
                                },
                                onMenu = { actions?.playlistMenu(p, it) },
                            ) { c.nav.push(Route.Playlist(p.id)) }
                        }
                    }
                    Kind.Albums -> items(albums.filter { matches(query, it.title, it.artists.joinToString(" ")) }, key = { it.id.provider.name + it.id.id }) { a ->
                        Entry(a.artwork, a.title, "Album • ${a.artists.joinToString(", ")}") { c.nav.push(Route.Album(a.id)) }
                    }
                    Kind.Artists -> items(artists.filter { matches(query, it.name) }, key = { it.id.provider.name + it.id.id }) { a ->
                        Entry(a.artwork, a.name, "Artist", round = true) { c.nav.push(Route.LibraryArtist(a)) }
                    }
                    Kind.Downloaded -> {
                        item {
                            Entry(
                                null,
                                "Downloaded Songs",
                                "${songs(keptSongs.size)} • on this phone",
                                pinned = true,
                                tile = {
                                    Box(Modifier.size(64.dp).background(roles.surface2, RoundedCornerShape(4.dp)), contentAlignment = Alignment.Center) {
                                        Glyph(MoreIcons.ArrowDown, roles.accent, 26.dp)
                                    }
                                },
                            ) { c.nav.push(Route.DownloadedSongs) }
                        }
                        items(keptPlaylists.filter { matches(query, it.first.name) }, key = { "p" + it.first.id.provider.name + it.first.id.id }) { (p, tracks) ->
                            Entry(p.artwork, p.name, "Playlist • ${downloadedLine(shownDownloads, tracks)}", onMenu = { actions?.playlistMenu(p, it) }) {
                                c.nav.push(Route.Playlist(p.id))
                            }
                        }
                        items(keptAlbums.filter { matches(query, it.first.title) }, key = { "a" + it.first.id.provider.name + it.first.id.id }) { (a, n) ->
                            Entry(a.artwork, a.title, "Album • ${a.artists.joinToString(", ")} • ${songs(n)}") { c.nav.push(Route.Album(a.id)) }
                        }
                    }
                }
                val listing = when (kind) {
                    Kind.Playlists, Kind.Downloaded -> c.lists.playlists
                    Kind.Albums -> c.lists.albums
                    Kind.Artists -> c.lists.artists
                }
                if (kind != Kind.Downloaded && listing.items.isEmpty() && query.isBlank()) {
                    item { Empty(listing, "Nothing here yet", "${kind.label} you add show here.") }
                }
                item { Spacer(Modifier.height(ChromeHeight + 120.dp)) }
            }
        }
        if (expressive) {
            val searched by androidx.compose.runtime.remember { androidx.compose.runtime.derivedStateOf { listState.firstVisibleItemIndex > 0 || listState.firstVisibleItemScrollOffset > 8 } }
            io.github.honeyslatt.opium.ui.chrome.M3ESearchBar(searched, { c.nav.select(io.github.honeyslatt.opium.ui.nav.Tab.Search) }, c.account)
        }
    }
}

/** How a playlist downloaded stands: coming, or how many of its songs are kept. */
private fun downloadedLine(shown: io.github.honeyslatt.opium.ui.components.DownloadsShown, tracks: List<io.github.honeyslatt.opium.core.model.Track>): String {
    val own = tracks.filter(::downloadable)
    val kept = own.count(shown::kept)
    val coming = own.any { shown.state(it).let { s -> s is io.github.honeyslatt.opium.core.DownloadState.Queued || s is io.github.honeyslatt.opium.core.DownloadState.Coming } }
    return when {
        coming -> "Downloading • $kept of ${own.size}"
        kept == own.size -> "Downloaded • ${songs(kept)}"
        else -> "$kept of ${songs(own.size)} downloaded"
    }
}

/** A row of Your Library: the cover, the name, what it is and by whom. */
@Composable
private fun Entry(
    artwork: Artwork?,
    title: String,
    subtitle: String,
    round: Boolean = false,
    pinned: Boolean = false,
    tile: (@Composable () -> Unit)? = null,
    onMenu: ((Rect) -> Unit)? = null,
    onClick: () -> Unit,
) {
    val anchor = rememberAnchor()
    Row(
        Modifier
            .fillMaxWidth()
            .anchor(anchor)
            .press(onLongClick = onMenu?.let { { it(anchor.rect) } }, onClick = onClick)
            .padding(horizontal = 16.dp, vertical = 8.dp),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(14.dp),
    ) {
        val expressive = io.github.honeyslatt.opium.ui.theme.look.expressive
        when {
            tile != null -> tile()
            // M3E's: 56 dp at radius 12, artists in the Sunny shape.
            expressive && round -> Cover(artwork, 56.dp, io.github.honeyslatt.opium.ui.m3e.RadialShape(io.github.honeyslatt.opium.ui.m3e.M3Shape.Sunny.radii), Modifier.size(56.dp))
            expressive -> Cover(artwork, 56.dp, RoundedCornerShape(12.dp), Modifier.size(56.dp))
            else -> Cover(artwork, 64.dp, if (round) CircleShape else RoundedCornerShape(4.dp), Modifier.size(64.dp))
        }
        Column(Modifier.weight(1f)) {
            BasicText(title, style = geist(16.sp).copy(color = roles.text), maxLines = 1, overflow = TextOverflow.Ellipsis)
            Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(5.dp)) {
                if (pinned) Glyph(MoreIcons.Pin, if (expressive) roles.primary else roles.accent, 14.dp)
                BasicText(subtitle, style = geist(14.sp).copy(color = roles.sub), maxLines = 1, overflow = TextOverflow.Ellipsis)
            }
        }
    }
}
