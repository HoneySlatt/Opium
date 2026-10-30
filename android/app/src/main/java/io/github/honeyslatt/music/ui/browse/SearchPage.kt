package io.github.honeyslatt.music.ui.browse

import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
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
import io.github.honeyslatt.music.core.SearchScope
import io.github.honeyslatt.music.core.songsLeftOut
import io.github.honeyslatt.music.data.LibraryLists
import io.github.honeyslatt.music.core.model.Artist
import io.github.honeyslatt.music.core.model.ProviderKind
import io.github.honeyslatt.music.core.model.SearchResults
import io.github.honeyslatt.music.ui.Icons
import io.github.honeyslatt.music.ui.actions.LocalActions
import io.github.honeyslatt.music.ui.components.Fold
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

/** How many searches Recently Searched keeps. */
private const val RECENT = 10

/** How many songs a search shows before its title is tapped. */
private const val SONGS = 6

/**
 * Search: the field, `Apple Music | Your Library` (remembered), and what
 * matches: the top result, songs, albums, artists, playlists. Before a
 * word is typed, the searches made lately.
 */
@Composable
fun SearchPage(c: LibraryContext) {
    var query by rememberSaveable { mutableStateOf("") }
    var library by remember { mutableStateOf(c.core.setting("search-scope") == "library") }
    var source by rememberSaveable { mutableStateOf<ProviderKind?>(null) }
    var sources by rememberSaveable { mutableStateOf(false) }
    var allSongs by remember { mutableStateOf(false) }
    var recent by remember { mutableStateOf(c.core.setting("recent-searches")?.split('\n')?.filter { it.isNotBlank() }.orEmpty()) }
    val words = query.trim()
    // Each search starts with its first songs only.
    androidx.compose.runtime.LaunchedEffect(words, library, source) { allSongs = false }
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
    // A result opened: its search is worth keeping.
    val keep = {
        recent = (listOf(words) + recent.filter { !it.equals(words, ignoreCase = true) }).take(RECENT)
        c.core.setSetting("recent-searches", recent.joinToString("\n"))
    }
    // The title folds the source chips.
    ListPage(c.nav, "Search", fold = Fold(sources) { sources = !sources }) {
        item { SearchReveal(true, query, if (library) "Your Library" else "Artists, Songs, Lyrics and More", focusOnOpen = false) { query = it } }
        item {
            Segmented(listOf("Apple Music", "Your Library"), if (library) 1 else 0) {
                library = it == 1
                c.core.setSetting("search-scope", if (library) "library" else "catalog")
            }
        }
        item { SourceChips(sources, source, c.services) { source = it } }
        val r = results
        when {
            r == null -> recentSearches(recent, onPick = { query = it }) {
                recent = emptyList()
                c.core.setSetting("recent-searches", "")
            }
            r !is Loading.Ready -> item { Empty(r.listing(), "", "") }
            else -> results(c, r.value, library, words, Fold(allSongs) { allSongs = !allSongs }, keep)
        }
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

private fun androidx.compose.foundation.lazy.LazyListScope.recentSearches(recent: List<String>, onPick: (String) -> Unit, onClear: () -> Unit) {
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
    items(recent.size) { i ->
        Row(
            Modifier.fillMaxWidth().height(48.dp).press { onPick(recent[i]) }.padding(horizontal = 20.dp),
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.spacedBy(14.dp),
        ) {
            Glyph(Icons.Search, roles.muted, 18.dp)
            Line(recent[i], geist(16.sp).copy(color = roles.text))
        }
    }
}

private fun androidx.compose.foundation.lazy.LazyListScope.results(
    c: LibraryContext,
    r: SearchResults,
    library: Boolean,
    words: String,
    allSongs: Fold,
    keep: () -> Unit,
) {
    if (r.tracks.isEmpty() && r.albums.isEmpty() && r.artists.isEmpty() && r.playlists.isEmpty()) {
        item {
            Empty(
                io.github.honeyslatt.music.data.Listing<Unit>(done = true),
                "No results",
                "Nothing ${if (library) "in your library" else "on Apple Music"} matches “$words”.",
            )
        }
        return
    }
    fun artist(a: Artist) = if (library) Route.LibraryArtist(a) else Route.Artist(a.id)
    r.artists.firstOrNull()?.let { top ->
        item { SectionHeader("Top Result") }
        item {
            ItemRow(top.artwork, top.name, "Artist", round = true) {
                keep()
                c.nav.push(artist(top))
            }
        }
    }
    if (r.tracks.isNotEmpty()) {
        // The first six; the title shows the others, when there are.
        item { SectionHeader("Songs", fold = allSongs.takeIf { r.tracks.size > SONGS }) }
        val songs = if (allSongs.open) r.tracks else r.tracks.take(SONGS)
        items(songs.size) { i ->
            TrackRow(songs[i]) {
                keep()
                c.playback.play(songs, i)
            }
        }
    }
    if (r.albums.isNotEmpty()) {
        item { SectionHeader("Albums") }
        item {
            val actions = LocalActions.current
            LazyRow(contentPadding = PaddingValues(horizontal = 20.dp), horizontalArrangement = Arrangement.spacedBy(12.dp)) {
                items(r.albums) { a ->
                    CoverCard(a.artwork, a.title, a.artists.joinToString(", "), a.explicit, modifier = Modifier.width(164.dp), onMenu = { actions?.albumMenu(a, it) }) {
                        keep()
                        c.nav.push(Route.Album(a.id))
                    }
                }
            }
        }
    }
    if (r.artists.size > 1) {
        item { SectionHeader("Artists") }
        val more = r.artists.drop(1).take(5)
        items(more.size) { i ->
            ItemRow(more[i].artwork, more[i].name, "Artist", round = true) {
                keep()
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
                        keep()
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
