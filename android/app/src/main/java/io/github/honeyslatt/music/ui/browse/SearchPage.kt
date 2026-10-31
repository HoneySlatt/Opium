package io.github.honeyslatt.music.ui.browse

import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyRow
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.BasicText
import androidx.compose.runtime.Composable
import androidx.compose.foundation.layout.statusBars
import androidx.compose.foundation.layout.windowInsetsTopHeight
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.produceState
import androidx.compose.runtime.remember
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.shadow
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import io.github.honeyslatt.music.core.RecentlySearched
import io.github.honeyslatt.music.core.SearchScope
import io.github.honeyslatt.music.core.Searched
import io.github.honeyslatt.music.core.keepSearched
import io.github.honeyslatt.music.core.keptSearched
import io.github.honeyslatt.music.core.model.FavouriteKind
import io.github.honeyslatt.music.ui.components.Cover
import io.github.honeyslatt.music.ui.components.ExplicitBadge
import io.github.honeyslatt.music.ui.components.anchor
import io.github.honeyslatt.music.ui.components.rememberAnchor
import io.github.honeyslatt.music.ui.components.underline
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.CircleShape
import io.github.honeyslatt.music.core.songsLeftOut
import io.github.honeyslatt.music.data.LibraryLists
import io.github.honeyslatt.music.core.model.Artist
import io.github.honeyslatt.music.core.model.ProviderKind
import io.github.honeyslatt.music.core.model.SearchResults
import io.github.honeyslatt.music.ui.Icons
import io.github.honeyslatt.music.ui.actions.LocalActions
import io.github.honeyslatt.music.ui.components.Fold
import io.github.honeyslatt.music.ui.components.sourcesFold
import io.github.honeyslatt.music.ui.components.CoverCard
import io.github.honeyslatt.music.ui.components.Glyph
import io.github.honeyslatt.music.ui.components.ItemRow
import io.github.honeyslatt.music.ui.components.Line
import io.github.honeyslatt.music.ui.components.ListPage
import io.github.honeyslatt.music.ui.components.SearchReveal
import io.github.honeyslatt.music.ui.components.SectionHeader
import io.github.honeyslatt.music.ui.components.SourceChips
import io.github.honeyslatt.music.ui.components.TrackRow
import io.github.honeyslatt.music.ui.components.matches
import io.github.honeyslatt.music.ui.components.press
import io.github.honeyslatt.music.ui.detail.Loading
import io.github.honeyslatt.music.ui.detail.listing
import io.github.honeyslatt.music.ui.detail.load
import io.github.honeyslatt.music.ui.library.Empty
import io.github.honeyslatt.music.ui.library.LibraryContext
import io.github.honeyslatt.music.ui.nav.Route
import io.github.honeyslatt.music.ui.theme.geist
import io.github.honeyslatt.music.ui.theme.roles
import kotlinx.coroutines.delay

/** How many finds Recently Searched keeps. */
private const val RECENT = 20

/** How many songs a search shows before its title is tapped. */
private const val SONGS = 6

/**
 * Search: the field, `Apple Music | Your Library` (remembered), and what
 * matches: the top result, songs, albums, artists, playlists. Before a
 * word is typed, what was opened or played from searches lately, as Apple
 * Music lists it.
 */
@Composable
fun SearchPage(c: LibraryContext) {
    var query by rememberSaveable { mutableStateOf("") }
    var library by remember { mutableStateOf(c.core.setting("search-scope") == "library") }
    var source by io.github.honeyslatt.music.ui.components.rememberSource(c.core, "search")
    // Folded at first, the source kept still applying; opened at will.
    var sources by rememberSaveable { mutableStateOf(false) }
    // The sections whose title showed all they found: Songs, Albums, Artists.
    var unfolded by remember { mutableStateOf(emptySet<String>()) }
    var recent by remember { mutableStateOf(keptSearched(c.core.searched)) }
    val words = query.trim()
    // Each search starts with its first results only.
    androidx.compose.runtime.LaunchedEffect(words, library, source) { unfolded = emptySet() }
    // A service chosen is asked alone: its own copies of what others have too.
    val results by produceState<Loading<SearchResults>?>(null, words, library, source) {
        if (words.isEmpty()) {
            value = null
            return@produceState
        }
        // Typed on: asked once the typing pauses.
        delay(300)
        value = Loading.Pending
        val lists = c.lists
        value = load {
            val found = c.core.session.search(if (library) SearchScope.LIBRARY else SearchScope.CATALOG, words, source)
            if (library && (source == null || source == ProviderKind.SOUND_CLOUD)) found.withLikes(lists, words) else found
        }
    }
    // A result opened or played: first in Recently Searched, once.
    val keep: (Searched, Boolean) -> Unit = { item, inLibrary ->
        recent = (listOf(RecentlySearched(item, inLibrary)) + recent.filter { it.item.id() != item.id() }).take(RECENT)
        keepSearched(c.core.searched, recent)
    }
    // Spotify's: the genres in tiles until the field is touched; back
    // from the search, the tiles again.
    val tiles = io.github.honeyslatt.music.ui.theme.look.mobile.search == io.github.honeyslatt.music.core.model.SearchView.TILES
    var active by rememberSaveable { mutableStateOf(false) }
    if (tiles && !active && query.isEmpty()) {
        BrowseAll(c) { active = true }
        return
    }
    androidx.activity.compose.BackHandler(enabled = tiles && active) {
        active = false
        query = ""
    }
    // The title folds the source chips.
    ListPage(c.nav, "Search", fold = sourcesFold(c.services, sources) { sources = !sources }) {
        item { SearchReveal(true, query, if (library) "Your Library" else "Artists, Songs, Lyrics and More", focusOnOpen = tiles) { query = it } }
        item {
            // The catalogue searched, as the source chosen says, or the
            // only service there is: a server's songs are only in the library.
            val catalogue = when (source ?: c.services.singleOrNull()) {
                ProviderKind.APPLE_MUSIC -> "Apple Music"
                ProviderKind.SOUND_CLOUD -> "SoundCloud"
                else -> "All"
            }
            Segmented(listOf(catalogue, "Your Library"), if (library) 1 else 0) {
                library = it == 1
                c.core.setSetting("search-scope", if (library) "library" else "catalog")
            }
        }
        item { SourceChips(sources, source, c.services) { source = it } }
        val r = results
        when {
            r == null -> recentSearches(c, recent, keep) {
                recent = emptyList()
                keepSearched(c.core.searched, recent)
            }
            r !is Loading.Ready -> item { Empty(r.listing(), "", "") }
            else -> results(c, r.value, library, source, words, { name -> Fold(name in unfolded) { unfolded = if (name in unfolded) unfolded - name else unfolded + name } }) { keep(it, library) }
        }
    }
}

/** Spotify's genres' colours, which the tiles take in turn. */
private val TileColours = listOf(
    0xFFE8115B, 0xFF006450, 0xFF1E3264, 0xFF8400E7, 0xFF8D67AB, 0xFF777777,
    0xFF477D95, 0xFF537AA1, 0xFFBA5D07, 0xFF148A08, 0xFFDC148C, 0xFF509BF5,
).map { androidx.compose.ui.graphics.Color(it) }

/**
 * Spotify's Search before a word: the account and the title, a white field
 * which opens the search, then Apple Music's genres in coloured tiles, each
 * opening its charts.
 */
@Composable
private fun BrowseAll(c: LibraryContext, onSearch: () -> Unit) {
    val genres by produceState<List<io.github.honeyslatt.music.core.model.Genre>>(emptyList()) {
        value = runCatching { c.core.session.genres() }.getOrDefault(emptyList())
    }
    androidx.compose.foundation.lazy.grid.LazyVerticalGrid(
        androidx.compose.foundation.lazy.grid.GridCells.Fixed(2),
        Modifier.fillMaxSize().background(roles.base),
        contentPadding = androidx.compose.foundation.layout.PaddingValues(horizontal = 16.dp),
        horizontalArrangement = androidx.compose.foundation.layout.Arrangement.spacedBy(12.dp),
        verticalArrangement = androidx.compose.foundation.layout.Arrangement.spacedBy(12.dp),
    ) {
        val span: androidx.compose.foundation.lazy.grid.LazyGridItemSpanScope.() -> androidx.compose.foundation.lazy.grid.GridItemSpan =
            { androidx.compose.foundation.lazy.grid.GridItemSpan(maxLineSpan) }
        item(span = span) {
            androidx.compose.foundation.layout.Column {
                Spacer(Modifier.windowInsetsTopHeight(androidx.compose.foundation.layout.WindowInsets.statusBars))
                androidx.compose.foundation.layout.Row(
                    Modifier.padding(top = 10.dp, bottom = 16.dp),
                    verticalAlignment = Alignment.CenterVertically,
                    horizontalArrangement = androidx.compose.foundation.layout.Arrangement.spacedBy(12.dp),
                ) {
                    c.account()
                    BasicText("Search", style = geist(24.sp, FontWeight.Bold).copy(color = roles.text))
                }
                androidx.compose.foundation.layout.Row(
                    Modifier.fillMaxWidth().height(48.dp)
                        .press(androidx.compose.foundation.shape.RoundedCornerShape(6.dp), shrink = 0.98f, onClick = onSearch)
                        .background(androidx.compose.ui.graphics.Color.White, androidx.compose.foundation.shape.RoundedCornerShape(6.dp))
                        .padding(horizontal = 14.dp),
                    verticalAlignment = Alignment.CenterVertically,
                    horizontalArrangement = androidx.compose.foundation.layout.Arrangement.spacedBy(12.dp),
                ) {
                    Glyph(Icons.Search, androidx.compose.ui.graphics.Color(0xFF121212), 24.dp)
                    BasicText("What do you want to listen to?", style = geist(16.sp, FontWeight.Medium).copy(color = androidx.compose.ui.graphics.Color(0xFF535353)))
                }
                if (genres.isNotEmpty()) {
                    BasicText("Browse all", Modifier.padding(top = 26.dp, bottom = 4.dp), style = geist(18.sp, FontWeight.Bold).copy(color = roles.text))
                }
            }
        }
        items(genres.size) { i ->
            val genre = genres[i]
            Box(
                Modifier.fillMaxWidth().height(98.dp)
                    .press(androidx.compose.foundation.shape.RoundedCornerShape(6.dp), shrink = 0.97f) { c.nav.push(io.github.honeyslatt.music.ui.nav.Route.Genre(genre)) }
                    .background(TileColours[i % TileColours.size], androidx.compose.foundation.shape.RoundedCornerShape(6.dp))
                    .padding(12.dp),
            ) {
                BasicText(genre.name, style = geist(17.sp, FontWeight.Bold).copy(color = androidx.compose.ui.graphics.Color.White))
            }
        }
        item(span = span) { Spacer(Modifier.height(io.github.honeyslatt.music.ui.chrome.ChromeHeight + 120.dp)) }
    }
}

/**
 * SoundCloud has no search of one's likes: they are matched here, in the
 * library as last read. A song another service has too shows once.
 */
private fun SearchResults.withLikes(lists: LibraryLists, words: String): SearchResults {
    val soundcloud = ProviderKind.SOUND_CLOUD
    // Taken in turns with the others', as the services' results are.
    val songs = inTurns(
        tracks,
        lists.songs.items.filter { it.id.provider == soundcloud && matches(words, it.title, it.artists.joinToString(", "), it.album) },
    )
    val out = songsLeftOut(songs).toSet()
    return copy(
        tracks = songs.filterIndexed { i, _ -> i.toUInt() !in out },
        playlists = playlists + lists.playlists.items.filter { it.id.provider == soundcloud && matches(words, it.name) },
    )
}

/** `a`'s and `b`'s items in turns, starting with `a`'s; the longer one's rest after. */
private fun <T> inTurns(a: List<T>, b: List<T>): List<T> =
    (0 until maxOf(a.size, b.size)).flatMap { i -> listOfNotNull(a.getOrNull(i), b.getOrNull(i)) }

/** The id of what was found, to keep it once. */
private fun Searched.id() = when (this) {
    is Searched.AlbumOpened -> album.id
    is Searched.ArtistOpened -> artist.id
    is Searched.SongPlayed -> track.id
    is Searched.PlaylistOpened -> playlist.id
}

private fun androidx.compose.foundation.lazy.LazyListScope.recentSearches(
    c: LibraryContext,
    recent: List<RecentlySearched>,
    keep: (Searched, Boolean) -> Unit,
    onClear: () -> Unit,
) {
    if (recent.isEmpty()) return
    item {
        Row(
            Modifier.fillMaxWidth().padding(start = 20.dp, end = 20.dp, top = 26.dp, bottom = 8.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            BasicText("Recently Searched", Modifier.weight(1f), style = geist(19.sp, FontWeight.Bold).copy(color = roles.text))
            BasicText("Clear", Modifier.press(onClick = onClear), style = geist(15.sp, FontWeight.Medium).copy(color = roles.tint))
        }
    }
    items(recent, key = { it.item.id().let { id -> "${id.provider}:${id.id}" } }) { entry ->
        SearchedRow(c, entry) { keep(entry.item, entry.library) }
    }
}

/**
 * A find of Recently Searched, as Apple Music shows it: a star when it is a
 * favourite, its cover (round for an artist), its title, what it is and by
 * whom, and whether it came from the library; a song has its menu.
 */
@Composable
private fun SearchedRow(c: LibraryContext, entry: RecentlySearched, onOpen: () -> Unit) {
    val actions = LocalActions.current
    val anchor = rememberAnchor()
    val more = rememberAnchor()
    class Shown(
        val artwork: io.github.honeyslatt.music.core.model.Artwork?,
        val title: String,
        val explicit: Boolean,
        val what: String,
        val round: Boolean,
        val favourite: Boolean,
        val menu: ((androidx.compose.ui.geometry.Rect) -> Unit)?,
        val song: Boolean,
        val open: () -> Unit,
    )
    fun line(kind: String, by: String?) = listOfNotNull(kind, by?.ifEmpty { null }).joinToString(" · ")
    val s = when (val i = entry.item) {
        is Searched.AlbumOpened -> i.album.let { a ->
            Shown(
                a.artwork, a.title, a.explicit, line("Album", a.artists.joinToString(", ")), false,
                actions?.isFavourite(FavouriteKind.ALBUM, a.catalogId ?: a.id, a.inFavourites) == true,
                { r -> actions?.albumMenu(a, r) }, false,
            ) { c.nav.push(Route.Album(a.id)) }
        }
        is Searched.ArtistOpened -> i.artist.let { a ->
            Shown(a.artwork, a.name, false, "Artist", true, actions?.isFavourite(a) == true, null, false) {
                c.nav.push(if (entry.library) Route.LibraryArtist(a) else Route.Artist(a.id))
            }
        }
        is Searched.SongPlayed -> i.track.let { t ->
            Shown(
                t.artwork, t.title, t.explicit, line("Song", t.artists.joinToString(", ")), false,
                t.id.provider != ProviderKind.NAVIDROME && actions?.isFavourite(FavouriteKind.SONG, t.id) == true,
                { r -> actions?.trackMenu(t, r) }, true,
            ) { c.playback.play(listOf(t), 0) }
        }
        is Searched.PlaylistOpened -> i.playlist.let { p ->
            Shown(p.artwork, p.name, false, line("Playlist", p.curator), false, actions?.isFavourite(p) == true, { r -> actions?.playlistMenu(p, r) }, false) {
                c.nav.push(Route.Playlist(p.id, p))
            }
        }
    }
    Row(
        Modifier
            .fillMaxWidth()
            .height(if (entry.library) 78.dp else 70.dp)
            .anchor(anchor)
            .press(onLongClick = s.menu?.let { m -> { m(anchor.rect) } }) {
                onOpen()
                s.open()
            },
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Box(Modifier.width(20.dp), contentAlignment = Alignment.Center) {
            if (s.favourite) Glyph(Icons.Starf, roles.tint, 9.dp)
        }
        Cover(s.artwork, 56.dp, if (s.round) CircleShape else RoundedCornerShape(6.dp), Modifier.size(56.dp))
        Row(
            Modifier.weight(1f).fillMaxHeight().underline(roles.line).padding(start = 14.dp, end = 6.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Column(Modifier.weight(1f)) {
                Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(5.dp)) {
                    Line(s.title, geist(16.sp).copy(color = roles.text), Modifier.weight(1f, fill = false))
                    if (s.explicit) ExplicitBadge()
                }
                Line(s.what, geist(13.5.sp).copy(color = roles.sub))
                if (entry.library) Line("From Your Library", geist(13.5.sp).copy(color = roles.sub))
            }
            if (s.song) {
                Box(
                    Modifier.size(44.dp).anchor(more).press(CircleShape) { s.menu?.invoke(more.rect) },
                    contentAlignment = Alignment.Center,
                ) { Glyph(Icons.More, roles.text, 20.dp) }
            }
        }
    }
}

private fun androidx.compose.foundation.lazy.LazyListScope.results(
    c: LibraryContext,
    r: SearchResults,
    library: Boolean,
    source: ProviderKind?,
    words: String,
    fold: (String) -> Fold,
    keep: (Searched) -> Unit,
) {
    if (r.tracks.isEmpty() && r.albums.isEmpty() && r.artists.isEmpty() && r.playlists.isEmpty()) {
        item {
            Empty(
                io.github.honeyslatt.music.data.Listing<Unit>(done = true),
                "No results",
                "Nothing ${
                    when {
                        library -> "in your library"
                        source == ProviderKind.APPLE_MUSIC -> "on Apple Music"
                        source == ProviderKind.SOUND_CLOUD -> "on SoundCloud"
                        else -> "on Apple Music or SoundCloud"
                    }
                } matches “$words”.",
            )
        }
        return
    }
    fun artist(a: Artist) = if (library) Route.LibraryArtist(a) else Route.Artist(a.id)
    r.artists.firstOrNull()?.let { top ->
        item { SectionHeader("Top Result") }
        item {
            ItemRow(top.artwork, top.name, "Artist", round = true) {
                keep(Searched.ArtistOpened(top))
                c.nav.push(artist(top))
            }
        }
    }
    if (r.tracks.isNotEmpty()) {
        // The first six; the title shows the others, when there are.
        val all = fold("Songs")
        item { SectionHeader("Songs", fold = all.takeIf { r.tracks.size > SONGS }) }
        val songs = if (all.open) r.tracks else r.tracks.take(SONGS)
        items(songs.size) { i ->
            TrackRow(songs[i]) {
                keep(Searched.SongPlayed(songs[i]))
                c.playback.play(songs, i)
            }
        }
    }
    if (r.albums.isNotEmpty()) {
        // A row to scroll; the title lays them all out in two columns.
        val all = fold("Albums")
        item { SectionHeader("Albums", fold = all.takeIf { r.albums.size > 2 }) }
        @Composable
        fun card(a: io.github.honeyslatt.music.core.model.Album, modifier: Modifier) {
            val actions = LocalActions.current
            CoverCard(a.artwork, a.title, a.artists.joinToString(", "), a.explicit, modifier = modifier, onMenu = { actions?.albumMenu(a, it) }) {
                keep(Searched.AlbumOpened(a))
                c.nav.push(Route.Album(a.id))
            }
        }
        if (all.open) {
            items(r.albums.chunked(2)) { pair ->
                Row(Modifier.padding(horizontal = 20.dp).padding(bottom = 20.dp), horizontalArrangement = Arrangement.spacedBy(14.dp)) {
                    pair.forEach { card(it, Modifier.weight(1f)) }
                    if (pair.size == 1) androidx.compose.foundation.layout.Spacer(Modifier.weight(1f))
                }
            }
        } else {
            item {
                LazyRow(contentPadding = PaddingValues(horizontal = 20.dp), horizontalArrangement = Arrangement.spacedBy(12.dp)) {
                    items(r.albums) { a -> card(a, Modifier.width(164.dp)) }
                }
            }
        }
    }
    if (r.artists.size > 1) {
        // Five after the top result; the title shows the others.
        val all = fold("Artists")
        item { SectionHeader("Artists", fold = all.takeIf { r.artists.size > 6 }) }
        val more = r.artists.drop(1).let { if (all.open) it else it.take(5) }
        items(more.size) { i ->
            ItemRow(more[i].artwork, more[i].name, "Artist", round = true) {
                keep(Searched.ArtistOpened(more[i]))
                c.nav.push(artist(more[i]))
            }
        }
    }
    if (r.playlists.isNotEmpty()) {
        item { SectionHeader("Playlists") }
        item {
            val actions = LocalActions.current
            LazyRow(contentPadding = PaddingValues(horizontal = 20.dp), horizontalArrangement = Arrangement.spacedBy(12.dp)) {
                items(r.playlists) { p ->
                    CoverCard(p.artwork, p.name, p.curator ?: "Playlist", modifier = Modifier.width(164.dp), onMenu = { actions?.playlistMenu(p, it) }) {
                        keep(Searched.PlaylistOpened(p))
                        c.nav.push(Route.Playlist(p.id, p))
                    }
                }
            }
        }
    }
}

/** A segmented control: the choice on a raised pill. */
@Composable
private fun Segmented(labels: List<String>, selected: Int, onSelect: (Int) -> Unit) {
    Row(
        Modifier
            .padding(start = 20.dp, end = 20.dp, top = 4.dp, bottom = 6.dp)
            .fillMaxWidth()
            .background(roles.surface, RoundedCornerShape(11.dp))
            .padding(3.dp),
    ) {
        labels.forEachIndexed { i, label ->
            val on = i == selected
            val lift by animateFloatAsState(if (on) 1f else 0f, label = "seg")
            val shape = RoundedCornerShape(8.dp)
            Box(
                Modifier
                    .weight(1f)
                    .height(32.dp)
                    .then(if (on) Modifier.shadow(2.dp, shape, ambientColor = Color.Black.copy(alpha = 0.12f), spotColor = Color.Black.copy(alpha = 0.12f)) else Modifier)
                    .background(roles.surface3.copy(alpha = lift), shape)
                    .press(shape) { onSelect(i) },
                contentAlignment = Alignment.Center,
            ) {
                BasicText(label, style = geist(14.sp, FontWeight.Medium).copy(color = if (on) roles.text else roles.sub))
            }
        }
    }
}
