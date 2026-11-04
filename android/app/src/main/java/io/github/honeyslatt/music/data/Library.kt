package io.github.honeyslatt.music.data

import io.github.honeyslatt.music.core.MusicException
import io.github.honeyslatt.music.core.keepAlbums
import io.github.honeyslatt.music.core.keepArtists
import io.github.honeyslatt.music.core.keepCollections
import io.github.honeyslatt.music.core.keepPlaylists
import io.github.honeyslatt.music.core.keepTracks
import io.github.honeyslatt.music.core.keptAlbums
import io.github.honeyslatt.music.core.keptArtists
import io.github.honeyslatt.music.core.keptCollections
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
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import kotlinx.coroutines.withTimeoutOrNull

/**
 * A list of the library, as it loads: done once every service asked for
 * it (`waiting`) has answered, not the first.
 */
data class Listing<T>(
    val items: List<T> = emptyList(),
    val done: Boolean = false,
    val error: String? = null,
    val waiting: Set<ProviderKind> = emptySet(),
) {
    /** `provider` asked for the list. */
    fun asked(provider: ProviderKind) = copy(waiting = waiting + provider, done = false)

    /** `provider` answered all of it, or as much as it will. */
    fun answered(provider: ProviderKind) = if (provider !in waiting) this else (waiting - provider).let { copy(waiting = it, done = it.isEmpty()) }
}

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

    /** Services a list of which couldn't load (the network gone, the phone asleep): loaded again once it is back. */
    private val unloaded = mutableSetOf<ProviderKind>()

    /** The network is back, or the app shows again: what couldn't load is loaded again. */
    fun retryFailed() {
        val again = unloaded.toList()
        if (again.isEmpty()) return
        unloaded.clear()
        clearErrors()
        again.filter { it in providers }.forEach {
            providers -= it
            load(it, again = true)
        }
    }

    /** What couldn't load, no longer said: it is tried again. */
    private fun clearErrors() {
        _lists.update {
            it.copy(
                playlists = it.playlists.copy(error = null),
                albums = it.albums.copy(error = null),
                artists = it.artists.copy(error = null),
                songs = it.songs.copy(error = null),
                recentlyAdded = it.recentlyAdded.copy(error = null),
                newestSongs = it.newestSongs.copy(error = null),
            )
        }
    }

    /** By file, whether what was kept of a list shows (once read). */
    private val shown = mutableMapOf<String, Deferred<Boolean>>()

    /** The lists (by file) the services have answered whole since the start. */
    private val loaded = mutableSetOf<String>()

    /** A service turned off or signed out of: its items go from the lists. */
    /** `provider`'s lists read again whole: the phone's files changed. */
    fun reload(provider: ProviderKind) {
        forget(provider)
        load(provider, again = true)
    }

    fun forget(provider: ProviderKind) {
        providers -= provider
        showing -= provider
        // Signed out or turned off: read whole when it is back.
        core.libraryRead(provider, 0)
        shown.keys.filter { it.endsWith("-${provider.name}.json") }.forEach { shown.remove(it)?.cancel() }
        loaded.removeAll { it.endsWith("-${provider.name}.json") }
        recentKept -= provider
        recentShown -= provider
        recentFresh -= provider
        _lists.update { l ->
            l.copy(
                playlists = l.playlists.copy(items = l.playlists.items.filter { it.id.provider != provider }).answered(provider),
                albums = l.albums.copy(items = l.albums.items.filter { it.id.provider != provider }).answered(provider),
                artists = l.artists.copy(items = l.artists.items.filter { it.id.provider != provider }).answered(provider),
                songs = l.songs.copy(items = l.songs.items.filter { it.id.provider != provider }).answered(provider),
                newestSongs = l.newestSongs.copy(items = l.newestSongs.items.filter { it.id.provider != provider }).answered(provider),
                recentlyAdded = l.recentlyAdded.copy(
                    items = l.recentlyAdded.items.filter {
                        when (it) {
                            is Collection.Album -> it.v1.id.provider
                            is Collection.Playlist -> it.v1.id.provider
                            is Collection.Station -> it.v1.id.provider
                        } != provider
                    },
                ).answered(provider),
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
        showKeptRecent(provider)
    }

    /** Load `provider`'s lists, adding to those of the others. */
    fun load(provider: ProviderKind, again: Boolean = false) {
        // Already, since the app started: the screen made anew (turned, or
        // gone dark at sunset) doesn't read the whole library again.
        if (provider in providers) return
        providers += provider
        showing += provider
        // Read whole within the hour (the app started again by the car, or
        // after Android ended it in the background): as kept, not read
        // again page by page; only what was added lately is asked.
        // On mobile data, read whole twice a day at most: what was added
        // lately comes all the same; on Wi-Fi, after an hour.
        val age = System.currentTimeMillis() - core.libraryReadAt(provider)
        val fresh = !again && age < if (core.metered) FRESH_ON_DATA else FRESH_FOR
        if (!fresh) core.libraryRead(provider, System.currentTimeMillis())
        if (newestAsked) newest(provider, ask = !fresh)
        lists(provider, ask = !fresh)
        if (fresh) recentlyAdded(provider)
    }

    private fun lists(provider: ProviderKind, ask: Boolean) {
        val session = core.session
        // Apple's lists come whole, many pages asked for at once.
        val apple = provider == ProviderKind.APPLE_MUSIC
        playlists(provider, ask)
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
        recentlyAdded(provider)
    }

    /** The services whose Recently Added was read from disk, asked to be (once), shown, and since answered. */
    private val recentKept = mutableSetOf<ProviderKind>()
    private val recentShown = mutableSetOf<ProviderKind>()
    private val recentFresh = mutableSetOf<ProviderKind>()

    private fun recentFile(provider: ProviderKind) = File(core.kept, "recently-added-${provider.name}.json").path

    /**
     * `provider`'s Recently Added as kept last time, at once, while it
     * connects: the service answers it only seconds after the app opens.
     */
    private fun showKeptRecent(provider: ProviderKind) {
        if (!recentKept.add(provider)) return
        scope.launch {
            val old = withContext(Dispatchers.IO) { keptCollections(recentFile(provider)) }
            if (old.isEmpty() || provider in recentFresh || provider !in showing) return@launch
            recentShown += provider
            _lists.update { l -> l.copy(recentlyAdded = l.recentlyAdded.copy(items = l.recentlyAdded.items.with(provider, old) { it.provider() })) }
        }
    }

    private fun playlists(provider: ProviderKind, ask: Boolean) = list(
        provider, ask, "playlists", ::keepPlaylists, ::keptPlaylists, { it.id.provider }, null,
        { core.session.libraryPlaylists(provider, it).let { p -> p.items to p.next } },
    ) { f -> _lists.update { it.copy(playlists = f(it.playlists)) } }

    /** `provider`'s playlists read again, one changed there (a new cover): Music's own as kept. */
    fun playlistsChanged(provider: ProviderKind) {
        if (provider == ProviderKind.MUSIC) own() else if (provider in providers) playlists(provider, ask = true)
    }

    private fun recentlyAdded(provider: ProviderKind) {
        val session = core.session
        showKeptRecent(provider)
        _lists.update { it.copy(recentlyAdded = it.recentlyAdded.asked(provider)) }
        // Apple's comes whole, in the order of what was added to last; the
        // others' by pages, up to three. Loaded again (the screen made anew),
        // they take the place of the last ones.
        scope.launch {
            val fresh = mutableListOf<Collection>()
            pages({ session.recentlyAdded(provider, it).let { p -> p.items to p.next } }, limit = 3) { items, done, error ->
                if (error != null) failed(provider)
                fresh += items
                // What was kept stays until the new list is all there.
                if (!done && provider in recentShown) return@pages
                recentFresh += provider
                if (done && error == null) fresh.toList().let { all -> scope.launch(Dispatchers.IO) { keepCollections(recentFile(provider), all) } }
                // Failed with what was kept shown: that stays.
                if (error != null && provider in recentShown && fresh.isEmpty()) {
                    _lists.update { l -> l.copy(recentlyAdded = l.recentlyAdded.copy(error = error).answered(provider)) }
                    return@pages
                }
                _lists.update { l ->
                    l.copy(
                        recentlyAdded = l.recentlyAdded.copy(
                            items = l.recentlyAdded.items.with(provider, fresh.toList()) { it.provider() },
                            error = error ?: l.recentlyAdded.error,
                        ).let { if (done) it.answered(provider) else it },
                    )
                }
            }
        }
    }

    /**
     * Something of `provider`'s just joined the library, or left it: its
     * Recently Added read again, a moment later for Apple's, whose newest
     * songs say so only then.
     */
    fun recentChanged(provider: ProviderKind) {
        if (provider !in providers) return
        scope.launch {
            if (provider == ProviderKind.APPLE_MUSIC) kotlinx.coroutines.delay(1_500)
            recentlyAdded(provider)
        }
    }

    /**
     * Pulled down: every service's lists read again, what shows staying until
     * the new ones are there; back once `of` them is, or after 30 s at most.
     */
    suspend fun refresh(of: (LibraryLists) -> Listing<*>) {
        unloaded.clear()
        clearErrors()
        providers.forEach {
            core.libraryRead(it, System.currentTimeMillis())
            if (newestAsked) newest(it, ask = true)
            lists(it, ask = true)
        }
        own()
        withTimeoutOrNull(30_000) { lists.first { of(it).waiting.isEmpty() } }
    }

    /** Not read whole: tried again with the network, and next time anyway. */
    private fun failed(provider: ProviderKind) {
        unloaded += provider
        core.libraryRead(provider, 0)
    }

    /** A playlist just made: first in the list, as the newest (once, Music's own being read again as it is made). */
    fun playlistMade(playlist: Playlist) {
        _lists.update { it.copy(playlists = it.playlists.copy(items = listOf(playlist) + it.playlists.items.filter { p -> p.id != playlist.id })) }
    }

    /** Music's own playlists, as kept with the other devices now. */
    fun own() {
        scope.launch {
            val own = runCatching { core.session.libraryPlaylists(ProviderKind.MUSIC, null).items }.getOrNull() ?: return@launch
            _lists.update { it.copy(playlists = it.playlists.copy(items = it.playlists.items.with(ProviderKind.MUSIC, own) { p -> p.id.provider })) }
        }
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

    fun albumDeleted(id: io.github.honeyslatt.music.core.model.ItemId) {
        _lists.update { l ->
            l.copy(
                albums = l.albums.copy(items = l.albums.items.filter { it.id != id }),
                recentlyAdded = l.recentlyAdded.copy(items = l.recentlyAdded.items.filter { !(it is Collection.Album && it.v1.id == id) }),
            )
        }
    }

    /** An album deleted, back: the service refused. */
    fun albumBack(album: Album) {
        _lists.update { l -> l.copy(albums = l.albums.copy(items = listOf(album) + l.albums.items.filter { it.id != album.id })) }
        recentChanged(album.id.provider)
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
        if (ask) set { it.asked(provider) }
        scope.launch {
            // Shown once, whether the service answers yet or not.
            val shownKept = shown.getOrPut(file) {
                scope.async {
                    val old = withContext(Dispatchers.IO) { kept(file) }
                    if (old.isNotEmpty()) set { it.copy(items = it.items.with(provider, old, of), done = it.waiting.isEmpty()) }
                    old.isNotEmpty()
                }
            }.await()
            if (!ask) return@launch
            // The first time with nothing kept, the pages show as they come;
            // after, the list shown stays until the new one is all there.
            val stepwise = !shownKept && file !in loaded
            val fresh = mutableListOf<T>()
            var failed: String? = null
            if (whole != null) {
                try {
                    fresh += whole()
                } catch (e: MusicException) {
                    failed = e.message ?: e.toString()
                }
                if (stepwise) set { l -> l.copy(items = if (failed == null) l.items.with(provider, fresh, of) else l.items, error = failed ?: l.error).answered(provider) }
            } else {
                pages(page) { items, done, error ->
                    fresh += items
                    failed = error
                    if (stepwise) set { l -> l.copy(items = l.items.with(provider, fresh.toList(), of), error = error ?: l.error).let { if (done) it.answered(provider) else it } }
                }
            }
            if (failed != null) {
                failed(provider)
                if (!stepwise) set { it.answered(provider) }
                return@launch
            }
            if (!stepwise) set { l -> l.copy(items = l.items.with(provider, fresh, of)).answered(provider) }
            loaded += file
            withContext(Dispatchers.IO) { keep(file, fresh) }
        }
    }

    /**
     * The list with `provider`'s items now `group`: each service's together,
     * in the same order every time (Music's own, Apple Music, SoundCloud,
     * Navidrome), so that a list read again doesn't move.
     */
    private fun <T> List<T>.with(provider: ProviderKind, group: List<T>, of: (T) -> ProviderKind): List<T> {
        val rank = { p: ProviderKind -> if (p == ProviderKind.MUSIC) -1 else p.ordinal }
        val rest = filter { of(it) != provider }
        val at = rest.indexOfFirst { rank(of(it)) > rank(provider) }.takeIf { it >= 0 } ?: rest.size
        return rest.take(at) + group + rest.drop(at)
    }

    private fun Collection.provider() = when (this) {
        is Collection.Album -> v1.id.provider
        is Collection.Playlist -> v1.id.provider
        is Collection.Station -> v1.id.provider
    }

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

/** How long a library read whole is taken as it was kept, the app started again. */
private const val FRESH_FOR = 60 * 60_000L

/** The same on mobile data. */
private const val FRESH_ON_DATA = 12 * 60 * 60_000L
