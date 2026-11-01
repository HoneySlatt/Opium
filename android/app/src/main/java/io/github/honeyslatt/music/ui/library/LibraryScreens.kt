package io.github.honeyslatt.music.ui.library

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.grid.items
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.lazy.itemsIndexed
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.BasicText
import androidx.compose.runtime.Composable
import androidx.compose.ui.graphics.Color
import androidx.compose.foundation.layout.fillMaxSize
import io.github.honeyslatt.music.ui.components.anchor
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.produceState
import androidx.compose.runtime.remember
import androidx.compose.runtime.LaunchedEffect
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
import io.github.honeyslatt.music.core.model.Track
import io.github.honeyslatt.music.core.songsLeftOut
import io.github.honeyslatt.music.core.recentInTime
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
import io.github.honeyslatt.music.ui.components.sourcesFold
import io.github.honeyslatt.music.ui.components.TrackRow
import io.github.honeyslatt.music.ui.components.byline
import io.github.honeyslatt.music.ui.components.full
import io.github.honeyslatt.music.ui.components.gridCell
import io.github.honeyslatt.music.ui.components.label
import io.github.honeyslatt.music.ui.components.press
import io.github.honeyslatt.music.ui.nav.Navigator
import io.github.honeyslatt.music.ui.nav.Route
import io.github.honeyslatt.music.ui.actions.LocalActions
import io.github.honeyslatt.music.ui.actions.SheetSpec
import io.github.honeyslatt.music.ui.components.LocalMenus
import io.github.honeyslatt.music.ui.components.SearchReveal
import io.github.honeyslatt.music.ui.components.fold
import io.github.honeyslatt.music.ui.components.matches
import io.github.honeyslatt.music.ui.components.sort
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
    /** Settings → Library → Recently Added by date. */
    val recentByDate: Boolean = false,
    @property:androidx.annotation.OptIn(androidx.media3.common.util.UnstableApi::class)
    val playback: io.github.honeyslatt.music.playback.Playback,
    val library: io.github.honeyslatt.music.data.Library? = null,
) {
    /** Pulled down: the library read again, the spinner turning until `of` it is there. */
    fun refresh(of: (LibraryLists) -> Listing<*>): (suspend () -> Unit)? = library?.let { l -> { l.refresh(of) } }
}

/** As many as Apple Music shows under Recently Added. */
private const val RECENT_SHOWN = 66

/** Library: the four lists, then Recently Added. */
@Composable
fun LibraryRoot(c: LibraryContext) {
    var source by io.github.honeyslatt.music.ui.components.rememberSource(c.core, "recently-added")
    // Folded at first, the source kept still applying; opened at will.
    var sources by rememberSaveable { mutableStateOf(false) }
    val added = remember(c.lists.recentlyAdded.items, source, c.recentByDate) {
        // Each service's together, as the library lists them: Apple Music's,
        // SoundCloud's, Navidrome's; or all by when each was added.
        val all = c.lists.recentlyAdded.items.let { if (c.recentByDate) recentInTime(it) else it }
        all.filter { source == null || it.provider() == source }.take(RECENT_SHOWN)
    }
    GridPage(c.nav, "Library", account = c.account, refresh = c.refresh { it.recentlyAdded }) {
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
        full { SectionHeader("Recently Added", fold = sourcesFold(c.services, sources) { sources = !sources }) }
        full { SourceChips(sources, source, c.services) { source = it } }
        full { Spacer(Modifier.height(8.dp)) }
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
        full { Empty(listing, "Nothing here yet", "Albums and playlists you add show here.", io.github.honeyslatt.music.ui.components.Shapes.Grid) }
        return
    }
    items(items) { item ->
        val actions = LocalActions.current
        when (item) {
            is Collection.Album -> CoverCard(
                item.v1.artwork,
                item.v1.title,
                item.v1.artists.joinToString(", "),
                item.v1.explicit,
                modifier = Modifier.gridCell(),
                onMenu = { actions?.albumMenu(item.v1, it) },
            ) {
                c.nav.push(Route.Album(item.v1.id))
            }
            is Collection.Playlist -> CoverCard(
                item.v1.artwork,
                item.v1.name,
                item.v1.curator ?: "Playlist",
                modifier = Modifier.gridCell(),
                onMenu = { actions?.playlistMenu(item.v1, it) },
            ) {
                c.nav.push(Route.Playlist(item.v1.id))
            }
            is Collection.Station -> CoverCard(item.v1.artwork, item.v1.name, "Station", modifier = Modifier.gridCell()) {}
        }
    }
}

/** A list still loading (in the shapes of what comes, for Spotify's), failed, or empty. */
@Composable
fun Empty(listing: Listing<*>, title: String, detail: String, shapes: io.github.honeyslatt.music.ui.components.Shapes = io.github.honeyslatt.music.ui.components.Shapes.Rows) {
    val (t, d) = when {
        listing.error != null -> "Couldn't load this" to listing.error
        !listing.done -> return io.github.honeyslatt.music.ui.components.Loader(shapes)
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
    val actions = LocalActions.current
    val playlists = c.lists.playlists
    var searching by rememberSaveable { mutableStateOf(false) }
    var query by rememberSaveable { mutableStateOf("") }
    val favourites = playlists.items.firstOrNull { it.favourites }?.takeIf { matches(query, "Favourite Songs") }
    ListPage(c.nav, "Playlists", actions = listOf(BarAction(Icons.Search, "Search Playlists") { searching = !searching }), refresh = c.refresh { it.playlists }) {
        item { SearchReveal(searching, query, "Search Playlists") { query = it } }
        item {
            ItemRow(
                artwork = null,
                title = "New Playlist…",
                leading = {
                    Box(Modifier.fillMaxWidth().height(56.dp).background(roles.surface, RoundedCornerShape(8.dp)), contentAlignment = Alignment.Center) {
                        Glyph(Icons.Plus, roles.tint, 26.dp)
                    }
                },
            ) { actions?.sheet = SheetSpec.PlaylistForm(null) }
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
                    onMenu = { actions?.playlistMenu(favourites, it) },
                ) { c.nav.push(Route.Playlist(favourites.id)) }
            }
        }
        // SoundCloud lists the user's own playlists, then those liked: one
        // both is shown once (twice, its key would crash the list).
        val rest = playlists.items.filter { !it.favourites && matches(query, it.name, it.curator) }.distinctBy { it.id }
        if (rest.isEmpty() && favourites == null) {
            item { Empty(playlists, "No playlists", "Playlists you make or add show here.") }
        }
        items(rest, key = { it.id.provider.name + it.id.id }) { p ->
            ItemRow(
                p.artwork,
                p.name,
                subtitle = p.trackCount?.let { songs(it.toInt()) } ?: p.curator,
                logo = p.id.provider.takeIf { it != ProviderKind.APPLE_MUSIC },
                star = actions?.isFavourite(p) ?: (p.inFavourites == true),
                leading = if (p.artwork == null) {
                    { io.github.honeyslatt.music.ui.components.PlaylistCover(p, 56.dp, RoundedCornerShape(8.dp), Modifier.size(56.dp)) }
                } else {
                    null
                },
                onMenu = { actions?.playlistMenu(p, it) },
            ) { c.nav.push(Route.Playlist(p.id)) }
        }
    }
}

fun songs(n: Int) = if (n == 1) "1 song" else "$n songs"

@Composable
fun Albums(c: LibraryContext) {
    var source by io.github.honeyslatt.music.ui.components.rememberSource(c.core, "albums")
    // Folded at first, the source kept still applying; opened at will.
    var sources by rememberSaveable { mutableStateOf(false) }
    var searching by rememberSaveable { mutableStateOf(false) }
    var query by rememberSaveable { mutableStateOf("") }
    var order by remember { mutableStateOf(c.core.setting("albums-sort")?.let { runCatching { AlbumSort.valueOf(it) }.getOrNull() } ?: AlbumSort.Added) }
    val menus = LocalMenus.current
    val all = c.lists.albums.items
    // What another service has too is left out once, not at every letter typed.
    val some by produceState(all.distinctBy { it.id }, all, source) {
        value = withContext(Dispatchers.Default) {
            if (source != null) {
                all.filter { it.id.provider == source }
            } else {
                val out = albumsLeftOut(all).toSet()
                all.filterIndexed { i, _ -> i.toUInt() !in out }
            }.distinctBy { it.id }
        }
    }
    val shown by produceState(some, some, query, order) {
        value = withContext(Dispatchers.Default) {
            some.filter { matches(query, it.title, it.artists.joinToString(", ")) }.sortedBy(order)
        }
    }
    GridPage(
        c.nav,
        "Albums",
        actions = listOf(
            BarAction(Icons.Search, "Search Albums") { searching = !searching },
            BarAction(Icons.Sort, "Sort") { rect ->
                menus.sort(rect, AlbumSort.entries, order, AlbumSort::label) {
                    order = it
                    c.core.setSetting("albums-sort", it.name)
                }
            },
        ),
        fold = sourcesFold(c.services, sources) { sources = !sources },
        refresh = c.refresh { it.albums },
    ) {
        full { SearchReveal(searching, query, "Search Albums") { query = it } }
        full { SourceChips(sources, source, c.services) { source = it } }
        full { Spacer(Modifier.height(10.dp)) }
        if (shown.isEmpty()) {
            full { Empty(c.lists.albums, "No albums here", "Nothing matches this source.", io.github.honeyslatt.music.ui.components.Shapes.Grid) }
        }
        items(shown, key = { it.id.provider.name + it.id.id }) { album -> AlbumCard(c, album) }
    }
}

@Composable
private fun AlbumCard(c: LibraryContext, album: Album) {
    val actions = LocalActions.current
    CoverCard(album.artwork, album.title, album.byline(), album.explicit, modifier = Modifier.gridCell(), onMenu = { actions?.albumMenu(album, it) }) {
        c.nav.push(Route.Album(album.id))
    }
}

@Composable
fun Artists(c: LibraryContext) {
    val all = c.lists.artists.items
    val shown by produceState(all.distinctBy { it.id }, all) {
        value = withContext(Dispatchers.Default) {
            val out = artistsLeftOut(all).toSet()
            // Two of the library's artists can be one in the catalog
            // (one name spelt two ways): it shows once.
            all.filterIndexed { i, _ -> i.toUInt() !in out }.distinctBy { it.id }
        }
    }
    var searching by rememberSaveable { mutableStateOf(false) }
    var query by rememberSaveable { mutableStateOf("") }
    val matching = shown.filter { matches(query, it.name) }
    ListPage(c.nav, "Artists", actions = listOf(BarAction(Icons.Search, "Search Artists") { searching = !searching }), refresh = c.refresh { it.artists }) {
        item { SearchReveal(searching, query, "Search Artists") { query = it } }
        if (matching.isEmpty()) item { Empty(c.lists.artists, "No artists", "Artists of the songs you add show here.") }
        items(matching, key = { it.id.provider.name + it.id.id }) { artist ->
            ItemRow(artist.artwork, artist.name, round = true) { c.nav.push(Route.LibraryArtist(artist)) }
        }
    }
}

@Composable
fun Songs(c: LibraryContext) {
    var source by io.github.honeyslatt.music.ui.components.rememberSource(c.core, "songs")
    // Folded at first, the source kept still applying; opened at will.
    var sources by rememberSaveable { mutableStateOf(false) }
    var searching by rememberSaveable { mutableStateOf(false) }
    var query by rememberSaveable { mutableStateOf("") }
    var order by remember { mutableStateOf(c.core.setting("songs-sort")?.let { runCatching { SongSort.valueOf(it) }.getOrNull() } ?: SongSort.Name) }
    val menus = LocalMenus.current
    // Newest first is a list of its own, in the order Apple keeps.
    LaunchedEffect(order) { if (order == SongSort.Added) c.library?.loadNewestSongs() }
    val listing = if (order == SongSort.Added) c.lists.newestSongs else c.lists.songs
    val all = listing.items
    // What another service has too is left out once, not at every letter typed.
    val some by produceState(all.distinctBy { it.id }, all, source) {
        value = withContext(Dispatchers.Default) {
            val some = if (source != null) {
                all.filter { it.id.provider == source }
            } else {
                val out = songsLeftOut(all).toSet()
                all.filterIndexed { i, _ -> i.toUInt() !in out }
            }
            // Apple lists a song saved twice twice: once is enough.
            some.distinctBy { it.id }
        }
    }
    val shown by produceState(some, some, query, order) {
        value = withContext(Dispatchers.Default) {
            some.filter { matches(query, it.title, it.artists.joinToString(", "), it.album) }.sortedBy(order)
        }
    }
    if (io.github.honeyslatt.music.ui.theme.look.layout.header == io.github.honeyslatt.music.core.model.Header.BANNER) {
        LikedSongs(c, listing, shown, source, { source = it }, searching, { searching = !searching }, query, { query = it }, order) {
            order = it
            c.core.setSetting("songs-sort", it.name)
        }
        return
    }
    ListPage(
        c.nav,
        "Songs",
        actions = listOf(
            BarAction(Icons.Search, "Search Songs") { searching = !searching },
            BarAction(Icons.Sort, "Sort") { rect ->
                menus.sort(rect, SongSort.entries, order, SongSort::label) {
                    order = it
                    c.core.setSetting("songs-sort", it.name)
                }
            },
        ),
        fold = sourcesFold(c.services, sources) { sources = !sources },
        refresh = c.refresh { if (order == SongSort.Added) it.newestSongs else it.songs },
    ) {
        item { SearchReveal(searching, query, "Search Songs") { query = it } }
        item { SourceChips(sources, source, c.services) { source = it } }
        item { PlayRow(c, shown) }
        if (shown.isEmpty()) item { Empty(listing, "No songs here", "Nothing matches this source.") }
        itemsIndexed(shown, key = { _, t -> t.id.provider.name + t.id.id }) { i, track -> TrackRow(track) { c.playback.play(shown, i) } }
        if (shown.isNotEmpty()) {
            item {
                BasicText(
                    if (listing.done) songs(shown.size) else "${songs(shown.size)} so far…",
                    Modifier.padding(start = 20.dp, top = 16.dp),
                    style = geist(13.5.sp).copy(color = roles.sub),
                )
            }
        }
    }
}

/**
 * The library's songs as Spotify's Liked Songs: a playlist's page with
 * the violet heart, the user and the count; the services to list in a
 * menu by the buttons instead of chips.
 */
@androidx.annotation.OptIn(androidx.media3.common.util.UnstableApi::class)
@Composable
private fun LikedSongs(
    c: LibraryContext,
    listing: io.github.honeyslatt.music.data.Listing<Track>,
    shown: List<Track>,
    source: ProviderKind?,
    setSource: (ProviderKind?) -> Unit,
    searching: Boolean,
    toggleSearch: () -> Unit,
    query: String,
    setQuery: (String) -> Unit,
    order: SongSort,
    setOrder: (SongSort) -> Unit,
) {
    val menus = LocalMenus.current
    val profile by c.core.appleProfile.collectAsStateWithLifecycle()
    io.github.honeyslatt.music.ui.detail.DetailPage(
        c.nav,
        "Songs",
        Color(0xFF5038A0),
        listOf(
            BarAction(Icons.Search, "Search Songs") { toggleSearch() },
            BarAction(Icons.Sort, "Sort") { rect -> menus.sort(rect, SongSort.entries, order, SongSort::label, setOrder) },
        ),
    ) {
        item {
            io.github.honeyslatt.music.ui.detail.DetailHeader(
                artwork = null,
                motion = null,
                title = "Songs",
                byline = profile?.name,
                onByline = null,
                meta = {
                    BasicText(
                        if (listing.done) songs(shown.size) else "${songs(shown.size)} so far…",
                        style = geist(14.sp).copy(color = roles.sub),
                    )
                },
                art = {
                    Box(
                        Modifier.fillMaxSize().background(Brush.linearGradient(Loved), RoundedCornerShape(4.dp)),
                        contentAlignment = Alignment.Center,
                    ) { Glyph(io.github.honeyslatt.music.ui.MoreIcons.HeartFill, Color.White, 96.dp) }
                },
            )
        }
        item { SearchReveal(searching, query, "Search Songs") { setQuery(it) } }
        item {
            PlayRow(c, shown) {
                // All the services, or one, from a menu, as on the desktop.
                if (c.services.size > 1) {
                    val anchor = io.github.honeyslatt.music.ui.components.rememberAnchor()
                    Row(
                        Modifier.padding(start = 9.dp, end = 4.dp)
                            .anchor(anchor)
                            .press(RoundedCornerShape(8.dp)) {
                                menus.sort(anchor.rect, listOf<ProviderKind?>(null) + c.services, source, { it?.label() ?: "All services" }, setSource)
                            },
                        verticalAlignment = Alignment.CenterVertically,
                        horizontalArrangement = Arrangement.spacedBy(4.dp),
                    ) {
                        BasicText(source?.label() ?: "All services", style = geist(14.sp, FontWeight.Medium).copy(color = roles.sub))
                        Glyph(Icons.Down, roles.sub, 16.dp)
                    }
                }
            }
        }
        if (shown.isEmpty()) item { Empty(listing, "No songs here", "Nothing matches this source.") }
        itemsIndexed(shown, key = { _, t -> t.id.provider.name + t.id.id }) { i, track -> TrackRow(track) { c.playback.play(shown, i) } }
    }
}

/**
 * Play and Shuffle under a page's header. In a theme with Spotify's banner,
 * the page's actions come first on the left, after `lead`; Shuffle is a
 * switch, green with a dot while on, and Play a round button in the accent
 * that plays `tracks` as shuffle is, or pauses and goes on while one of
 * them plays.
 */
@androidx.annotation.OptIn(androidx.media3.common.util.UnstableApi::class)
@Composable
fun PlayRow(c: LibraryContext, tracks: List<Track>, lead: @Composable () -> Unit = {}) {
    if (io.github.honeyslatt.music.ui.theme.look.layout.header != io.github.honeyslatt.music.core.model.Header.BANNER) {
        PlayShuffle(tracks.isNotEmpty(), { c.playback.play(tracks, 0) }, { c.playback.shuffle(tracks) })
        return
    }
    val playback = c.playback
    val shuffling by playback.shuffleOn.collectAsStateWithLifecycle()
    val now by playback.now.collectAsStateWithLifecycle()
    val playing by playback.playing.collectAsStateWithLifecycle()
    val here = now != null && tracks.any { it.id == now?.id }
    Row(
        // The first icon's glyph lines up with the title above.
        Modifier.fillMaxWidth().padding(start = 7.dp, end = 16.dp, top = 4.dp, bottom = 8.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        lead()
        io.github.honeyslatt.music.ui.detail.LocalBannerActions.current.forEach { RowAction(it) }
        Spacer(Modifier.weight(1f))
        Box(
            Modifier.size(48.dp).press(CircleShape) { playback.setShuffle(!shuffling) },
            contentAlignment = Alignment.Center,
        ) {
            Glyph(Icons.Shuffle, if (shuffling) roles.accent else roles.sub, 28.dp)
            if (shuffling) {
                Box(Modifier.align(Alignment.BottomCenter).padding(bottom = 2.dp).size(4.dp).background(roles.accent, CircleShape))
            }
        }
        Spacer(Modifier.width(16.dp))
        Box(
            Modifier.size(56.dp).press(CircleShape, shrink = 0.94f) {
                when {
                    tracks.isEmpty() -> {}
                    here -> playback.toggle()
                    shuffling -> playback.shuffle(tracks)
                    else -> playback.play(tracks, 0)
                }
            }.background(roles.accent, CircleShape),
            contentAlignment = Alignment.Center,
        ) { Glyph(if (here && playing) Icons.Pause else Icons.Play, roles.onAccent, 26.dp) }
    }
}

/** A page's action as Spotify draws it by Play: a bare icon; added, a green
 *  disc with a check; a playlist starred, a green star. */
@Composable
private fun RowAction(a: BarAction) {
    val anchor = io.github.honeyslatt.music.ui.components.rememberAnchor()
    Box(
        Modifier.size(44.dp).anchor(anchor).press(CircleShape) { a.onClick(anchor.rect) },
        contentAlignment = Alignment.Center,
    ) {
        when (a.icon) {
            Icons.Check -> Box(Modifier.size(26.dp).background(roles.accent, CircleShape), contentAlignment = Alignment.Center) {
                Glyph(Icons.Check, roles.onAccent, 16.dp)
            }
            Icons.Plus -> Glyph(Icons.Pluscircle, roles.sub, 28.dp)
            Icons.Starf -> Glyph(Icons.Starf, roles.accent, 26.dp)
            else -> Glyph(a.icon, roles.sub, 26.dp)
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

enum class AlbumSort(val label: String) { Added("Date Added"), Name("Name"), Artist("Artist"), Released("Release Date") }

enum class SongSort(val label: String) { Name("Name"), Artist("Artist"), Album("Album"), Added("Date Added") }

/**
 * Albums in `order`: names without case or accents, dates newest first;
 * those without the date sorted by go last.
 */
private fun List<Album>.sortedBy(order: AlbumSort): List<Album> = when (order) {
    AlbumSort.Name -> sortedOnce { fold(it.title) }
    AlbumSort.Artist -> sortedOnce { fold(it.artists.joinToString(", ")) + '\u0000' + fold(it.title) }
    AlbumSort.Added -> sortedWith(compareBy(nullsLast(reverseOrder())) { it.added })
    AlbumSort.Released -> sortedWith(compareBy(nullsLast(reverseOrder())) { it.released ?: it.year?.toString() })
}

/** Songs in `order`; Date Added keeps the order they came in, newest first. */
private fun List<Track>.sortedBy(order: SongSort): List<Track> = when (order) {
    SongSort.Name -> sortedOnce { fold(it.title) }
    SongSort.Artist -> sortedOnce { fold(it.artists.joinToString(", ")) + '\u0000' + fold(it.title) }
    SongSort.Album -> map { fold(it.album.orEmpty()) to it }
        .sortedWith(compareBy({ it.first }, { it.second.discNumber?.toInt() ?: 1 }, { it.second.trackNumber?.toInt() ?: 0 }))
        .map { it.second }
    SongSort.Added -> this
}

/** Sorted by `key`, worked out once for each item rather than at every comparison. */
private fun <T> List<T>.sortedOnce(key: (T) -> String): List<T> =
    map { key(it) to it }.sortedBy { it.first }.map { it.second }
