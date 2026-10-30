package io.github.honeyslatt.music.ui.actions

import android.content.Context
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateMapOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.compose.runtime.staticCompositionLocalOf
import androidx.compose.ui.geometry.Rect
import androidx.media3.common.util.UnstableApi
import io.github.honeyslatt.music.core.MusicException
import io.github.honeyslatt.music.core.model.Album
import io.github.honeyslatt.music.core.model.Artwork
import io.github.honeyslatt.music.core.model.FavouriteKind
import io.github.honeyslatt.music.core.model.ItemId
import io.github.honeyslatt.music.core.model.Playlist
import io.github.honeyslatt.music.core.model.ProviderKind
import io.github.honeyslatt.music.core.model.Track
import io.github.honeyslatt.music.data.Core
import io.github.honeyslatt.music.data.Library
import io.github.honeyslatt.music.playback.Playback
import io.github.honeyslatt.music.ui.Icons
import io.github.honeyslatt.music.ui.components.Hud
import io.github.honeyslatt.music.ui.components.MenuHead
import io.github.honeyslatt.music.ui.components.MenuItem
import io.github.honeyslatt.music.ui.components.MenuSpec
import io.github.honeyslatt.music.ui.components.Menus
import io.github.honeyslatt.music.ui.nav.Navigator
import io.github.honeyslatt.music.ui.nav.Route
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
}

val LocalActions = staticCompositionLocalOf<Actions?> { null }

/** How many songs a station plays at once. */
private const val STATION_SONGS = 25u

/**
 * What the menus offer, and doing it: the queue at once, the services'
 * changes shown at once and taken back if the service refuses.
 */
@UnstableApi
class Actions(
    context: Context,
    private val core: Core,
    private val library: Library,
    private val playback: Playback,
    private val nav: Navigator,
    val hud: Hud,
    val menus: Menus,
    private val scope: CoroutineScope,
) {
    private val session get() = core.session
    private val prefs = context.getSharedPreferences("music", Context.MODE_PRIVATE)

    /** Called before a page opens, as Now Playing closes for it. */
    var beforeLeaving: () -> Unit = {}

    var sheet by mutableStateOf<SheetSpec?>(null)

    /** Favourites as known: what the service said, or what was just done. */
    private val favourites = mutableStateMapOf<ItemId, Boolean>()
    private val asking = mutableSetOf<ItemId>()

    /** Songs added to the library, or taken out, since it loaded. */
    private val inLibraryNow = mutableStateMapOf<ItemId, Boolean>()

    /** The songs of the user's playlists, to say which have a song already. */
    val contents = mutableStateMapOf<ItemId, Set<ItemId>>()

    /** Settings → Library. */
    val favouritesToLibrary get() = prefs.getBoolean("favourites-to-library", true)
    val playlistSongsToLibrary get() = prefs.getBoolean("playlist-songs-to-library", true)

    fun inLibrary(t: Track) = t.id.provider == ProviderKind.NAVIDROME || (inLibraryNow[t.id] ?: t.inLibrary)

    /** Whether `id` is a favourite; asked of the service the first time. */
    fun isFavourite(kind: FavouriteKind, id: ItemId, said: Boolean? = null): Boolean {
        favourites[id]?.let { return it }
        if (said != null) return said
        if (id.provider != ProviderKind.NAVIDROME && asking.add(id)) {
            scope.launch {
                runCatching { session.isFavourite(kind, id) }.getOrNull()?.let { favourites[id] = it }
            }
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
            } catch (e: MusicException) {
                undo()
                hud.show(failed, Icons.Alert)
            }
        }
    }

    // ---- Menus ----

    fun isStation(track: Track) = playback.isStation(track)

    fun trackMenu(track: Track, anchor: Rect, inPlaylist: Playlist? = null, onRemoved: () -> Unit = {}) {
        if (playback.isStation(track)) return
        val head = MenuHead(track.artwork, track.title, track.artists.joinToString(", "))
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
                            else -> null
                        },
                        MenuItem("Add to a Playlist…", Icons.Listplus) { addToPlaylists(listOf(track)) },
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
                )
            },
        )
    }

    fun albumMenu(album: Album, anchor: Rect) {
        val head = MenuHead(album.artwork, album.title, album.artists.joinToString(", "))
        val saved = album.added != null || album.id.id.startsWith("l.")
        menus.show(
            MenuSpec(anchor, head) {
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
                        if (!saved && album.id.provider == ProviderKind.APPLE_MUSIC) {
                            MenuItem("Add to Library", Icons.Pluscircle) { addAlbumToLibrary(album) }
                        } else {
                            null
                        },
                        MenuItem("Add to a Playlist…", Icons.Listplus) {
                            withAlbum(album) { sheet = SheetSpec.AddToPlaylists(it, songsCount(it.size), album.title, album.artwork) }
                        },
                    ),
                    listOfNotNull(
                        album.artistId?.let { id -> MenuItem("Go to Artist", Icons.Mic) { go(Route.Artist(id)) } },
                    ),
                )
            },
        )
    }

    fun playlistMenu(playlist: Playlist, anchor: Rect, onDeleted: () -> Unit = {}) {
        val head = MenuHead(playlist.artwork, if (playlist.favourites) "Favourite Songs" else playlist.name, "Playlist", star = playlist.favourites)
        val favouriteId = playlist.catalogId ?: playlist.id
        menus.show(
            MenuSpec(anchor, head) {
                val queue = listOf(
                    MenuItem("Play Next", Icons.Next) { withPlaylist(playlist) { playNext(it) } },
                    MenuItem("Add to Queue", Icons.Later) { withPlaylist(playlist) { addToQueue(it) } },
                )
                val favourite = !playlist.favourites && isFavourite(FavouriteKind.PLAYLIST, favouriteId, playlist.inFavourites)
                if (playlist.favourites) listOf(queue) else listOf(
                    queue,
                    listOfNotNull(
                        if (playlist.editable) MenuItem("Edit…", Icons.Pencil) { sheet = SheetSpec.PlaylistForm(playlist) } else null,
                        if (playlist.id.provider == ProviderKind.APPLE_MUSIC) {
                            MenuItem(if (favourite) "Remove from Favourites" else "Favourite", if (favourite) Icons.Starf else Icons.Star) {
                                togglePlaylistFavourite(playlist)
                            }
                        } else {
                            null
                        },
                    ),
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

    fun artistMenu(detail: io.github.honeyslatt.music.core.model.ArtistDetail, anchor: Rect) {
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

    fun isFavourite(artist: io.github.honeyslatt.music.core.model.Artist) =
        isFavourite(FavouriteKind.ARTIST, artist.id, artist.inFavourites)

    fun toggleFavourite(artist: io.github.honeyslatt.music.core.model.Artist) {
        val was = isFavourite(artist)
        favourites[artist.id] = !was
        hud.show(if (was) "Removed from Favourites" else "Favourited", if (was) Icons.Star else Icons.Starf)
        change("Couldn't change the favourites", undo = { favourites[artist.id] = was }) {
            session.setFavourite(FavouriteKind.ARTIST, artist.id, !was)
        }
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
                val tracks = session.stationTracks(station, STATION_SONGS)
                playback.play(tracks, 0)
                hud.show("${track.title} Station", Icons.Radio)
            } catch (e: MusicException) {
                hud.show("Couldn't start the station", Icons.Alert)
            }
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
        }
    }

    private fun addAlbumToLibrary(album: Album) {
        hud.show("Added to Library")
        change("Couldn't add to the library") { session.addToLibrary(FavouriteKind.ALBUM, album.catalogId ?: album.id) }
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
            change("Couldn't delete it", undo = { inLibraryNow.remove(track.id) }) { session.removeFromLibrary(id) }
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

    /** The user's playlists `tracks` can go in: theirs, of the songs' service. */
    fun editablePlaylists(tracks: List<Track>): List<Playlist> {
        val provider = tracks.firstOrNull()?.id?.provider ?: return emptyList()
        if (tracks.any { it.id.provider != provider }) return emptyList()
        return library.lists.value.playlists.items.filter { it.editable && it.id.provider == provider }
    }

    /** The playlists songs were last added to, most recent first. */
    fun recentPlaylists(tracks: List<Track>): List<Playlist> {
        val fit = editablePlaylists(tracks)
        return recent().mapNotNull { id -> fit.firstOrNull { it.id.id == id } }
    }

    private fun recent() = prefs.getString("recent-playlists", null)?.split('\n')?.filter { it.isNotEmpty() } ?: emptyList()

    private fun usedNow(playlists: List<Playlist>) {
        val ids = (playlists.map { it.id.id } + recent()).distinct().take(5)
        prefs.edit().putString("recent-playlists", ids.joinToString("\n")).apply()
    }

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
                session.addToPlaylist(p.id, tracks.map { it.id })
            }
        }
    }

    /** Songs put in a playlist join the library, unless the setting says not to. */
    private fun joinLibrary(tracks: List<Track>) {
        if (!playlistSongsToLibrary) return
        tracks.filter { it.id.provider == ProviderKind.APPLE_MUSIC && !inLibrary(it) }.forEach { t ->
            inLibraryNow[t.id] = true
            change("Couldn't add to the library", undo = { inLibraryNow.remove(t.id) }) { session.addToLibrary(FavouriteKind.SONG, t.id) }
        }
    }

    fun createPlaylist(name: String, description: String, tracks: List<Track>) {
        sheet = null
        joinLibrary(tracks)
        val provider = tracks.firstOrNull()?.id?.provider ?: ProviderKind.APPLE_MUSIC
        scope.launch {
            try {
                val made = session.createPlaylist(provider, name, description, tracks.map { it.id })
                library.playlistMade(made)
                contents[made.id] = tracks.map { it.id }.toSet()
                usedNow(listOf(made))
                hud.show(if (tracks.isEmpty()) "Created" else "Added to “$name”")
            } catch (e: MusicException) {
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

    fun confirmDelete(playlist: Playlist, onDeleted: () -> Unit = {}) {
        sheet = SheetSpec.Confirm(
            "Delete Playlist?",
            "“${playlist.name}” goes from your library on every device.",
            "Delete Playlist",
        ) {
            library.playlistDeleted(playlist.id)
            onDeleted()
            hud.show("Deleted", Icons.Trash)
            change("Couldn't delete the playlist", undo = { library.playlistMade(playlist) }) { session.deletePlaylist(playlist.id) }
        }
    }

    /** Apple names a playlist's row by the song's library id; SoundCloud by the song's. */
    private fun removeFromPlaylist(playlist: Playlist, track: Track, onRemoved: () -> Unit) {
        val entry = if (playlist.id.provider == ProviderKind.SOUND_CLOUD) track.id else track.libraryId ?: return
        onRemoved()
        contents[playlist.id]?.let { contents[playlist.id] = it - track.id }
        hud.show("Removed", Icons.Minuscircle)
        change("Couldn't remove it") { session.removeFromPlaylist(playlist.id, entry) }
    }
}

fun songsCount(n: Int) = if (n == 1) "1 song" else "$n songs"
