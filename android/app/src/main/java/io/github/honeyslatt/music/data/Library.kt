package io.github.honeyslatt.music.data

import io.github.honeyslatt.music.core.MusicException
import io.github.honeyslatt.music.core.model.Album
import io.github.honeyslatt.music.core.model.Artist
import io.github.honeyslatt.music.core.model.Collection
import io.github.honeyslatt.music.core.model.Playlist
import io.github.honeyslatt.music.core.model.ProviderKind
import io.github.honeyslatt.music.core.model.Track
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch

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

    private val providers = mutableListOf<ProviderKind>()
    private var newestAsked = false

    /** Load the songs newest first, of every service loaded. */
    fun loadNewestSongs() {
        if (newestAsked) return
        newestAsked = true
        providers.toList().forEach(::loadNewest)
    }

    private fun loadNewest(provider: ProviderKind) {
        scope.launch {
            pages({ core.session.libraryTracksNewest(provider, it).let { p -> p.items to p.next } }) { items, done, error ->
                _lists.update { it.copy(newestSongs = it.newestSongs.more(items, done, error)) }
            }
        }
    }

    /** Load `provider`'s lists, adding to those of the others. */
    fun load(provider: ProviderKind) {
        providers += provider
        if (newestAsked) loadNewest(provider)
        val session = core.session
        scope.launch {
            pages({ session.libraryPlaylists(provider, it).let { p -> p.items to p.next } }) { items, done, error ->
                _lists.update { it.copy(playlists = it.playlists.more(items, done, error)) }
            }
        }
        scope.launch {
            pages({ session.libraryAlbums(provider, it).let { p -> p.items to p.next } }) { items, done, error ->
                _lists.update { it.copy(albums = it.albums.more(items, done, error)) }
            }
        }
        scope.launch {
            pages({ session.libraryArtists(provider, it).let { p -> p.items to p.next } }) { items, done, error ->
                _lists.update { it.copy(artists = it.artists.more(items, done, error)) }
            }
        }
        scope.launch {
            pages({ session.libraryTracks(provider, it).let { p -> p.items to p.next } }) { items, done, error ->
                _lists.update { it.copy(songs = it.songs.more(items, done, error)) }
            }
        }
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
