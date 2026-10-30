package io.github.honeyslatt.music.ui.library

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.grid.items
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.lazy.itemsIndexed
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.BasicText
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.produceState
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import io.github.honeyslatt.music.core.albumsLeftOut
import io.github.honeyslatt.music.core.artistsLeftOut
import io.github.honeyslatt.music.core.model.Album
import io.github.honeyslatt.music.core.model.Collection
import io.github.honeyslatt.music.core.model.ProviderKind
import io.github.honeyslatt.music.core.songsLeftOut
import io.github.honeyslatt.music.data.LibraryLists
import io.github.honeyslatt.music.data.Listing
import io.github.honeyslatt.music.ui.Icons
import io.github.honeyslatt.music.ui.components.BarAction
import io.github.honeyslatt.music.ui.components.CoverCard
import io.github.honeyslatt.music.ui.components.Glyph
import io.github.honeyslatt.music.ui.components.GridPage
import io.github.honeyslatt.music.ui.components.ItemRow
import io.github.honeyslatt.music.ui.components.LibraryRow
import io.github.honeyslatt.music.ui.components.Line
import io.github.honeyslatt.music.ui.components.ListPage
import io.github.honeyslatt.music.ui.components.SectionHeader
import io.github.honeyslatt.music.ui.components.SourceChips
import io.github.honeyslatt.music.ui.components.TrackRow
import io.github.honeyslatt.music.ui.components.byline
import io.github.honeyslatt.music.ui.components.full
import io.github.honeyslatt.music.ui.components.gridCell
import io.github.honeyslatt.music.ui.components.label
import io.github.honeyslatt.music.ui.components.press
import io.github.honeyslatt.music.ui.nav.Navigator
import io.github.honeyslatt.music.ui.nav.Route
import io.github.honeyslatt.music.ui.theme.geist
import io.github.honeyslatt.music.ui.theme.roles
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext

/** What the library pages need besides the lists. */
class LibraryContext(
    val core: io.github.honeyslatt.music.data.Core,
    val nav: Navigator,
    val lists: LibraryLists,
    val services: List<ProviderKind>,
    val account: @Composable () -> Unit,
    /** Settings → Artwork → Animated artwork. */
    val animatedArtwork: Boolean = true,
    @property:androidx.annotation.OptIn(androidx.media3.common.util.UnstableApi::class)
    val playback: io.github.honeyslatt.music.playback.Playback,
)

/** Library: the four lists, then Recently Added. */
@Composable
fun LibraryRoot(c: LibraryContext) {
    var source by rememberSaveable { mutableStateOf<ProviderKind?>(null) }
    val added = c.lists.recentlyAdded.items.filter { source == null || it.provider() == source }.take(8)
    GridPage(c.nav, "Library", account = c.account) {
        full {
            androidx.compose.foundation.layout.Column(Modifier.padding(top = 6.dp)) {
                listOf(
                    Triple(Icons.Listmusic, "Playlists", Route.Playlists),
                    Triple(Icons.Mic, "Artists", Route.Artists),
                    Triple(Icons.Album, "Albums", Route.Albums),
                    Triple(Icons.Note, "Songs", Route.Songs),
                ).forEachIndexed { i, (icon, label, route) ->
                    LibraryRow(icon, label, last = i == 3) { c.nav.push(route) }
                }
            }
        }
        full { SectionHeader("Recently Added") { c.nav.push(Route.RecentlyAdded) } }
        full { SourceChips(source, c.services) { source = it } }
        full { Spacer(Modifier.height(8.dp)) }
        collections(c, added, c.lists.recentlyAdded)
    }
}

@Composable
fun RecentlyAdded(c: LibraryContext) {
    var source by rememberSaveable { mutableStateOf<ProviderKind?>(null) }
    val added = c.lists.recentlyAdded.items.filter { source == null || it.provider() == source }
    GridPage(c.nav, "Recently Added") {
        full { SourceChips(source, c.services) { source = it } }
        full { Spacer(Modifier.height(10.dp)) }
        collections(c, added, c.lists.recentlyAdded)
    }
}

private fun Collection.provider() = when (this) {
    is Collection.Album -> v1.id.provider
    is Collection.Playlist -> v1.id.provider
    is Collection.Station -> v1.id.provider
}

private fun androidx.compose.foundation.lazy.grid.LazyGridScope.collections(
    c: LibraryContext,
    items: List<Collection>,
    listing: Listing<*>,
) {
    if (items.isEmpty()) {
        full { Empty(listing, "Nothing here yet", "Albums and playlists you add show here.") }
        return
    }
    items(items) { item ->
        when (item) {
            is Collection.Album -> CoverCard(item.v1.artwork, item.v1.title, item.v1.artists.joinToString(", "), item.v1.explicit, modifier = Modifier.gridCell()) {
                c.nav.push(Route.Album(item.v1.id))
            }
            is Collection.Playlist -> CoverCard(item.v1.artwork, item.v1.name, item.v1.curator ?: "Playlist", modifier = Modifier.gridCell()) {
                c.nav.push(Route.Playlist(item.v1.id))
            }
            is Collection.Station -> CoverCard(item.v1.artwork, item.v1.name, "Station", modifier = Modifier.gridCell()) {}
        }
    }
}

/** A list still loading, failed, or empty. */
@Composable
fun Empty(listing: Listing<*>, title: String, detail: String) {
    val (t, d) = when {
        listing.error != null -> "Couldn't load this" to listing.error
        !listing.done -> "Loading…" to ""
        else -> title to detail
    }
    androidx.compose.foundation.layout.Column(
        Modifier.fillMaxWidth().padding(horizontal = 32.dp, vertical = 48.dp),
        horizontalAlignment = Alignment.CenterHorizontally,
    ) {
        BasicText(t, style = geist(17.sp, FontWeight.SemiBold).copy(color = roles.text, textAlign = TextAlign.Center))
        if (d.isNotEmpty()) {
            Spacer(Modifier.height(4.dp))
            BasicText(d, style = geist(14.5.sp).copy(color = roles.sub, textAlign = TextAlign.Center))
        }
    }
}

@Composable
fun Playlists(c: LibraryContext) {
    val playlists = c.lists.playlists
    val favourites = playlists.items.firstOrNull { it.favourites }
    ListPage(c.nav, "Playlists", actions = listOf(BarAction(Icons.Search, "Search Playlists") {})) {
        item {
            ItemRow(
                artwork = null,
                title = "New Playlist…",
                leading = {
                    Box(Modifier.fillMaxWidth().height(56.dp).background(roles.surface, RoundedCornerShape(8.dp)), contentAlignment = Alignment.Center) {
                        Glyph(Icons.Plus, roles.tint, 26.dp)
                    }
                },
            ) {}
        }
        if (favourites != null) {
            item {
                ItemRow(
                    artwork = null,
                    title = "Favourite Songs",
                    subtitle = favourites.trackCount?.let { songs(it.toInt()) },
                    leading = {
                        Box(
                            Modifier.fillMaxWidth().height(56.dp).background(
                                Brush.linearGradient(listOf(roles.accent, roles.mauve)),
                                RoundedCornerShape(8.dp),
                            ),
                            contentAlignment = Alignment.Center,
                        ) { Glyph(Icons.Starf, roles.onAccent, 26.dp) }
                    },
                ) { c.nav.push(Route.Playlist(favourites.id)) }
            }
        }
        val rest = playlists.items.filter { !it.favourites }
        if (rest.isEmpty() && favourites == null) {
            item { Empty(playlists, "No playlists", "Playlists you make or add show here.") }
        }
        items(rest, key = { it.id.provider.name + it.id.id }) { p ->
            ItemRow(
                p.artwork,
                p.name,
                subtitle = p.trackCount?.let { songs(it.toInt()) } ?: p.curator,
                logo = p.id.provider.takeIf { it != ProviderKind.APPLE_MUSIC },
                star = p.inFavourites == true,
            ) { c.nav.push(Route.Playlist(p.id)) }
        }
    }
}

fun songs(n: Int) = if (n == 1) "1 song" else "$n songs"

@Composable
fun Albums(c: LibraryContext) {
    var source by rememberSaveable { mutableStateOf<ProviderKind?>(null) }
    val all = c.lists.albums.items
    val shown by produceState(all, all, source) {
        value = withContext(Dispatchers.Default) {
            if (source != null) {
                all.filter { it.id.provider == source }
            } else {
                val out = albumsLeftOut(all).toSet()
                all.filterIndexed { i, _ -> i.toUInt() !in out }
            }
        }
    }
    GridPage(c.nav, "Albums", actions = listOf(BarAction(Icons.Search, "Search Albums") {}, BarAction(Icons.Sort, "Sort") {})) {
        full { SourceChips(source, c.services) { source = it } }
        full { Spacer(Modifier.height(10.dp)) }
        if (shown.isEmpty()) {
            full { Empty(c.lists.albums, "No albums here", "Nothing matches this source.") }
        }
        items(shown, key = { it.id.provider.name + it.id.id }) { album -> AlbumCard(c, album) }
    }
}

@Composable
private fun AlbumCard(c: LibraryContext, album: Album) {
    CoverCard(album.artwork, album.title, album.byline(), album.explicit, modifier = Modifier.gridCell()) {
        c.nav.push(Route.Album(album.id))
    }
}

@Composable
fun Artists(c: LibraryContext) {
    val all = c.lists.artists.items
    val shown by produceState(all, all) {
        value = withContext(Dispatchers.Default) {
            val out = artistsLeftOut(all).toSet()
            all.filterIndexed { i, _ -> i.toUInt() !in out }
        }
    }
    ListPage(c.nav, "Artists", actions = listOf(BarAction(Icons.Search, "Search Artists") {})) {
        if (shown.isEmpty()) item { Empty(c.lists.artists, "No artists", "Artists of the songs you add show here.") }
        items(shown, key = { it.id.provider.name + it.id.id }) { artist ->
            ItemRow(artist.artwork, artist.name, round = true) { c.nav.push(Route.LibraryArtist(artist)) }
        }
    }
}

@Composable
fun Songs(c: LibraryContext) {
    var source by rememberSaveable { mutableStateOf<ProviderKind?>(null) }
    val all = c.lists.songs.items
    val shown by produceState(all, all, source) {
        value = withContext(Dispatchers.Default) {
            if (source != null) {
                all.filter { it.id.provider == source }
            } else {
                val out = songsLeftOut(all).toSet()
                all.filterIndexed { i, _ -> i.toUInt() !in out }
            }
        }
    }
    ListPage(c.nav, "Songs", actions = listOf(BarAction(Icons.Search, "Search Songs") {}, BarAction(Icons.Sort, "Sort") {})) {
        item { SourceChips(source, c.services) { source = it } }
        item { PlayShuffle(shown.isNotEmpty(), { c.playback.play(shown, 0) }, { c.playback.shuffle(shown) }) }
        if (shown.isEmpty()) item { Empty(c.lists.songs, "No songs here", "Nothing matches this source.") }
        itemsIndexed(shown, key = { _, t -> t.id.provider.name + t.id.id }) { i, track -> TrackRow(track) { c.playback.play(shown, i) } }
        if (shown.isNotEmpty()) {
            item {
                BasicText(
                    if (c.lists.songs.done) songs(shown.size) else "${songs(shown.size)} so far…",
                    Modifier.padding(start = 20.dp, top = 16.dp),
                    style = geist(13.5.sp).copy(color = roles.sub),
                )
            }
        }
    }
}

/** Play and Shuffle, side by side. */
@Composable
fun PlayShuffle(enabled: Boolean, onPlay: () -> Unit = {}, onShuffle: () -> Unit = {}) {
    Row(
        Modifier.fillMaxWidth().padding(start = 20.dp, end = 20.dp, top = 10.dp, bottom = 8.dp),
        horizontalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        listOf(Triple(Icons.Play, "Play", onPlay), Triple(Icons.Shuffle, "Shuffle", onShuffle)).forEach { (icon, label, action) ->
            val shape = RoundedCornerShape(12.dp)
            Row(
                Modifier
                    .weight(1f)
                    .height(48.dp)
                    .press(shape, shrink = 0.97f) { if (enabled) action() }
                    .background(roles.surface, shape),
                horizontalArrangement = Arrangement.Center,
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Glyph(icon, roles.tint, 18.dp)
                Spacer(Modifier.padding(start = 8.dp))
                Line(label, geist(16.sp, FontWeight.SemiBold).copy(color = roles.tint))
            }
        }
    }
}

/** A tab or page that comes in a later step. */
@Composable
fun Coming(nav: Navigator, title: String, account: (@Composable () -> Unit)? = null) {
    ListPage(nav, title, account = account) {
        item {
            BasicText(
                "Coming next.",
                Modifier.padding(horizontal = 20.dp, vertical = 12.dp),
                style = geist(15.sp).copy(color = roles.sub),
            )
        }
    }
}
