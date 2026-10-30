package io.github.honeyslatt.music.ui.detail

import androidx.compose.runtime.remember
import kotlinx.coroutines.awaitAll
import kotlinx.coroutines.async
import androidx.compose.foundation.background
import androidx.compose.foundation.gestures.snapping.SnapPosition
import androidx.compose.foundation.gestures.snapping.rememberSnapFlingBehavior
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyListScope
import androidx.compose.foundation.lazy.LazyRow
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.lazy.rememberLazyListState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.BasicText
import androidx.compose.runtime.Composable
import io.github.honeyslatt.music.ui.actions.LocalActions
import io.github.honeyslatt.music.core.model.ProviderKind
import androidx.compose.runtime.getValue
import androidx.compose.runtime.produceState
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.drawWithContent
import androidx.compose.ui.draw.shadow
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.RectangleShape
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import io.github.honeyslatt.music.core.model.Album
import io.github.honeyslatt.music.core.model.Artist
import io.github.honeyslatt.music.core.model.ArtistDetail
import io.github.honeyslatt.music.core.model.ItemId
import io.github.honeyslatt.music.core.model.Track
import io.github.honeyslatt.music.ui.Icons
import io.github.honeyslatt.music.ui.components.BarAction
import io.github.honeyslatt.music.ui.components.Cover
import io.github.honeyslatt.music.ui.components.Glyph
import io.github.honeyslatt.music.ui.components.Line
import io.github.honeyslatt.music.ui.components.SectionHeader
import io.github.honeyslatt.music.ui.components.TrackRow
import io.github.honeyslatt.music.ui.components.press
import io.github.honeyslatt.music.ui.library.Empty
import io.github.honeyslatt.music.ui.library.LibraryContext
import io.github.honeyslatt.music.ui.library.PlayShuffle
import io.github.honeyslatt.music.ui.library.songs
import io.github.honeyslatt.music.ui.nav.Route
import io.github.honeyslatt.music.ui.theme.geist
import io.github.honeyslatt.music.ui.theme.roles

/** An artist's Apple Music profile, as iOS 27 lays it out. */
@Composable
fun ArtistPage(c: LibraryContext, id: ItemId) {
    val loaded by produceState<Loading<ArtistDetail>>(Loading.Pending, id) { value = load { c.core.session.artist(id) } }
    val d = (loaded as? Loading.Ready)?.value
    val page = rememberPageColour(d?.artist?.artwork)
    val actions = LocalActions.current
    val artist = d?.artist
    val favourite = artist != null && actions?.isFavourite(artist) == true
    DetailPage(
        c.nav,
        artist?.name.orEmpty(),
        page,
        listOfNotNull(
            if (artist?.id?.provider == ProviderKind.APPLE_MUSIC) {
                BarAction(if (favourite) Icons.Starf else Icons.Star, "Favourite") { actions?.toggleFavourite(artist) }
            } else {
                null
            },
            BarAction(Icons.More, "More") { rect -> d?.takeIf { it.topSongs.isNotEmpty() }?.let { actions?.artistMenu(it, rect) } },
        ),
    ) {
        item { Hero(d?.artist, d?.motion?.takeIf { c.animatedArtwork }) { d?.topSongs?.let { c.playback.shuffle(it) } } }
        if (d == null) {
            item { Empty(loaded.listing(), "", "") }
            return@DetailPage
        }
        d.latest?.let { latest -> item { SectionHeader("Latest Release") }; item { Latest(latest) { c.nav.push(Route.Album(latest.id)) } } }
        songColumns("Top Songs", d.topSongs) { i -> c.playback.play(d.topSongs, i) }
        albumShelf(c, "Albums", d.albums)
        albumShelf(c, "Singles & EPs", d.singles)
        videoShelf("Music Videos", d.videos)
        albumShelf(c, "Appears On", d.appearsOn)
        shelf("Artist Playlists", d.playlists.map { ShelfCover(it.artwork, it.name, it.curator) { c.nav.push(Route.Playlist(it.id)) } })
        albumShelf(c, "Essential Albums", d.essentials)
        artistShelf(c, "Similar Artists", d.similar)
    }
}

private fun LazyListScope.albumShelf(c: LibraryContext, title: String, albums: List<Album>) =
    shelf(title, albums.map { ShelfCover(it.artwork, it.title, it.year?.toString(), it.explicit) { c.nav.push(Route.Album(it.id)) } })

private fun LazyListScope.artistShelf(c: LibraryContext, title: String, artists: List<Artist>) {
    if (artists.isEmpty()) return
    item { SectionHeader(title) }
    item {
        LazyRow(contentPadding = PaddingValues(horizontal = 20.dp), horizontalArrangement = Arrangement.spacedBy(12.dp)) {
            items(artists) { a ->
                Column(Modifier.width(124.dp).press(shrink = 0.97f) { c.nav.push(Route.Artist(a.id)) }, horizontalAlignment = Alignment.CenterHorizontally) {
                    Cover(a.artwork, 124.dp, CircleShape, Modifier.size(124.dp))
                    Spacer(Modifier.height(7.dp))
                    Line(a.name, geist(14.sp, FontWeight.Medium).copy(color = roles.text))
                }
            }
        }
    }
}

/** The 420 dp picture, or video, with the name over it and the shuffle button. */
@Composable
private fun Hero(artist: Artist?, motion: String?, onShuffle: () -> Unit) {
    val page = roles.base
    Box(Modifier.fillMaxWidth().height(420.dp)) {
        BoxWithConstraints(
            Modifier.matchParentSize().drawWithContent {
                drawContent()
                drawRect(
                    Brush.verticalGradient(
                        0f to Color.Black.copy(alpha = 0.25f),
                        0.3f to Color.Transparent,
                        0.5f to Color.Transparent,
                        0.86f to Color.Black.copy(alpha = 0.6f),
                        1f to page,
                    ),
                )
            },
        ) {
            Cover(artist?.artwork, maxWidth, RectangleShape, Modifier.fillMaxSize())
            if (motion != null) MotionArtwork(motion, Modifier.fillMaxSize())
        }
        Line(
            artist?.name.orEmpty(),
            geist(38.sp, FontWeight.Bold, -0.025f).copy(color = Color.White),
            Modifier.align(Alignment.BottomStart).padding(start = 20.dp, end = 96.dp, bottom = 22.dp),
        )
        Box(
            Modifier
                .align(Alignment.BottomEnd)
                .padding(end = 20.dp, bottom = 14.dp)
                .size(60.dp)
                .shadow(10.dp, CircleShape)
                .press(CircleShape, shrink = 0.92f, onClick = onShuffle)
                .background(roles.accent, CircleShape),
            contentAlignment = Alignment.Center,
        ) { Glyph(Icons.Shuffle, Color.White, 26.dp) }
    }
}

@Composable
private fun Latest(album: Album, onClick: () -> Unit) {
    Row(
        Modifier.fillMaxWidth().press(onClick = onClick).padding(horizontal = 20.dp),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(14.dp),
    ) {
        Cover(album.artwork, 104.dp, RoundedCornerShape(10.dp), Modifier.size(104.dp))
        Column {
            BasicText(releaseDate(album.released)?.uppercase() ?: album.year?.toString().orEmpty(), style = geist(12.sp, FontWeight.SemiBold, 0.04f).copy(color = roles.sub))
            Line(album.title, geist(16.sp, FontWeight.SemiBold).copy(color = roles.text), Modifier.padding(vertical = 2.dp))
            Line(listOfNotNull(album.kind(), album.trackCount?.let { songs(it.toInt()) }).joinToString(" · "), geist(13.5.sp).copy(color = roles.sub))
        }
    }
}

/** Songs in columns of four that page sideways: Top Songs, Best New Songs. */
fun LazyListScope.songColumns(title: String, tracks: List<Track>, onOpen: (() -> Unit)? = null, play: (Int) -> Unit) {
    if (tracks.isEmpty()) return
    item { SectionHeader(title, onOpen = onOpen) }
    item {
        val state = rememberLazyListState()
        BoxWithConstraints(Modifier.fillMaxWidth()) {
            val column = maxWidth - 44.dp
            LazyRow(
                state = state,
                flingBehavior = rememberSnapFlingBehavior(state, SnapPosition.Start),
                contentPadding = PaddingValues(start = 0.dp, end = 20.dp),
                horizontalArrangement = Arrangement.spacedBy(0.dp),
            ) {
                items(tracks.withIndex().chunked(4)) { four ->
                    Column(Modifier.width(column)) { four.forEach { (i, t) -> TrackRow(t) { play(i) } } }
                }
            }
        }
    }
}

/** An artist as the library has them: a round photo, their name opening their profile. */
@Composable
fun LibraryArtistPage(c: LibraryContext, artist: Artist) {
    // Their releases on every service the library has them on, and under
    // each of their names there, as on the desktop: each asked by the
    // library's own id, which Apple's library artists need.
    val name = io.github.honeyslatt.music.ui.components.fold(artist.name.trim())
    val twins = remember(artist, c.lists.artists.items) {
        (listOf(artist) + c.lists.artists.items.filter { it.id == artist.id || io.github.honeyslatt.music.ui.components.fold(it.name.trim()) == name })
            .map { it.libraryId ?: it.id }
            .distinct()
    }
    val albums by produceState<Loading<List<Album>>>(Loading.Pending, twins) {
        value = load {
            val found = kotlinx.coroutines.coroutineScope {
                twins.map { id -> async { runCatching { c.core.session.libraryArtistAlbums(id) }.getOrDefault(emptyList()) } }.awaitAll().flatten()
            }
            val out = io.github.honeyslatt.music.core.albumsLeftOut(found).toSet()
            found.filterIndexed { i, _ -> i.toUInt() !in out }.distinctBy { it.id }
        }
    }
    val all = (albums as? Loading.Ready)?.value.orEmpty().sortedByDescending { it.released ?: it.year?.toString() }
    val (short, long) = all.partition { it.kind() != "Album" }
    io.github.honeyslatt.music.ui.components.ListPage(c.nav, artist.name, largeTitle = false) {
        item {
            Column(Modifier.fillMaxWidth(), horizontalAlignment = Alignment.CenterHorizontally) {
                Cover(artist.artwork, 180.dp, CircleShape, Modifier.size(180.dp))
                Row(
                    Modifier.padding(top = 14.dp).press(RoundedCornerShape(8.dp)) { c.nav.push(Route.Artist(artist.id)) }.padding(horizontal = 6.dp),
                    verticalAlignment = Alignment.CenterVertically,
                    horizontalArrangement = Arrangement.spacedBy(4.dp),
                ) {
                    Line(artist.name, geist(26.sp, FontWeight.Bold, -0.01f).copy(color = roles.text))
                    Glyph(Icons.Right, roles.muted, 22.dp)
                }
            }
        }
        item { PlayShuffle(enabled = all.isNotEmpty()) }
        if (all.isEmpty()) item { Empty(albums.listing(), "No albums", "Nothing of theirs in the library yet.") }
        albumShelf(c, "Albums", long)
        albumShelf(c, "Singles & EPs", short)
    }
}
