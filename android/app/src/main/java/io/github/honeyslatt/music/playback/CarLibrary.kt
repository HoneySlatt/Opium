package io.github.honeyslatt.music.playback

import android.net.Uri
import android.os.Bundle
import androidx.media3.common.MediaItem
import androidx.media3.common.MediaMetadata
import androidx.media3.common.util.UnstableApi
import androidx.media3.session.CommandButton
import androidx.media3.session.LibraryResult
import androidx.media3.session.MediaConstants
import androidx.media3.session.MediaLibraryService.LibraryParams
import androidx.media3.session.MediaLibraryService.MediaLibrarySession
import androidx.media3.session.MediaSession
import androidx.media3.session.SessionCommand
import androidx.media3.session.SessionResult
import com.google.common.collect.ImmutableList
import com.google.common.util.concurrent.Futures
import com.google.common.util.concurrent.ListenableFuture
import com.google.common.util.concurrent.SettableFuture
import io.github.honeyslatt.music.MusicApp
import io.github.honeyslatt.music.R
import io.github.honeyslatt.music.core.QueueRepeat
import io.github.honeyslatt.music.core.SearchScope
import io.github.honeyslatt.music.core.model.Album
import io.github.honeyslatt.music.core.model.Artist
import io.github.honeyslatt.music.core.model.Artwork
import io.github.honeyslatt.music.core.model.Collection
import io.github.honeyslatt.music.core.model.FavouriteKind
import io.github.honeyslatt.music.core.model.ItemId
import io.github.honeyslatt.music.core.model.Playlist
import io.github.honeyslatt.music.core.model.ProviderKind
import io.github.honeyslatt.music.core.model.Shelf
import io.github.honeyslatt.music.core.model.Station
import io.github.honeyslatt.music.core.model.Track
import io.github.honeyslatt.music.data.Connection
import io.github.honeyslatt.music.data.Listing
import io.github.honeyslatt.music.ui.browse.Browse
import io.github.honeyslatt.music.ui.components.at
import io.github.honeyslatt.music.ui.detail.Loading
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.launch
import kotlinx.coroutines.withTimeoutOrNull

/**
 * The app's music as Android Auto shows it, as Apple Music's is there:
 * Home, Radio and Library as tabs, and what is picked plays through the
 * app's queue. Favourite, shuffle and repeat show by the song while the
 * car is connected.
 */
@UnstableApi
class CarLibrary(private val app: MusicApp, private val scope: CoroutineScope) : MediaLibrarySession.Callback {
    private val session get() = app.core.session
    private val playback get() = app.playback
    private val browse = Browse(app.core, scope)

    /** What the ids handed out stand for, the latest few thousand. */
    private sealed interface Node {
        data class OfAlbum(val album: Album) : Node
        data class OfPlaylist(val playlist: Playlist) : Node
        data class OfArtist(val artist: Artist) : Node
        data class OfStation(val station: Station) : Node
        data class Song(val list: List<Track>, val index: Int) : Node
    }

    private val nodes = object : LinkedHashMap<String, Node>(256, 0.75f, true) {
        override fun removeEldestEntry(eldest: MutableMap.MutableEntry<String, Node>?) = size > 5_000
    }
    private var issued = 0L

    private fun id(node: Node): String = "n${issued++}".also { nodes[it] = node }

    // ---- Browsing ----

    override fun onGetLibraryRoot(session: MediaLibrarySession, browser: MediaSession.ControllerInfo, params: LibraryParams?): ListenableFuture<LibraryResult<MediaItem>> {
        app.start()
        val extras = Bundle().apply {
            putInt(MediaConstants.EXTRAS_KEY_CONTENT_STYLE_BROWSABLE, MediaConstants.EXTRAS_VALUE_CONTENT_STYLE_LIST_ITEM)
            putInt(MediaConstants.EXTRAS_KEY_CONTENT_STYLE_PLAYABLE, MediaConstants.EXTRAS_VALUE_CONTENT_STYLE_LIST_ITEM)
        }
        val root = folder(ROOT, "Music", style = MediaConstants.EXTRAS_VALUE_CONTENT_STYLE_LIST_ITEM)
        return Futures.immediateFuture(LibraryResult.ofItem(root, LibraryParams.Builder().setExtras(extras).build()))
    }

    override fun onGetChildren(
        session: MediaLibrarySession,
        browser: MediaSession.ControllerInfo,
        parentId: String,
        page: Int,
        pageSize: Int,
        params: LibraryParams?,
    ): ListenableFuture<LibraryResult<ImmutableList<MediaItem>>> = later {
        app.start()
        // Out of reach (no network in a tunnel): said as such, for the car to show.
        val all = runCatching { children(parentId) }.getOrElse { return@later LibraryResult.ofError(LibraryResult.RESULT_ERROR_IO) }
        val from = (page.toLong() * pageSize).coerceIn(0, all.size.toLong()).toInt()
        val to = (from.toLong() + pageSize).coerceAtMost(all.size.toLong()).toInt()
        LibraryResult.ofItemList(ImmutableList.copyOf(all.subList(from, to)), params)
    }

    override fun onGetItem(session: MediaLibrarySession, browser: MediaSession.ControllerInfo, mediaId: String): ListenableFuture<LibraryResult<MediaItem>> =
        Futures.immediateFuture(LibraryResult.ofItem(MediaItem.Builder().setMediaId(mediaId).build(), null))

    private suspend fun children(parent: String): List<MediaItem> = when (parent) {
        ROOT -> listOf(
            tab(HOME, "Home", R.drawable.car_home, MediaConstants.EXTRAS_VALUE_CONTENT_STYLE_LIST_ITEM),
            tab(RADIO, "Radio", R.drawable.car_radio, MediaConstants.EXTRAS_VALUE_CONTENT_STYLE_GRID_ITEM),
            tab(LIBRARY, "Library", R.drawable.car_library, MediaConstants.EXTRAS_VALUE_CONTENT_STYLE_LIST_ITEM),
        )
        HOME -> shelves(Browse.Page.Home).mapIndexedNotNull { i, shelf ->
            if (shelf.items.isEmpty() && shelf.songs.isEmpty()) null
            else folder("$HOME/$i", shelf.title, style = if (shelf.items.isEmpty()) MediaConstants.EXTRAS_VALUE_CONTENT_STYLE_LIST_ITEM else MediaConstants.EXTRAS_VALUE_CONTENT_STYLE_GRID_ITEM)
        }
        // Each shelf under its title, its stations in a grid.
        RADIO -> shelves(Browse.Page.Radio).flatMap { shelf ->
            (shelf.items.map(::collection) + songs(shelf.songs)).map { it.inGroup(shelf.title) }
        }
        LIBRARY -> listOf(
            playable(SHUFFLE, "Shuffle Library", null, null),
            folder(RECENT, "Recently Added", style = MediaConstants.EXTRAS_VALUE_CONTENT_STYLE_GRID_ITEM),
            folder(PLAYLISTS, "Playlists", style = MediaConstants.EXTRAS_VALUE_CONTENT_STYLE_LIST_ITEM),
            folder(ARTISTS, "Artists", style = MediaConstants.EXTRAS_VALUE_CONTENT_STYLE_LIST_ITEM),
            folder(ALBUMS, "Albums", style = MediaConstants.EXTRAS_VALUE_CONTENT_STYLE_GRID_ITEM),
            folder(SONGS, "Songs", style = MediaConstants.EXTRAS_VALUE_CONTENT_STYLE_LIST_ITEM),
        )
        RECENT -> listing { it.recentlyAdded }.map(::collection)
        PLAYLISTS -> listing { it.playlists }.distinctBy { it.id }.sortedWith(compareByDescending<Playlist> { it.favourites }.thenBy { it.name.lowercase() }).map { collection(Collection.Playlist(it)) }
        ARTISTS -> listing { it.artists }.distinctBy { it.id }.sortedBy { it.name.lowercase() }.map { artist ->
            folder(id(Node.OfArtist(artist)), artist.name, art = artist.artwork, style = MediaConstants.EXTRAS_VALUE_CONTENT_STYLE_GRID_ITEM)
        }
        ALBUMS -> listing { it.albums }.distinctBy { it.id }.sortedBy { it.title.lowercase() }.map { collection(Collection.Album(it)) }
        SONGS -> songs(allSongs())
        else -> when {
            parent.startsWith("$HOME/") -> {
                val shelf = shelves(Browse.Page.Home).getOrNull(parent.removePrefix("$HOME/").toIntOrNull() ?: -1)
                shelf?.let { it.items.map(::collection) + songs(it.songs) }.orEmpty()
            }
            else -> when (val node = nodes[parent]) {
                is Node.OfAlbum -> songs(session.album(node.album.id).tracks)
                is Node.OfPlaylist -> songs(playlistTracks(node.playlist.id))
                is Node.OfArtist -> session.libraryArtistAlbums(node.artist.id).distinctBy { it.id }.map { collection(Collection.Album(it)) }
                else -> emptyList()
            }
        }
    }

    /** A page of Apple's, once connected and read; none if it can't be. */
    private suspend fun shelves(page: Browse.Page): List<Shelf> {
        withTimeoutOrNull(15_000) { app.core.apple.first { it is Connection.Connected || it is Connection.Failed || it == Connection.Off } }
        val got = withTimeoutOrNull(20_000) { browse.shelves(page).first { it !is Loading.Pending } }
        return (got as? Loading.Ready)?.value.orEmpty()
    }

    /** One of the library's lists, waiting a while for it to come in. */
    private suspend fun <T> listing(of: (io.github.honeyslatt.music.data.LibraryLists) -> Listing<T>): List<T> {
        val lists = app.library.lists
        withTimeoutOrNull(10_000) { lists.first { of(it).items.isNotEmpty() || of(it).done } }
        return of(lists.value).items
    }

    /** The library's songs, by title, each once: as Shuffle Library and Songs play them. */
    private suspend fun allSongs(): List<Track> = listing { it.songs }.distinctBy { it.id }.sortedBy { it.title.lowercase() }

    private suspend fun playlistTracks(id: ItemId): List<Track> {
        val all = mutableListOf<Track>()
        var cursor: String? = null
        do {
            val page = session.playlistTracks(id, cursor)
            all += page.items
            cursor = page.next
        } while (cursor != null)
        return all
    }

    private fun songs(tracks: List<Track>): List<MediaItem> = tracks.mapIndexed { i, t ->
        playable(id(Node.Song(tracks, i)), t.title, t.artists.joinToString(", "), t.artwork, album = t.album)
    }

    private fun collection(c: Collection): MediaItem = when (c) {
        is Collection.Album -> folder(id(Node.OfAlbum(c.v1)), c.v1.title, c.v1.artists.joinToString(", "), c.v1.artwork, MediaConstants.EXTRAS_VALUE_CONTENT_STYLE_LIST_ITEM, MediaMetadata.MEDIA_TYPE_ALBUM)
        is Collection.Playlist -> folder(id(Node.OfPlaylist(c.v1)), c.v1.name, c.v1.curator, c.v1.artwork, MediaConstants.EXTRAS_VALUE_CONTENT_STYLE_LIST_ITEM, MediaMetadata.MEDIA_TYPE_PLAYLIST)
        is Collection.Station -> playable(id(Node.OfStation(c.v1)), c.v1.name, c.v1.description, c.v1.artwork, type = MediaMetadata.MEDIA_TYPE_RADIO_STATION)
    }

    private fun tab(id: String, title: String, icon: Int, style: Int) = MediaItem.Builder()
        .setMediaId(id)
        .setMediaMetadata(
            MediaMetadata.Builder()
                .setTitle(title)
                .setIsBrowsable(true)
                .setIsPlayable(false)
                .setMediaType(MediaMetadata.MEDIA_TYPE_FOLDER_MIXED)
                .setArtworkUri(Uri.parse("android.resource://${app.packageName}/$icon"))
                .setExtras(childStyle(style))
                .build(),
        )
        .build()

    /** A folder; `style`: how what it holds shows, in a list or a grid. */
    private fun folder(
        id: String,
        title: String,
        subtitle: String? = null,
        art: Artwork? = null,
        style: Int = MediaConstants.EXTRAS_VALUE_CONTENT_STYLE_LIST_ITEM,
        type: Int = MediaMetadata.MEDIA_TYPE_FOLDER_MIXED,
    ) = MediaItem.Builder()
        .setMediaId(id)
        .setMediaMetadata(
            MediaMetadata.Builder()
                .setTitle(title)
                .setSubtitle(subtitle)
                .setArtist(subtitle)
                .setArtworkUri(art?.let { ArtProvider.uri(app, it.at(400)) })
                .setIsBrowsable(true)
                .setIsPlayable(false)
                .setMediaType(type)
                .setExtras(childStyle(style))
                .build(),
        )
        .build()

    private fun playable(id: String, title: String, subtitle: String?, art: Artwork?, album: String? = null, type: Int = MediaMetadata.MEDIA_TYPE_MUSIC) = MediaItem.Builder()
        .setMediaId(id)
        .setMediaMetadata(
            MediaMetadata.Builder()
                .setTitle(title)
                .setSubtitle(subtitle)
                .setArtist(subtitle)
                .setAlbumTitle(album)
                .setArtworkUri(art?.let { ArtProvider.uri(app, it.at(400)) })
                .setIsBrowsable(false)
                .setIsPlayable(true)
                .setMediaType(type)
                .build(),
        )
        .build()

    private fun childStyle(style: Int) = Bundle().apply {
        putInt(MediaConstants.EXTRAS_KEY_CONTENT_STYLE_BROWSABLE, style)
        putInt(MediaConstants.EXTRAS_KEY_CONTENT_STYLE_PLAYABLE, if (style == MediaConstants.EXTRAS_VALUE_CONTENT_STYLE_GRID_ITEM) style else MediaConstants.EXTRAS_VALUE_CONTENT_STYLE_LIST_ITEM)
    }

    private fun MediaItem.inGroup(title: String): MediaItem {
        val extras = Bundle(mediaMetadata.extras ?: Bundle()).apply { putString(MediaConstants.EXTRAS_KEY_CONTENT_STYLE_GROUP_TITLE, title) }
        return buildUpon().setMediaMetadata(mediaMetadata.buildUpon().setExtras(extras).build()).build()
    }

    // ---- Playing ----

    /**
     * Picked in the car, or asked aloud: it plays through the app's queue
     * at once, and the player the session hands it to leaves it be.
     */
    override fun onSetMediaItems(
        mediaSession: MediaSession,
        controller: MediaSession.ControllerInfo,
        mediaItems: MutableList<MediaItem>,
        startIndex: Int,
        startPositionMs: Long,
    ): ListenableFuture<MediaSession.MediaItemsWithStartPosition> = later {
        app.start()
        val item = mediaItems.getOrNull(startIndex) ?: mediaItems.firstOrNull()
        val query = item?.requestMetadata?.searchQuery
        runCatching {
            when {
                item == null -> {}
                query != null -> playSearch(query)
                else -> play(item.mediaId)
            }
        }
        MediaSession.MediaItemsWithStartPosition(mediaItems, startIndex, startPositionMs)
    }

    private suspend fun play(id: String) {
        when (id) {
            SHUFFLE -> playback.shuffle(allSongs())
            else -> when (val node = nodes[id]) {
                is Node.Song -> playback.play(node.list, node.index)
                is Node.OfStation -> playback.playStation(node.station)
                is Node.OfAlbum -> playback.play(session.album(node.album.id).tracks, 0)
                is Node.OfPlaylist -> playback.play(playlistTracks(node.playlist.id), 0)
                else -> {}
            }
        }
    }

    /** "Play …" said to the assistant: the songs found, the best first; nothing said, the library shuffled. */
    private suspend fun playSearch(query: String) {
        if (query.isBlank()) {
            if (playback.now.value != null) playback.toggle() else playback.shuffle(allSongs())
            return
        }
        val found = runCatching { session.search(SearchScope.CATALOG, query, null) }.getOrNull() ?: return
        when {
            found.tracks.isNotEmpty() -> playback.play(found.tracks, 0)
            found.albums.isNotEmpty() -> playback.play(session.album(found.albums.first().id).tracks, 0)
        }
    }

    // ---- Favourite, shuffle and repeat ----

    /** The car, while connected: the buttons by the song show only then. */
    private val cars = mutableSetOf<MediaSession.ControllerInfo>()
    private var shownIn: MediaSession? = null

    private fun isCar(session: MediaSession, controller: MediaSession.ControllerInfo) =
        session.isAutoCompanionController(controller) || session.isAutomotiveController(controller) || controller.packageName == "com.google.android.projection.gearhead"

    override fun onConnect(session: MediaSession, controller: MediaSession.ControllerInfo): MediaSession.ConnectionResult {
        val available = MediaSession.ConnectionResult.DEFAULT_SESSION_AND_LIBRARY_COMMANDS.buildUpon().apply { commands.forEach(::add) }.build()
        return MediaSession.ConnectionResult.AcceptedResultBuilder(session).setAvailableSessionCommands(available).build()
    }

    override fun onPostConnect(session: MediaSession, controller: MediaSession.ControllerInfo) {
        if (!isCar(session, controller)) return
        cars += controller
        shownIn = session
        refresh()
    }

    override fun onDisconnected(session: MediaSession, controller: MediaSession.ControllerInfo) {
        if (cars.remove(controller) && cars.isEmpty()) session.setMediaButtonPreferences(emptyList())
    }

    /** The buttons as they stand now, while the car is connected. */
    fun refresh() {
        val session = shownIn ?: return
        if (cars.isNotEmpty()) session.setMediaButtonPreferences(buttons())
    }

    /** Whether the song playing is a favourite, once its service has said. */
    private val favourites = mutableMapOf<ItemId, Boolean>()

    fun isFavourite(track: Track): Boolean {
        favourites[track.id]?.let { return it }
        if (track.id.provider != ProviderKind.NAVIDROME) {
            favourites[track.id] = false
            scope.launch {
                runCatching { session.isFavourite(FavouriteKind.SONG, track.id) }.getOrNull()?.let {
                    favourites[track.id] = it
                    refresh()
                }
            }
        }
        return false
    }

    /** The buttons by the song in the car. */
    fun buttons(): List<CommandButton> {
        val track = playback.now.value
        val favourite = track?.let(::isFavourite) == true
        val repeatIcon = when (playback.repeat.value) {
            QueueRepeat.OFF -> CommandButton.ICON_REPEAT_OFF
            QueueRepeat.ALL -> CommandButton.ICON_REPEAT_ALL
            QueueRepeat.ONE -> CommandButton.ICON_REPEAT_ONE
        }
        return listOfNotNull(
            track?.takeIf { it.id.provider != ProviderKind.NAVIDROME }?.let {
                button(if (favourite) CommandButton.ICON_STAR_FILLED else CommandButton.ICON_STAR_UNFILLED, if (favourite) "Unfavourite" else "Favourite", FAVOURITE)
            },
            button(if (playback.shuffleOn.value) CommandButton.ICON_SHUFFLE_ON else CommandButton.ICON_SHUFFLE_OFF, "Shuffle", SHUFFLE_MODE),
            button(repeatIcon, "Repeat", REPEAT_MODE),
        )
    }

    private fun button(icon: Int, name: String, command: String) =
        CommandButton.Builder(icon).setDisplayName(name).setSessionCommand(SessionCommand(command, Bundle.EMPTY)).build()

    val commands = listOf(FAVOURITE, SHUFFLE_MODE, REPEAT_MODE).map { SessionCommand(it, Bundle.EMPTY) }

    override fun onCustomCommand(session: MediaSession, controller: MediaSession.ControllerInfo, customCommand: SessionCommand, args: Bundle): ListenableFuture<SessionResult> {
        when (customCommand.customAction) {
            FAVOURITE -> playback.now.value?.let(::toggleFavourite)
            SHUFFLE_MODE -> playback.setShuffle(!playback.shuffleOn.value)
            REPEAT_MODE -> playback.cycleRepeat()
            else -> return Futures.immediateFuture(SessionResult(SessionResult.RESULT_ERROR_NOT_SUPPORTED))
        }
        refresh()
        return Futures.immediateFuture(SessionResult(SessionResult.RESULT_SUCCESS))
    }

    /** As the app does: a song favourited joins the library, unless the setting says not to. */
    private fun toggleFavourite(track: Track) {
        val on = !isFavourite(track)
        favourites[track.id] = on
        scope.launch {
            runCatching {
                session.setFavourite(FavouriteKind.SONG, track.id, on)
                if (on && track.id.provider == ProviderKind.APPLE_MUSIC && app.settings.favouritesToLibrary && !track.inLibrary) {
                    session.addToLibrary(FavouriteKind.SONG, track.id)
                }
            }.onFailure {
                favourites[track.id] = !on
                refresh()
            }
        }
    }

    /** `work`'s result as a future, done on the app's main scope. */
    private fun <T> later(work: suspend () -> T): ListenableFuture<T> {
        val future = SettableFuture.create<T>()
        scope.launch {
            try {
                future.set(work())
            } catch (e: kotlinx.coroutines.CancellationException) {
                future.cancel(false)
                throw e
            } catch (e: Exception) {
                future.setException(e)
            }
        }
        return future
    }

    private companion object {
        const val ROOT = "root"
        const val HOME = "home"
        const val RADIO = "radio"
        const val LIBRARY = "library"
        const val SHUFFLE = "library/shuffle"
        const val RECENT = "library/recent"
        const val PLAYLISTS = "library/playlists"
        const val ARTISTS = "library/artists"
        const val ALBUMS = "library/albums"
        const val SONGS = "library/songs"
        const val FAVOURITE = "favourite"
        const val SHUFFLE_MODE = "shuffle"
        const val REPEAT_MODE = "repeat"
    }
}
