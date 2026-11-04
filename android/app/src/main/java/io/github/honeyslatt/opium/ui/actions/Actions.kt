package io.github.honeyslatt.opium.ui.actions

import android.content.Context
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateMapOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.compose.runtime.staticCompositionLocalOf
import androidx.compose.ui.geometry.Rect
import androidx.media3.common.util.UnstableApi
import io.github.honeyslatt.opium.core.OpiumException
import io.github.honeyslatt.opium.core.model.Album
import io.github.honeyslatt.opium.core.model.Artwork
import io.github.honeyslatt.opium.core.model.FavouriteKind
import io.github.honeyslatt.opium.core.model.ItemId
import io.github.honeyslatt.opium.core.model.Playlist
import io.github.honeyslatt.opium.core.model.ProviderKind
import io.github.honeyslatt.opium.core.model.Track
import io.github.honeyslatt.opium.data.Core
import io.github.honeyslatt.opium.data.Library
import io.github.honeyslatt.opium.data.LocalFiles
import io.github.honeyslatt.opium.playback.Playback
import io.github.honeyslatt.opium.ui.Icons
import io.github.honeyslatt.opium.ui.components.Hud
import io.github.honeyslatt.opium.ui.components.MenuHead
import io.github.honeyslatt.opium.ui.components.MenuItem
import io.github.honeyslatt.opium.ui.components.MenuSpec
import io.github.honeyslatt.opium.ui.components.label
import io.github.honeyslatt.opium.data.downloadable as downloadableHere
import io.github.honeyslatt.opium.ui.components.Menus
import io.github.honeyslatt.opium.ui.nav.Navigator
import io.github.honeyslatt.opium.ui.nav.Route
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.async
import kotlinx.coroutines.awaitAll
import kotlinx.coroutines.launch

/** The sheets an action opens. */
sealed interface SheetSpec {
    /** Songs to add to playlists of their service, said by `title`. */
    data class AddToPlaylists(val tracks: List<Track>, val title: String, val subtitle: String, val artwork: Artwork?) : SheetSpec

    /** A new playlist (with `tracks` in it), or `editing`'s name and words. */
    data class PlaylistForm(val editing: Playlist?, val tracks: List<Track> = emptyList()) : SheetSpec

    data class Confirm(val title: String, val text: String, val label: String, val action: () -> Unit) : SheetSpec

    /** Navidrome asks for the password again, for `then` (a cover changed). */
    data class NavidromePassword(val then: () -> Unit) : SheetSpec

    /** The phone's music files' tags, one song's or an album's at once. */
    data class Info(val tracks: List<Track>) : SheetSpec
}

val LocalActions = staticCompositionLocalOf<Actions?> { null }

/**
 * What the menus offer, and doing it: the queue at once, the services'
 * changes shown at once and taken back if the service refuses.
 */
@UnstableApi
class Actions(
    private val context: Context,
    private val core: Core,
    private val library: Library,
    private val playback: Playback,
    private val nav: Navigator,
    val hud: Hud,
    val menus: Menus,
    private val scope: CoroutineScope,
) {
    private val session get() = core.session
    private val prefs = context.getSharedPreferences("opium", Context.MODE_PRIVATE)
    private val settings = (context.applicationContext as io.github.honeyslatt.opium.OpiumApp).settings
    private val downloads = (context.applicationContext as io.github.honeyslatt.opium.OpiumApp).downloads

    /** Called before a page opens, as Now Playing closes for it. */
    var beforeLeaving: () -> Unit = {}

    var sheet by mutableStateOf<SheetSpec?>(null)

    /** Favourites as known: what the service said, or what was just done. */
    private val favourites = mutableStateMapOf<ItemId, Boolean>()
    private val asking = mutableSetOf<ItemId>()

    /** The songs of Apple's playlist of favourites, read once page by page,
     *  so that a list can mark them without asking for each. */
    private var favouriteSongs by mutableStateOf<Set<ItemId>>(emptySet())
    private var readingFavourites = false

    /**
     * Whether a song is saved, for Spotify's check in the lists: as known,
     * else as Apple's playlist of favourites has it; a SoundCloud like is
     * its library.
     */
    fun saved(track: Track): Boolean {
        favourites[track.id]?.let { return it }
        if (track.id.provider == ProviderKind.SOUND_CLOUD) return inLibrary(track)
        readFavouriteSongs()
        return track.id in favouriteSongs || track.libraryId?.let { it in favouriteSongs } == true
    }

    /** Apple's playlist of favourites, read once the library lists it. */
    private fun readFavouriteSongs() {
        if (readingFavourites) return
        val playlist = library.lists.value.playlists.items.firstOrNull { it.favourites } ?: return
        readingFavourites = true
        scope.launch {
            runCatching {
                var page: String? = null
                do {
                    val got = session.playlistTracks(playlist.id, page)
                    favouriteSongs = favouriteSongs + got.items.flatMap { listOfNotNull(it.id, it.libraryId) }
                    page = got.next
                } while (page != null)
            }
        }
    }

    /** Songs added to the library, or taken out, since it loaded. */
    private val inLibraryNow = mutableStateMapOf<ItemId, Boolean>()

    /** The songs of the user's playlists, to say which have a song already. */
    val contents = mutableStateMapOf<ItemId, Set<ItemId>>()

    /** Settings → Library. */
    val favouritesToLibrary get() = settings.favouritesToLibrary
    val playlistSongsToLibrary get() = settings.playlistSongsToLibrary

    fun inLibrary(t: Track) = t.id.provider == ProviderKind.NAVIDROME || (inLibraryNow[t.id] ?: t.inLibrary)

    /**
     * Whether `id` is a favourite; asked of the service the first time, and
     * again a little later if it did not answer. Until it does, a song is
     * one when Apple's playlist of favourites has it.
     */
    fun isFavourite(kind: FavouriteKind, id: ItemId, said: Boolean? = null): Boolean {
        favourites[id]?.let { return it }
        if (said != null) return said
        if (id.provider != ProviderKind.NAVIDROME && asking.add(id)) {
            scope.launch {
                for (wait in listOf(0L, 5_000L, 30_000L)) {
                    kotlinx.coroutines.delay(wait)
                    runCatching { session.isFavourite(kind, id) }.getOrNull()?.let {
                        favourites[id] = it
                        return@launch
                    }
                }
                // Asked again the next time it is shown.
                asking.remove(id)
            }
        }
        if (kind == FavouriteKind.SONG && id.provider == ProviderKind.APPLE_MUSIC) {
            readFavouriteSongs()
            return id in favouriteSongs
        }
        return false
    }

    private fun go(route: Route) {
        beforeLeaving()
        nav.push(route)
    }

    /** Do `change` on the service; on failure, say so and undo. */
    private fun change(failed: String, undo: () -> Unit = {}, change: suspend () -> Unit) {
        scope.launch {
            try {
                change()
            } catch (e: OpiumException) {
                undo()
                hud.show(failed, Icons.Alert)
            }
        }
    }

    // ---- Menus ----

    fun isStation(track: Track) = playback.isStation(track)

    /** The sleep timer's choices, the one set ticked. */
    fun sleepMenu(anchor: Rect) {
        val set = playback.sleep.value
        menus.show(
            MenuSpec(anchor, null) {
                listOf(
                    listOf(5, 15, 30, 45, 60).map { m ->
                        MenuItem(if (m == 60) "1 Hour" else "$m Minutes", Icons.Clock, checked = (set as? Playback.Sleep.At)?.minutes == m) {
                            playback.sleepIn(m)
                        }
                    } + MenuItem("End of Track", Icons.Note, checked = set == Playback.Sleep.EndOfTrack) { playback.sleepAtEndOfTrack() },
                    listOfNotNull(set?.let { MenuItem("Turn Off Timer", Icons.X) { playback.sleepIn(null) } }),
                )
            },
        )
    }

    /** The sleep timer's line in Now Playing's menu: what is left of it. */
    private fun sleepLabel(): String = when (val s = playback.sleep.value) {
        null -> "Sleep Timer"
        Playback.Sleep.EndOfTrack -> "Sleep Timer · End of Track"
        is Playback.Sleep.At -> {
            val left = ((s.endsAt - android.os.SystemClock.elapsedRealtime()).coerceAtLeast(0) + 59_999) / 60_000
            "Sleep Timer · $left min left"
        }
    }

    fun trackMenu(track: Track, anchor: Rect, inPlaylist: Playlist? = null, onRemoved: () -> Unit = {}, nowPlaying: Boolean = false) {
        if (playback.isStation(track)) return
        val head = MenuHead(track.artwork, track.title, track.artists.joinToString(", "))
        // The phone's own file: no library, favourite nor station of a service, its tags to change.
        if (track.id.provider == ProviderKind.LOCAL) {
            val here = LocalFiles.inIndex(track.id.id)
            menus.show(
                MenuSpec(anchor, head) {
                    listOf(
                        listOf(
                            MenuItem("Play Next", Icons.Next) { playNext(listOf(track)) },
                            MenuItem("Add to Queue", Icons.Later) { addToQueue(listOf(track)) },
                        ),
                        listOf(MenuItem("Add to a Playlist…", Icons.Listplus) { addToPlaylists(listOf(track)) }),
                        listOfNotNull(
                            track.albumId?.let { id -> MenuItem("Go to Album", Icons.Album) { go(Route.Album(id)) } },
                            MenuItem("Go to Artist", Icons.Mic) { goToArtist(track) },
                        ),
                        listOfNotNull(if (here) MenuItem("Get Info", Icons.Info) { sheet = SheetSpec.Info(listOf(track)) } else null),
                    )
                },
            )
            return
        }
        menus.show(
            MenuSpec(anchor, head) {
                val favourite = isFavourite(FavouriteKind.SONG, track.id)
                val apple = track.id.provider == ProviderKind.APPLE_MUSIC
                listOf(
                    listOf(
                        MenuItem("Play Next", Icons.Next) { playNext(listOf(track)) },
                        MenuItem("Add to Queue", Icons.Later) { addToQueue(listOf(track)) },
                    ),
                    listOfNotNull(
                        when {
                            track.id.provider == ProviderKind.NAVIDROME -> null
                            !inLibrary(track) -> MenuItem("Add to Library", Icons.Pluscircle) { addToLibrary(track) }
                            apple && track.libraryId != null ->
                                MenuItem("Delete from Library…", Icons.Trash, danger = true) { confirmDelete(track) }
                            // A like, on SoundCloud: undone at once, as easily made again.
                            track.id.provider == ProviderKind.SOUND_CLOUD ->
                                MenuItem("Remove from Library", Icons.Minuscircle, danger = true) { removeLike(track) }
                            else -> null
                        },
                        MenuItem("Add to a Playlist…", Icons.Listplus) { addToPlaylists(listOf(track)) },
                        downloadItem(listOf(track), track.title),
                        if (track.id.provider != ProviderKind.NAVIDROME) {
                            MenuItem(if (favourite) "Remove from Favourites" else "Favourite", if (favourite) Icons.Starf else Icons.Star) {
                                toggleFavourite(track)
                            }
                        } else {
                            null
                        },
                    ),
                    listOfNotNull(
                        track.albumId?.let { id -> MenuItem("Go to Album", Icons.Album) { go(Route.Album(id)) } },
                        MenuItem("Go to Artist", Icons.Mic) { goToArtist(track) },
                        if (apple) MenuItem("Start Station", Icons.Radio) { startStation(track) } else null,
                    ),
                    listOfNotNull(
                        inPlaylist?.takeIf { it.editable }?.let { p ->
                            MenuItem("Remove from Playlist", Icons.Minuscircle, danger = true) { removeFromPlaylist(p, track, onRemoved) }
                        },
                    ),
                    // Now Playing's own: the sleep timer.
                    listOfNotNull(if (nowPlaying) MenuItem(sleepLabel(), Icons.Clock, checked = playback.sleep.value != null) { sleepMenu(anchor) } else null),
                )
            },
        )
    }

    fun albumMenu(listed: Album, anchor: Rect, onDeleted: () -> Unit = {}) {
        val head = MenuHead(listed.artwork, listed.title, listed.artists.joinToString(", "))
        val saved = listed.added != null || listed.id.id.startsWith("l.")
        val apple = listed.id.provider == ProviderKind.APPLE_MUSIC
        readWhole(listed)
        menus.show(
            MenuSpec(anchor, head) {
                val album = whole[listed.id] ?: listed
                // Its id in the service's catalog: none for one the user uploaded.
                val catalog = album.catalogId ?: album.id.takeUnless { it.id.startsWith("l.") }
                // Apple's favourites and Suggest Less; a SoundCloud like is its library.
                val rated = catalog?.takeIf { apple }
                val favourite = rated != null && isFavourite(FavouriteKind.ALBUM, rated, album.inFavourites)
                val less = rated != null && suggestsLess(FavouriteKind.ALBUM, rated)
                listOf(
                    listOf(
                        MenuItem("Play", Icons.Play) { withAlbum(album) { playback.play(it, 0) } },
                        MenuItem("Shuffle", Icons.Shuffle) { withAlbum(album) { playback.shuffle(it) } },
                    ),
                    listOf(
                        MenuItem("Play Next", Icons.Next) { withAlbum(album) { playNext(it) } },
                        MenuItem("Add to Queue", Icons.Later) { withAlbum(album) { addToQueue(it) } },
                    ),
                    listOfNotNull(
                        when {
                            !apple -> null
                            !saved -> MenuItem("Add to Library", Icons.Pluscircle) { addAlbumToLibrary(album) }
                            album.id.id.startsWith("l.") ->
                                MenuItem("Delete from Library…", Icons.Trash, danger = true) { confirmDelete(listed, onDeleted) }
                            else -> null
                        },
                        MenuItem("Add to a Playlist…", Icons.Listplus) {
                            withAlbum(album) { sheet = SheetSpec.AddToPlaylists(it, songsCount(it.size), album.title, album.artwork) }
                        },
                        // The phone's files' album: their tags at once.
                        if (album.id.provider == ProviderKind.LOCAL) {
                            MenuItem("Edit Info", Icons.Pencil) { withAlbum(album) { sheet = SheetSpec.Info(it) } }
                        } else {
                            null
                        },
                        if (downloadable(album.id.provider)) {
                            MenuItem("Download", io.github.honeyslatt.opium.ui.MoreIcons.ArrowDown) { withAlbum(album) { download(it) } }
                        } else {
                            null
                        },
                        rated?.let { id ->
                            MenuItem(if (favourite) "Remove from Favourites" else "Favourite", if (favourite) Icons.Starf else Icons.Star) {
                                toggleFavourite(FavouriteKind.ALBUM, id, was = favourite)
                            }
                        },
                        rated?.let { id ->
                            MenuItem(if (less) "Undo Suggest Less" else "Suggest Less", Icons.Thumbsdown) { toggleSuggestLess(FavouriteKind.ALBUM, id, was = less) }
                        },
                    ),
                    listOfNotNull(
                        album.artistId?.let { id -> MenuItem("Go to Artist", Icons.Mic) { go(Route.Artist(id)) } },
                        rated?.let { id -> MenuItem("Start Station", Icons.Radio) { startStation(ItemId(ProviderKind.APPLE_MUSIC, "ra.${id.id}"), album.title) } },
                        catalog?.takeIf { shareable(it) }?.let { id -> MenuItem("Share Album", Icons.Share) { share(FavouriteKind.ALBUM, id) } },
                    ),
                )
            },
        )
    }

    /** Library albums as their pages read them, with the catalog's id and the artist their lists leave out. */
    private val whole = mutableStateMapOf<ItemId, Album>()

    /** A library album of Apple's, read once its menu opens, for what its list leaves out. */
    private fun readWhole(album: Album) {
        val lacking = album.catalogId == null || album.artistId == null
        if (album.id.provider != ProviderKind.APPLE_MUSIC || !album.id.id.startsWith("l.") || !lacking || album.id in whole) return
        scope.launch { runCatching { session.album(album.id).album }.getOrNull()?.let { whole[album.id] = it } }
    }

    fun playlistMenu(playlist: Playlist, anchor: Rect, onDeleted: () -> Unit = {}) {
        val head = MenuHead(playlist.artwork, if (playlist.favourites) "Favourite Songs" else playlist.name, "Playlist", star = playlist.favourites)
        val favouriteId = playlist.catalogId ?: playlist.id
        // Its page on the web: none for an Apple playlist of the user's never published.
        val shared = (playlist.catalogId ?: playlist.id.takeUnless { it.id.startsWith("p.") })?.takeIf { shareable(it) }
        menus.show(
            MenuSpec(anchor, head) {
                val play = listOf(
                    MenuItem("Play", Icons.Play) { withPlaylist(playlist) { playback.play(it, 0) } },
                    MenuItem("Shuffle", Icons.Shuffle) { withPlaylist(playlist) { playback.shuffle(it) } },
                )
                val queue = listOf(
                    MenuItem("Play Next", Icons.Next) { withPlaylist(playlist) { playNext(it) } },
                    MenuItem("Add to Queue", Icons.Later) { withPlaylist(playlist) { addToQueue(it) } },
                )
                val toPlaylist = MenuItem("Add to a Playlist…", Icons.Listplus) {
                    withPlaylist(playlist) { sheet = SheetSpec.AddToPlaylists(it, songsCount(it.size), head.title, playlist.artwork) }
                }
                // Its songs downloaded, and the list kept to find them offline.
                val download = MenuItem("Download", io.github.honeyslatt.opium.ui.MoreIcons.ArrowDown) {
                    withPlaylist(playlist) {
                        downloads.keepPlaylist(playlist.id, it)
                        download(it)
                    }
                }
                val favourite = !playlist.favourites && isFavourite(FavouriteKind.PLAYLIST, favouriteId, playlist.inFavourites)
                if (playlist.favourites) listOf(play, queue, listOf(toPlaylist, download)) else listOf(
                    play,
                    queue,
                    listOfNotNull(
                        toPlaylist,
                        download,
                        if (playlist.editable) MenuItem("Edit…", Icons.Pencil) { sheet = SheetSpec.PlaylistForm(playlist) } else null,
                        if (playlist.id.provider == ProviderKind.APPLE_MUSIC) {
                            MenuItem(if (favourite) "Remove from Favourites" else "Favourite", if (favourite) Icons.Starf else Icons.Star) {
                                togglePlaylistFavourite(playlist)
                            }
                        } else {
                            null
                        },
                    ),
                    listOfNotNull(shared?.let { id -> MenuItem("Share Playlist", Icons.Share) { share(FavouriteKind.PLAYLIST, id) } }),
                    listOfNotNull(
                        if (playlist.editable) {
                            MenuItem("Delete from Library…", Icons.Trash, danger = true) { confirmDelete(playlist, onDeleted) }
                        } else {
                            null
                        },
                    ),
                )
            },
        )
    }

    fun artistMenu(detail: io.github.honeyslatt.opium.core.model.ArtistDetail, anchor: Rect) {
        val songs = detail.topSongs
        menus.show(
            MenuSpec(anchor, null) {
                listOf(
                    listOf(
                        MenuItem("Play", Icons.Play) { playback.play(songs, 0) },
                        MenuItem("Shuffle", Icons.Shuffle) { playback.shuffle(songs) },
                    ),
                )
            },
        )
    }

    fun isFavourite(artist: io.github.honeyslatt.opium.core.model.Artist) =
        isFavourite(FavouriteKind.ARTIST, artist.id, artist.inFavourites)

    fun toggleFavourite(artist: io.github.honeyslatt.opium.core.model.Artist) {
        val was = isFavourite(artist)
        favourites[artist.id] = !was
        hud.show(if (was) "Removed from Favourites" else "Favourited", if (was) Icons.Star else Icons.Starf)
        change("Couldn't change the favourites", undo = { favourites[artist.id] = was }) {
            session.setFavourite(FavouriteKind.ARTIST, artist.id, !was)
        }
    }

    private val mosaics = java.util.concurrent.ConcurrentHashMap<ItemId, List<Artwork>>()

    /** The covers of a playlist's first four albums, for one without a cover of its own. */
    suspend fun mosaic(playlist: ItemId): List<Artwork> {
        mosaics[playlist]?.let { return it }
        val covers = runCatching { session.playlistTracks(playlist, null).items }.getOrNull().orEmpty()
            .distinctBy { it.albumId ?: it.id }
            .mapNotNull { it.artwork }
            .take(4)
        mosaics[playlist] = covers
        return covers
    }

    // ---- The queue ----

    // With nothing playing, they just play: no word needed.
    fun playNext(tracks: List<Track>) {
        val idle = playback.now.value == null
        playback.playNext(tracks)
        if (!idle) hud.show("Playing Next", Icons.Next)
    }

    fun addToQueue(tracks: List<Track>) {
        val idle = playback.now.value == null
        playback.addToQueue(tracks)
        if (!idle) hud.show("Added to Queue", Icons.Later)
    }

    private fun withAlbum(album: Album, then: (List<Track>) -> Unit) {
        scope.launch {
            runCatching { session.album(album.id).tracks }
                .onSuccess { if (it.isNotEmpty()) then(it) }
                .onFailure { hud.show("Couldn't load the album", Icons.Alert) }
        }
    }

    private fun withPlaylist(playlist: Playlist, then: (List<Track>) -> Unit) {
        scope.launch {
            runCatching { allTracks(playlist.id) }
                .onSuccess { if (it.isNotEmpty()) then(it) }
                .onFailure { hud.show("Couldn't load the playlist", Icons.Alert) }
        }
    }

    private suspend fun allTracks(playlist: ItemId): List<Track> {
        val all = mutableListOf<Track>()
        var page: String? = null
        do {
            val p = session.playlistTracks(playlist, page)
            all += p.items
            page = p.next
        } while (page != null)
        return all
    }

    // ---- Downloads ----

    /** Songs of `provider` can be downloaded: Opium's playlists have none of their own. */
    private fun downloadable(provider: ProviderKind) = provider != ProviderKind.OPIUM && provider != ProviderKind.LOCAL

    /** A song's Download, or its Remove Download once kept or coming. */
    private fun downloadItem(tracks: List<Track>, title: String): MenuItem? {
        val track = tracks.singleOrNull()?.takeIf(::downloadableHere) ?: return null
        return when (downloads.states.value[track.id]) {
            null, is io.github.honeyslatt.opium.core.DownloadState.Failed, io.github.honeyslatt.opium.core.DownloadState.Gone ->
                MenuItem("Download", io.github.honeyslatt.opium.ui.MoreIcons.ArrowDown) { download(tracks) }
            else -> MenuItem("Remove Download", Icons.Minuscircle, danger = true) { confirmRemoveDownload(tracks, title) }
        }
    }

    /** A song tapped that can't play: its service out of reach, it not downloaded. */
    fun notDownloaded(track: Track) {
        hud.show("Not downloaded, and ${track.id.provider.label()} is out of reach", Icons.Alert)
    }

    /** Download what of `tracks` isn't yet. */
    fun download(tracks: List<Track>) {
        val states = downloads.states.value
        val own = tracks.filter(::downloadableHere).filter {
            val s = states[it.id]
            s == null || s is io.github.honeyslatt.opium.core.DownloadState.Failed
        }
        if (own.isEmpty()) return
        downloads.add(own)
        hud.show("Downloading ${songsCount(own.size)}", io.github.honeyslatt.opium.ui.MoreIcons.ArrowDown)
    }

    /** What of `tracks` waits or comes, no longer. */
    fun stopDownloading(tracks: List<Track>) {
        val states = downloads.states.value
        val ids = tracks.map { it.id }.filter {
            states[it] is io.github.honeyslatt.opium.core.DownloadState.Queued || states[it] is io.github.honeyslatt.opium.core.DownloadState.Coming
        }
        downloads.remove(ids)
        hud.show("Download stopped", Icons.X)
    }

    /** `tracks` off the phone, asked first. */
    fun confirmRemoveDownload(tracks: List<Track>, title: String) {
        sheet = SheetSpec.Confirm(
            "Remove Download?",
            "“$title” goes from this phone. It stays in your library, to play online.",
            "Remove Download",
        ) {
            downloads.remove(tracks.flatMap { listOfNotNull(it.id, it.libraryId) }.distinct())
            hud.show("Download removed", Icons.Trash)
        }
    }

    // ---- Pages ----

    fun goToArtist(track: Track) {
        scope.launch {
            val artist = runCatching { session.songLinks(track.id).artist }.getOrNull()
            if (artist != null) go(Route.Artist(artist)) else hud.show("No artist page", Icons.Alert)
        }
    }

    fun goToAlbum(track: Track) {
        track.albumId?.let { go(Route.Album(it)) }
    }

    fun startStation(track: Track) {
        scope.launch {
            try {
                val station = session.songLinks(track.id).station ?: return@launch hud.show("No station for this song", Icons.Alert)
                playback.playStation(station)
                hud.show("${track.title} Station", Icons.Radio)
            } catch (e: OpiumException) {
                hud.show("Couldn't start the station", Icons.Alert)
            }
        }
    }

    /** An album's station, as Apple names it: `ra.` and the album's id. */
    private fun startStation(station: ItemId, title: String) {
        scope.launch {
            try {
                playback.playStation(station)
                hud.show("$title Station", Icons.Radio)
            } catch (e: OpiumException) {
                hud.show("Couldn't start the station", Icons.Alert)
            }
        }
    }

    /** Whether `id`'s service has pages on the web to share: Opium's own and Navidrome's are private. */
    private fun shareable(id: ItemId) = id.provider == ProviderKind.APPLE_MUSIC || id.provider == ProviderKind.SOUND_CLOUD

    /** Its link on the service's website, through Android's share sheet. */
    private fun share(kind: FavouriteKind, id: ItemId) {
        scope.launch {
            val link = runCatching { session.shareLink(kind, id) }.getOrNull()
                ?: return@launch hud.show("Couldn't share it", Icons.Alert)
            val send = android.content.Intent(android.content.Intent.ACTION_SEND)
                .setType("text/plain")
                .putExtra(android.content.Intent.EXTRA_TEXT, link)
            context.startActivity(android.content.Intent.createChooser(send, null).addFlags(android.content.Intent.FLAG_ACTIVITY_NEW_TASK))
        }
    }

    // ---- Library and favourites ----

    fun addToLibrary(track: Track) {
        inLibraryNow[track.id] = true
        hud.show("Added to Library")
        change("Couldn't add to the library", undo = { inLibraryNow.remove(track.id) }) {
            if (track.id.provider == ProviderKind.SOUND_CLOUD) {
                // A like, on SoundCloud.
                session.setFavourite(FavouriteKind.SONG, track.id, true)
                favourites[track.id] = true
            } else {
                session.addToLibrary(FavouriteKind.SONG, track.id)
            }
            library.recentChanged(track.id.provider)
        }
    }

    fun addAlbumToLibrary(album: Album) {
        hud.show("Added to Library")
        change("Couldn't add to the library") {
            session.addToLibrary(FavouriteKind.ALBUM, album.catalogId ?: album.id)
            library.recentChanged(album.id.provider)
        }
    }

    private fun confirmDelete(track: Track) {
        val id = track.libraryId ?: return
        sheet = SheetSpec.Confirm(
            "Delete from Library?",
            "“${track.title}” goes from your library on every device, and from your playlists.",
            "Delete Song",
        ) {
            inLibraryNow[track.id] = false
            hud.show("Deleted", Icons.Trash)
            change("Couldn't delete it", undo = { inLibraryNow.remove(track.id) }) {
                session.removeFromLibrary(id)
                library.recentChanged(track.id.provider)
                leavePlaylists(listOf(track))
            }
        }
    }

    private fun confirmDelete(album: Album, onDeleted: () -> Unit) {
        sheet = SheetSpec.Confirm(
            "Delete from Library?",
            "“${album.title}” and its songs go from your library on every device, and from your playlists.",
            "Delete Album",
        ) {
            library.albumDeleted(album.id)
            onDeleted()
            hud.show("Deleted", Icons.Trash)
            change("Couldn't delete it", undo = { library.albumBack(album) }) {
                // Its songs, read while it is still in the library.
                val songs = if (settings.removalsLeavePlaylists) session.album(album.id).tracks else emptyList()
                session.removeFromLibrary(album.id)
                library.recentChanged(album.id.provider)
                leavePlaylists(songs)
            }
        }
    }

    private fun removeLike(track: Track) {
        inLibraryNow[track.id] = false
        hud.show("Removed from Library", Icons.Minuscircle)
        change("Couldn't remove it", undo = { inLibraryNow.remove(track.id) }) {
            session.removeFromLibrary(track.id)
            library.recentChanged(track.id.provider)
            leavePlaylists(listOf(track))
        }
    }

    /**
     * Songs taken out of the library leave the user's playlists too, if
     * Settings → Library says so: Opium's own, and SoundCloud's of theirs.
     * Apple takes its songs out of its playlists itself.
     */
    private fun leavePlaylists(songs: List<Track>) {
        if (!settings.removalsLeavePlaylists || songs.isEmpty()) return
        val ids = songs.flatMap { listOfNotNull(it.id, it.libraryId) }.toSet()
        val providers = songs.map { it.id.provider }.toSet()
        val mine = library.lists.value.playlists.items.filter { p ->
            p.editable && (p.id.provider == ProviderKind.OPIUM || (p.id.provider == ProviderKind.SOUND_CLOUD && p.id.provider in providers))
        }
        mine.forEach { p ->
            change("Couldn't take it out of “${p.name}”") {
                // Opium and SoundCloud name a playlist's row by the song's id.
                val there = allTracks(p.id).filter { it.id in ids }.map { it.id }
                there.forEach { session.removeFromPlaylist(p.id, it) }
                if (there.isNotEmpty()) contents[p.id]?.let { contents[p.id] = it - there.toSet() }
            }
        }
    }

    /** Favourite, or not; a song favourited joins the library unless the setting says not to. */
    fun toggleFavourite(track: Track) {
        val on = !isFavourite(FavouriteKind.SONG, track.id)
        favourites[track.id] = on
        val soundCloud = track.id.provider == ProviderKind.SOUND_CLOUD
        val joins = on && !soundCloud && favouritesToLibrary && !inLibrary(track)
        if (soundCloud) inLibraryNow[track.id] = on
        if (joins) inLibraryNow[track.id] = true
        hud.show(
            when {
                !on -> "Removed from Favourites"
                joins -> "Favourited and Added to Library"
                else -> "Favourited"
            },
            if (on) Icons.Starf else Icons.Star,
        )
        change(
            "Couldn't change the favourites",
            undo = {
                favourites[track.id] = !on
                if (soundCloud || joins) inLibraryNow.remove(track.id)
            },
        ) {
            if (joins) session.addToLibrary(FavouriteKind.SONG, track.id)
            session.setFavourite(FavouriteKind.SONG, track.id, on)
            // A SoundCloud like is the library.
            if (soundCloud || joins) library.recentChanged(track.id.provider)
        }
    }

    /** Favourite, or not, from `was` as the menu showed it. */
    private fun toggleFavourite(kind: FavouriteKind, id: ItemId, was: Boolean) {
        favourites[id] = !was
        hud.show(if (was) "Removed from Favourites" else "Favourited", if (was) Icons.Star else Icons.Starf)
        change("Couldn't change the favourites", undo = { favourites[id] = was }) {
            session.setFavourite(kind, id, !was)
        }
    }

    /** Suggest Less as known: what the service said, or what was just done. */
    private val less = mutableStateMapOf<ItemId, Boolean>()
    private val askingLess = mutableSetOf<ItemId>()

    /** Whether less like `id` was asked for; asked of the service the first time. */
    private fun suggestsLess(kind: FavouriteKind, id: ItemId): Boolean {
        less[id]?.let { return it }
        if (askingLess.add(id)) {
            scope.launch {
                runCatching { session.suggestsLess(kind, id) }.getOrNull()?.let { less[id] = it }
                    // Asked again the next time it is shown.
                    ?: askingLess.remove(id)
            }
        }
        return false
    }

    private fun toggleSuggestLess(kind: FavouriteKind, id: ItemId, was: Boolean) {
        less[id] = !was
        hud.show(if (was) "Suggest Less Undone" else "Suggesting Less", Icons.Thumbsdown)
        change("Couldn't change it", undo = { less[id] = was }) {
            session.suggestLess(kind, id, !was)
        }
    }

    fun isFavourite(playlist: Playlist) =
        !playlist.favourites && isFavourite(FavouriteKind.PLAYLIST, playlist.catalogId ?: playlist.id, playlist.inFavourites)

    fun togglePlaylistFavourite(playlist: Playlist) {
        val id = playlist.catalogId ?: playlist.id
        val was = isFavourite(playlist)
        favourites[id] = !was
        hud.show(if (was) "Removed from Favourites" else "Favourited", if (was) Icons.Star else Icons.Starf)
        change("Couldn't change the favourites", undo = { favourites[id] = was }) {
            session.setFavourite(FavouriteKind.PLAYLIST, id, !was)
        }
    }

    // ---- Playlists ----

    fun addToPlaylists(tracks: List<Track>) {
        val one = tracks.singleOrNull()
        sheet = SheetSpec.AddToPlaylists(
            tracks,
            one?.title ?: songsCount(tracks.size),
            one?.artists?.joinToString(", ") ?: "",
            tracks.firstOrNull()?.artwork,
        )
    }

    /** The user's playlists `tracks` can go in: Opium's own, and theirs of the songs' service. */
    fun editablePlaylists(tracks: List<Track>): List<Playlist> {
        val provider = tracks.map { it.id.provider }.distinct().singleOrNull()
        return library.lists.value.playlists.items.filter { it.editable && (it.id.provider == ProviderKind.OPIUM || it.id.provider == provider) }
    }

    /** Where a new playlist of `tracks` can go: Opium's own, or their service; any connected, without songs. */
    fun newPlaylistServices(tracks: List<Track>): List<ProviderKind> {
        val connected = io.github.honeyslatt.opium.data.SERVICES.filter { core.connection(it).value is io.github.honeyslatt.opium.data.Connection.Connected }
        val theirs = if (tracks.isEmpty()) connected else listOfNotNull(tracks.map { it.id.provider }.distinct().singleOrNull()?.takeIf { it in connected })
        return listOf(ProviderKind.OPIUM) + theirs
    }

    /** The playlists songs were last added to, most recent first. */
    fun recentPlaylists(tracks: List<Track>): List<Playlist> {
        val fit = editablePlaylists(tracks)
        return recent().mapNotNull { id -> fit.firstOrNull { it.id.id == id } }
    }

    private val devices = (context.applicationContext as io.github.honeyslatt.opium.OpiumApp).devices

    /** As every device has them; those kept on the phone before, taken in once. */
    private fun recent(): List<String> {
        prefs.getString("recent-playlists", null)?.let { old ->
            if (devices.usedPlaylists().isEmpty()) devices.usePlaylists(old.split('\n').filter { it.isNotEmpty() })
            prefs.edit().remove("recent-playlists").apply()
        }
        return devices.usedPlaylists()
    }

    private fun usedNow(playlists: List<Playlist>) = devices.usePlaylists(playlists.map { it.id.id })

    /** Load what the playlists hold, those not known yet. */
    fun loadContents(playlists: List<Playlist>) {
        scope.launch {
            playlists.filter { it.id !in contents }.map { p ->
                async { runCatching { allTracks(p.id) }.getOrNull()?.let { contents[p.id] = it.map { t -> t.id }.toSet() } }
            }.awaitAll()
        }
    }

    fun addToChosen(tracks: List<Track>, chosen: List<Playlist>) {
        if (chosen.isEmpty()) return
        sheet = null
        usedNow(chosen)
        joinLibrary(tracks)
        hud.show(if (chosen.size == 1) "Added to “${chosen[0].name}”" else "Added to ${chosen.size} Playlists")
        chosen.forEach { p ->
            contents[p.id]?.let { contents[p.id] = it + tracks.map { t -> t.id } }
            change("Couldn't add to “${p.name}”", undo = { contents.remove(p.id) }) {
                session.addToPlaylist(p.id, tracks)
            }
        }
    }

    /** Songs put in a playlist join the library, unless the setting says not to. */
    private fun joinLibrary(tracks: List<Track>) {
        if (!playlistSongsToLibrary) return
        tracks.filter { it.id.provider == ProviderKind.APPLE_MUSIC && !inLibrary(it) }.forEach { t ->
            inLibraryNow[t.id] = true
            change("Couldn't add to the library", undo = { inLibraryNow.remove(t.id) }) {
                session.addToLibrary(FavouriteKind.SONG, t.id)
                library.recentChanged(t.id.provider)
            }
        }
    }

    fun createPlaylist(name: String, description: String, tracks: List<Track>, provider: ProviderKind, cover: ByteArray? = null) {
        sheet = null
        joinLibrary(tracks)
        scope.launch {
            try {
                val made = session.createPlaylist(provider, name, description, tracks)
                if (cover != null && coverable(provider)) setPlaylistCover(made.id, cover)
                library.recentChanged(provider)
                library.playlistMade(made)
                contents[made.id] = tracks.map { it.id }.toSet()
                usedNow(listOf(made))
                hud.show(if (tracks.isEmpty()) "Created" else "Added to “$name”")
            } catch (e: OpiumException) {
                hud.show("Couldn't make the playlist", Icons.Alert)
            }
        }
    }

    fun editPlaylist(playlist: Playlist, name: String, description: String) {
        sheet = null
        if (name == playlist.name && description == playlist.description.orEmpty()) return
        library.playlistEdited(playlist.id, name, description)
        hud.show("Saved")
        change(
            "Couldn't save the playlist",
            undo = { library.playlistEdited(playlist.id, playlist.name, playlist.description.orEmpty()) },
        ) { session.editPlaylist(playlist.id, name, description) }
    }

    /** Whether `provider`'s playlists can take a cover from here: Opium's own and Navidrome's. */
    fun coverable(provider: ProviderKind) = provider == ProviderKind.OPIUM || provider == ProviderKind.NAVIDROME

    /**
     * `playlist`'s cover `jpeg` (made by [io.github.honeyslatt.opium.data.CoverImage]),
     * or none for the one it had; Navidrome may want the password first.
     */
    fun setPlaylistCover(playlist: io.github.honeyslatt.opium.core.model.ItemId, jpeg: ByteArray?) {
        scope.launch {
            try {
                session.setPlaylistCover(playlist, jpeg)
                if (playlist.provider == ProviderKind.NAVIDROME) core.keepNavidrome()
                library.playlistsChanged(playlist.provider)
                coverChanges.value++
            } catch (e: OpiumException.PasswordNeeded) {
                sheet = SheetSpec.NavidromePassword { setPlaylistCover(playlist, jpeg) }
            } catch (e: OpiumException) {
                hud.show(e.message?.let { "Couldn't change the cover: $it" } ?: "Couldn't change the cover", Icons.Alert)
            }
        }
    }

    /** Counts the covers changed: a page showing one reads it again. */
    val coverChanges = kotlinx.coroutines.flow.MutableStateFlow(0)

    /** The password for Navidrome's session, then what wanted it; false: refused. */
    suspend fun navidromePassword(password: String, then: () -> Unit): Boolean {
        val opened = try {
            core.openNavidromeSession(password)
        } catch (e: OpiumException) {
            hud.show("Couldn't reach Navidrome", Icons.Alert)
            return true
        }
        if (opened) {
            sheet = null
            then()
        }
        return opened
    }

    fun confirmDelete(playlist: Playlist, onDeleted: () -> Unit = {}) {
        sheet = SheetSpec.Confirm(
            "Delete Playlist?",
            "“${playlist.name}” goes from your library on every device.",
            "Delete Playlist",
        ) {
            library.playlistDeleted(playlist.id)
            onDeleted()
            hud.show("Deleted", Icons.Trash)
            change("Couldn't delete the playlist", undo = { library.playlistMade(playlist) }) {
                session.deletePlaylist(playlist.id)
                library.recentChanged(playlist.id.provider)
            }
        }
    }

    /** Apple names a playlist's row by the song's library id; SoundCloud and Opium by the song's. */
    private fun removeFromPlaylist(playlist: Playlist, track: Track, onRemoved: () -> Unit) {
        val entry = if (playlist.id.provider == ProviderKind.APPLE_MUSIC) track.libraryId ?: return else track.id
        onRemoved()
        contents[playlist.id]?.let { contents[playlist.id] = it - track.id }
        hud.show("Removed", Icons.Minuscircle)
        change("Couldn't remove it") { session.removeFromPlaylist(playlist.id, entry) }
    }
}

fun songsCount(n: Int) = if (n == 1) "1 song" else "$n songs"
