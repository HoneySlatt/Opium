package io.github.honeyslatt.music.ui.detail

import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.lazy.itemsIndexed
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.produceState
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.fillMaxWidth
import io.github.honeyslatt.music.ui.components.press
import io.github.honeyslatt.music.ui.theme.geist
import io.github.honeyslatt.music.ui.theme.roles
import io.github.honeyslatt.music.core.AlbumDetail
import io.github.honeyslatt.music.core.model.Album
import io.github.honeyslatt.music.core.model.ItemId
import io.github.honeyslatt.music.core.model.ProviderKind
import io.github.honeyslatt.music.ui.Icons
import io.github.honeyslatt.music.ui.components.BarAction
import io.github.honeyslatt.music.ui.components.byline
import io.github.honeyslatt.music.ui.library.Empty
import io.github.honeyslatt.music.ui.library.LibraryContext
import io.github.honeyslatt.music.ui.library.PlayShuffle
import io.github.honeyslatt.music.data.Listing
import io.github.honeyslatt.music.ui.nav.Route

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
    val tracks = detail?.tracks.orEmpty()
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
    DetailPage(c.nav, album?.title.orEmpty(), page, listOf(BarAction(Icons.More, "More") {})) {
        item {
            DetailHeader(
                artwork = album?.artwork,
                motion = album?.motion?.takeIf { c.animatedArtwork },
                title = album?.title.orEmpty(),
                byline = album?.artists?.joinToString(", "),
                onByline = album?.artistId?.let { artist -> { c.nav.push(Route.Artist(artist)) } },
                meta = {
                    if (album != null) Meta(listOfNotNull(album.kind(), album.year?.toString()).joinToString(" · "))
                    if (quality != null) Tag(quality.label(), quality.lossless())
                },
            )
        }
        item { PlayShuffle(tracks.isNotEmpty(), { c.playback.play(tracks, 0) }, { c.playback.shuffle(tracks) }) }
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
        io.github.honeyslatt.music.ui.components.Line(
            "Show Full Album",
            geist(15.sp, androidx.compose.ui.text.font.FontWeight.SemiBold).copy(color = roles.tint),
        )
    }
}
