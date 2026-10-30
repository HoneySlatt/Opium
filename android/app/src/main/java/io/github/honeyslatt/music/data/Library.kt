package io.github.honeyslatt.music.data

import io.github.honeyslatt.music.core.MusicException
import io.github.honeyslatt.music.core.keepAlbums
import io.github.honeyslatt.music.core.keepArtists
import io.github.honeyslatt.music.core.keepPlaylists
import io.github.honeyslatt.music.core.keepTracks
import io.github.honeyslatt.music.core.keptAlbums
import io.github.honeyslatt.music.core.keptArtists
import io.github.honeyslatt.music.core.keptPlaylists
import io.github.honeyslatt.music.core.keptTracks
import io.github.honeyslatt.music.core.model.Album
import io.github.honeyslatt.music.core.model.Artist
import io.github.honeyslatt.music.core.model.Collection
import io.github.honeyslatt.music.core.model.Playlist
import io.github.honeyslatt.music.core.model.ProviderKind
import io.github.honeyslatt.music.core.model.Track
import java.io.File
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Deferred
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.async
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext

/** A list of the library, as it loads. */
data class Listing<T>(val items: List<T> = emptyList(), val done: Boolean = false, val error: String? = null)

data class LibraryLists(
    val playlists: Listing<Playlist> = Listing(),
    val albums: Listing<Album> = Listing(),
    val artists: Listing<Artist> = Listing(),
    val songs: Listing<Track> = Listing(),
    val recentlyAdded: Listing<Collection> = Listing(),
    /** The songs, the latest added first: loaded once Songs is sorted so. */
    val newestSongs: Listing<Track> = Listing(),
)

/** The library of every service connected, each list filled page by page. */
class Library(private val core: Core, private val scope: CoroutineScope) {
    private val _lists = MutableStateFlow(LibraryLists())
    val lists: StateFlow<LibraryLists> = _lists.asStateFlow()

    private val providers = mutableSetOf<ProviderKind>()

    /** By file, whether what was kept of a list shows (once read). */
    private val shown = mutableMapOf<String, Deferred<Boolean>>()

    /** A service turned off or signed out of: its items go from the lists. */
    fun forget(provider: ProviderKind) {
        providers -= provider
        showing -= provider
        shown.keys.filter { it.endsWith("-${provider.name}.json") }.forEach { shown.remove(it)?.cancel() }
        _lists.update { l ->
            l.copy(
                playlists = l.playlists.copy(items = l.playlists.items.filter { it.id.provider != provider }),
                albums = l.albums.copy(items = l.albums.items.filter { it.id.provider != provider }),
                artists = l.artists.copy(items = l.artists.items.filter { it.id.provider != provider }),
                songs = l.songs.copy(items = l.songs.items.filter { it.id.provider != provider }),
                newestSongs = l.newestSongs.copy(items = l.newestSongs.items.filter { it.id.provider != provider }),
                recentlyAdded = l.recentlyAdded.copy(
                    items = l.recentlyAdded.items.filter {
                        when (it) {
                            is Collection.Album -> it.v1.id.provider
                            is Collection.Playlist -> it.v1.id.provider
                            is Collection.Station -> it.v1.id.provider
                        } != provider
                    },
                ),
            )
        }
    }
    private var newestAsked = false

    /** The services whose kept lists show, though they may not answer yet. */
    private val showing = mutableSetOf<ProviderKind>()

    /** Load the songs newest first, of every service shown. */
    fun loadNewestSongs() {
        if (newestAsked) return
        newestAsked = true
        showing.forEach { newest(it, ask = it in providers) }
    }

    private fun newest(provider: ProviderKind, ask: Boolean) {
        val session = core.session
        list(
            provider, ask, "newest", ::keepTracks, ::keptTracks, { it.id.provider },
            if (provider == ProviderKind.APPLE_MUSIC) session::appleLibraryTracksNewest else null,
            { session.libraryTracksNewest(provider, it).let { p -> p.items to p.next } },
        ) { f -> _lists.update { it.copy(newestSongs = f(it.newestSongs)) } }
    }

    /** `provider`'s lists as kept last time, while it connects or can't. */
    fun showKept(provider: ProviderKind) {
        showing += provider
        if (newestAsked) newest(provider, ask = false)
        lists(provider, ask = false)
    }

    /** Load `provider`'s lists, adding to those of the others. */
    fun load(provider: ProviderKind) {
        providers += provider
        showing += provider
        if (newestAsked) newest(provider, ask = true)
        lists(provider, ask = true)
    }

    private fun lists(provider: ProviderKind, ask: Boolean) {
        val session = core.session
        // Apple's lists come whole, many pages asked for at once.
        val apple = provider == ProviderKind.APPLE_MUSIC
        list(
            provider, ask, "playlists", ::keepPlaylists, ::keptPlaylists, { it.id.provider }, null,
            { session.libraryPlaylists(provider, it).let { p -> p.items to p.next } },
        ) { f -> _lists.update { it.copy(playlists = f(it.playlists)) } }
        list(
            provider, ask, "albums", ::keepAlbums, ::keptAlbums, { it.id.provider },
            if (apple) session::appleLibraryAlbums else null,
            { session.libraryAlbums(provider, it).let { p -> p.items to p.next } },
        ) { f -> _lists.update { it.copy(albums = f(it.albums)) } }
        list(
            provider, ask, "artists", ::keepArtists, ::keptArtists, { it.id.provider },
            if (apple) session::appleLibraryArtists else null,
            { session.libraryArtists(provider, it).let { p -> p.items to p.next } },
        ) { f -> _lists.update { it.copy(artists = f(it.artists)) } }
        list(
            provider, ask, "songs", ::keepTracks, ::keptTracks, { it.id.provider },
            if (apple) session::appleLibraryTracks else null,
            { session.libraryTracks(provider, it).let { p -> p.items to p.next } },
        ) { f -> _lists.update { it.copy(songs = f(it.songs)) } }
        if (!ask) return
        // Apple gives 25 at a time: three pages fill Library's 66.
        scope.launch {
            pages({ session.recentlyAdded(provider, it).let { p -> p.items to p.next } }, limit = 3) { items, done, error ->
                _lists.update { it.copy(recentlyAdded = it.recentlyAdded.more(items, done, error)) }
            }
        }
    }

    /** A playlist just made: first in the list, as the newest. */
    fun playlistMade(playlist: Playlist) {
        _lists.update { it.copy(playlists = it.playlists.copy(items = listOf(playlist) + it.playlists.items)) }
    }

    fun playlistEdited(id: io.github.honeyslatt.music.core.model.ItemId, name: String, description: String) {
        _lists.update { l ->
            l.copy(
                playlists = l.playlists.copy(
                    items = l.playlists.items.map { if (it.id == id) it.copy(name = name, description = description.ifEmpty { null }) else it },
                ),
            )
        }
    }

    fun playlistDeleted(id: io.github.honeyslatt.music.core.model.ItemId) {
        _lists.update { l -> l.copy(playlists = l.playlists.copy(items = l.playlists.items.filter { it.id != id })) }
    }

    /**
     * `provider`'s part of a list: as it was kept last time, at once, then as
     * the service has it now, kept for next time. What was kept stays until
     * the new list is all there; with nothing kept, its pages show as they
     * come. `whole` gets the list in one go, else it comes by `page`. Only
     * what was kept shows unless `ask`: the service isn't there yet.
     */
    private fun <T> list(
        provider: ProviderKind,
        ask: Boolean,
        name: String,
        keep: (String, List<T>) -> Unit,
        kept: (String) -> List<T>,
        of: (T) -> ProviderKind,
        whole: (suspend () -> List<T>)?,
        page: suspend (String?) -> Pair<List<T>, String?>,
        set: ((Listing<T>) -> Listing<T>) -> Unit,
    ) {
        val file = File(core.kept, "$name-${provider.name}.json").path
        scope.launch {
            // Shown once, whether the service answers yet or not.
            val shownKept = shown.getOrPut(file) {
                scope.async {
                    val old = withContext(Dispatchers.IO) { kept(file) }
                    if (old.isNotEmpty()) set { it.copy(items = it.items.with(provider, old, of), done = true) }
                    old.isNotEmpty()
                }
            }.await()
            if (!ask) return@launch
            val fresh = mutableListOf<T>()
            var failed: String? = null
            if (whole != null) {
                try {
                    fresh += whole()
                } catch (e: MusicException) {
                    failed = e.message ?: e.toString()
                }
                if (!shownKept) set { l -> l.copy(items = l.items.with(provider, fresh, of), done = true, error = failed ?: l.error) }
            } else {
                pages(page) { items, done, error ->
                    fresh += items
                    failed = error
                    if (!shownKept) {
                        set { l -> l.copy(items = l.items.with(provider, l.items.filter { of(it) == provider } + items, of), done = done, error = error ?: l.error) }
                    }
                }
            }
            if (failed != null) return@launch
            if (shownKept) set { l -> l.copy(items = l.items.with(provider, fresh, of), done = true) }
            withContext(Dispatchers.IO) { keep(file, fresh) }
        }
    }

    /**
     * The list with `provider`'s items now `group`: each service's together,
     * in the same order every time (Apple Music, SoundCloud, Navidrome), so
     * that a list read again doesn't move.
     */
    private fun <T> List<T>.with(provider: ProviderKind, group: List<T>, of: (T) -> ProviderKind): List<T> {
        val rest = filter { of(it) != provider }
        val at = rest.indexOfFirst { of(it).ordinal > provider.ordinal }.takeIf { it >= 0 } ?: rest.size
        return rest.take(at) + group + rest.drop(at)
    }

    private fun <T> Listing<T>.more(items: List<T>, done: Boolean, error: String?) =
        copy(items = this.items + items, done = done, error = error ?: this.error)

    /** Every page from the first, each handed over as it comes. */
    private suspend fun <T> pages(
        page: suspend (String?) -> Pair<List<T>, String?>,
        limit: Int = Int.MAX_VALUE,
        each: (List<T>, Boolean, String?) -> Unit,
    ) {
        var cursor: String? = null
        var count = 0
        do {
            val (items, next) = try {
                page(cursor)
            } catch (e: MusicException) {
                each(emptyList(), true, e.message)
                return
            }
            cursor = next
            count++
            each(items, cursor == null || count >= limit, null)
        } while (cursor != null && count < limit)
    }
}
