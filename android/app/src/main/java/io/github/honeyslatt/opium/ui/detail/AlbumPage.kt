package io.github.honeyslatt.opium.ui.detail

import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.lazy.itemsIndexed
import androidx.compose.runtime.Composable
import androidx.compose.runtime.setValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import io.github.honeyslatt.opium.core.model.AlbumExtras
import io.github.honeyslatt.opium.ui.actions.LocalActions
import androidx.compose.runtime.getValue
import androidx.compose.runtime.produceState
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.fillMaxWidth
import io.github.honeyslatt.opium.ui.components.press
import io.github.honeyslatt.opium.ui.theme.geist
import io.github.honeyslatt.opium.ui.theme.roles
import io.github.honeyslatt.opium.core.AlbumDetail
import io.github.honeyslatt.opium.core.model.Album
import io.github.honeyslatt.opium.core.model.ItemId
import io.github.honeyslatt.opium.core.model.Track
import io.github.honeyslatt.opium.core.model.ProviderKind
import io.github.honeyslatt.opium.ui.Icons
import io.github.honeyslatt.opium.ui.components.BarAction
import io.github.honeyslatt.opium.ui.components.byline
import io.github.honeyslatt.opium.ui.components.page
import io.github.honeyslatt.opium.ui.library.Empty
import io.github.honeyslatt.opium.ui.library.LibraryContext
import io.github.honeyslatt.opium.ui.library.PlayShuffle
import io.github.honeyslatt.opium.data.Listing
import io.github.honeyslatt.opium.ui.nav.Route

/** What a page is waiting for. */
sealed interface Loading<out T> {
    data object Pending : Loading<Nothing>
    data class Ready<T>(val value: T) : Loading<T>
    data class Failed(val reason: String) : Loading<Nothing>
}

fun <T> Loading<T>.listing(): Listing<T> = when (this) {
    Loading.Pending -> Listing()
    is Loading.Ready -> Listing(done = true)
    is Loading.Failed -> Listing(done = true, error = reason)
}

suspend fun <T> load(get: suspend () -> T): Loading<T> =
    try {
        Loading.Ready(get())
    } catch (e: kotlinx.coroutines.CancellationException) {
        // Called off (typed on, left the page): not a failure to show.
        throw e
    } catch (e: Exception) {
        Loading.Failed(e.message ?: e.toString())
    }

/** Single, EP or Album, as the title tells. */
fun Album.kind() = when {
    title.endsWith(" - Single") -> "Single"
    title.endsWith(" - EP") -> "EP"
    else -> "Album"
}

@Composable
fun AlbumPage(c: LibraryContext, id: ItemId) {
    val loaded by produceState<Loading<AlbumDetail>>(Loading.Pending, id) { value = load { c.core.session.album(id) } }
    val detail = (loaded as? Loading.Ready)?.value
    val album = detail?.album
    // Whole from its copies on the library's other services: the songs one
    // lacks come from another, as on the desktop.
    val others = c.lists.albums.items
    val whole by produceState<List<Track>?>(null, detail, others.size) {
        val d = detail ?: return@produceState
        val twins = io.github.honeyslatt.opium.core.albumTwins(d.album, others).map { others[it.toInt()].id }
        if (twins.isEmpty()) return@produceState
        val copies = listOf(d.tracks) + twins.mapNotNull { runCatching { c.core.session.album(it).tracks }.getOrNull() }
        if (copies.size > 1) value = io.github.honeyslatt.opium.core.wholeAlbum(copies)
    }
    // Offline, a downloaded album's page from its songs kept.
    val downloads = (androidx.compose.ui.platform.LocalContext.current.applicationContext as io.github.honeyslatt.opium.OpiumApp).downloads
    val keptSongs by downloads.kept.collectAsStateWithLifecycle()
    val offline = if (loaded is Loading.Failed) {
        keptSongs.map { it.track }.filter { it.albumId == id }.sortedWith(compareBy({ it.discNumber ?: 1u }, { it.trackNumber ?: 0u }))
    } else {
        emptyList()
    }
    val tracks = whole ?: detail?.tracks ?: offline
    // More by the artist, from the catalog.
    val more by produceState<List<Album>>(emptyList(), album?.artistId) {
        val artist = album?.artistId?.takeIf { it.provider == ProviderKind.APPLE_MUSIC } ?: return@produceState
        value = (load { c.core.session.artist(artist) } as? Loading.Ready)?.value
            ?.let { it.albums + it.singles }
            ?.filter { it.id != album.id && it.title != album.title }
            .orEmpty()
    }
    val page = rememberPageColour(album?.artwork)
    val quality = tracks.firstNotNullOfOrNull { it.quality }
    val actions = LocalActions.current
    // Its other editions and its songs' videos, when Apple has them.
    val extras by produceState<AlbumExtras?>(null, id) {
        if (id.provider == ProviderKind.APPLE_MUSIC) value = runCatching { c.core.session.albumExtras(id) }.getOrNull()
    }
    // A catalogue album gets a + that adds it to the library.
    var added by remember(id) { mutableStateOf(false) }
    val saved = album == null || album.added != null || album.id.id.startsWith("l.") || album.id.provider != ProviderKind.APPLE_MUSIC
    val bar = listOfNotNull(
        if (!saved) {
            BarAction(if (added) Icons.Check else Icons.Plus, "Add to Library") {
                if (!added) {
                    added = true
                    actions?.addAlbumToLibrary(album!!)
                }
            }
        } else {
            null
        },
        downloadAction(tracks, album?.title ?: offline.firstOrNull()?.album.orEmpty()),
        BarAction(Icons.More, "More") { rect -> album?.let { actions?.albumMenu(it, rect, onDeleted = { c.nav.pop() }) } },
    )
    val first = offline.firstOrNull()
    DetailPage(c.nav, album?.title ?: first?.album.orEmpty(), page, bar) {
        item {
            DetailHeader(
                artwork = album?.artwork ?: first?.artwork,
                motion = album?.motion?.takeIf { c.animatedArtwork },
                title = album?.title ?: first?.album.orEmpty(),
                byline = album?.artists?.joinToString(", ") ?: first?.artists?.joinToString(", "),
                onByline = album?.artistId?.let { artist -> { c.nav.push(Route.Artist(artist)) } },
                meta = {
                    if (album != null) Meta(listOfNotNull(album.kind(), album.year?.toString()).joinToString(" · "))
                    quality?.badge()?.let { Tag(it, quality.lossless()) }
                },
            )
        }
        item { io.github.honeyslatt.opium.ui.library.PlayRow(c, tracks) }
        item { Spacer(Modifier.height(12.dp)) }
        if (tracks.isEmpty()) item { Empty(loaded.listing(), "No songs", "This album has no songs here.") }
        itemsIndexed(tracks, key = { i, t -> "$i" + t.id.id }) { i, track ->
            AlbumTrackRow(track.trackNumber?.toInt() ?: (i + 1), track, last = i == tracks.lastIndex) { c.playback.play(tracks, i) }
        }
        // A library album with only some of its songs: the catalog's whole.
        album?.catalogId?.takeIf { it != album.id }?.let { whole ->
            item { FullAlbum { c.nav.push(Route.Album(whole)) } }
        }
        if (album != null && tracks.isNotEmpty()) {
            item {
                Foot(listOfNotNull(releaseDate(album.released)?.let { "Released $it" }, countAndLength(tracks), album.copyright))
            }
        }
        extras?.let { e ->
            shelf("Other Versions", e.otherVersions.map { ShelfCover(it.artwork, it.title, it.byline(), it.explicit) { c.nav.push(Route.Album(it.id)) } })
            videoShelf("Music Videos", e.videos)
        }
        shelf(
            "More by ${album?.artists?.firstOrNull().orEmpty()}",
            more.map { ShelfCover(it.artwork, it.title, it.year?.toString(), it.explicit) { c.nav.push(Route.Album(it.id)) } },
        )
    }
}

/** An album's line under its title in a shelf: its year. */
fun Album.yearLine() = year?.toString() ?: byline()

@Composable
private fun FullAlbum(onClick: () -> Unit) {
    val shape = androidx.compose.foundation.shape.RoundedCornerShape(12.dp)
    androidx.compose.foundation.layout.Box(
        Modifier
            .fillMaxWidth()
            .padding(start = 20.dp, end = 20.dp, top = 16.dp)
            .height(46.dp)
            .press(shape, shrink = 0.97f, onClick = onClick)
            .background(roles.surface, shape),
        contentAlignment = androidx.compose.ui.Alignment.Center,
    ) {
        io.github.honeyslatt.opium.ui.components.Line(
            "Show Full Album",
            geist(15.sp, androidx.compose.ui.text.font.FontWeight.SemiBold).copy(color = roles.tint),
        )
    }
}

/**
 * A page's download button: Download, Stop downloading while its songs
 * come, Remove Download (asked first) once all are kept; none when none of
 * its songs can be downloaded.
 */
@Composable
fun downloadAction(tracks: List<Track>, title: String, playlist: ItemId? = null): BarAction? {
    val state = io.github.honeyslatt.opium.ui.components.LocalDownloads.current.page(tracks) ?: return null
    val actions = LocalActions.current ?: return null
    val downloads = (androidx.compose.ui.platform.LocalContext.current.applicationContext as io.github.honeyslatt.opium.OpiumApp).downloads
    val label = when (state) {
        io.github.honeyslatt.opium.ui.components.PageDownload.Offer -> "Download"
        is io.github.honeyslatt.opium.ui.components.PageDownload.Coming -> "Stop Downloading"
        io.github.honeyslatt.opium.ui.components.PageDownload.Kept -> "Remove Download"
    }
    return BarAction(io.github.honeyslatt.opium.ui.MoreIcons.ArrowDown, label, glyph = { size, idle -> io.github.honeyslatt.opium.ui.components.DownloadGlyph(state, size, idle) }) {
        when (state) {
            io.github.honeyslatt.opium.ui.components.PageDownload.Offer -> {
                playlist?.let { downloads.keepPlaylist(it, tracks) }
                actions.download(tracks)
            }
            is io.github.honeyslatt.opium.ui.components.PageDownload.Coming -> actions.stopDownloading(tracks)
            io.github.honeyslatt.opium.ui.components.PageDownload.Kept -> actions.confirmRemoveDownload(tracks, title)
        }
    }
}
