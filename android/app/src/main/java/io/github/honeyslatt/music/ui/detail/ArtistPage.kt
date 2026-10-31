package io.github.honeyslatt.music.ui.detail

import androidx.compose.runtime.remember
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.compose.foundation.border
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.lazy.itemsIndexed
import androidx.compose.ui.graphics.graphicsLayer
import io.github.honeyslatt.music.ui.components.ExplicitBadge
import io.github.honeyslatt.music.ui.components.anchor
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
import io.github.honeyslatt.music.core.model.Artwork
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
    val motion = d?.motion?.takeIf { c.animatedArtwork }
    // Under a video, the page takes the video's colours, so that it fades
    // into the page as Apple Music's does; else the photo's.
    val frame = d?.banner?.takeIf { motion != null }?.let { Artwork(it.replace("2400x1350", "{size}x{size}"), null) }
    val page = rememberPageColour(frame ?: d?.artist?.artwork)
    val actions = LocalActions.current
    val artist = d?.artist
    val favourite = artist != null && actions?.isFavourite(artist) == true
    if (io.github.honeyslatt.music.ui.theme.look.layout.header == io.github.honeyslatt.music.core.model.Header.BANNER) {
        SpotifyArtistPage(c, loaded, motion, page, favourite)
        return
    }
    DetailPage(
        c.nav,
        artist?.name.orEmpty(),
        page,
        listOf(BarAction(Icons.More, "More") { rect -> d?.takeIf { it.topSongs.isNotEmpty() }?.let { actions?.artistMenu(it, rect) } }),
    ) {
        item {
            Hero(
                artist,
                motion,
                favourite = favourite.takeIf { artist?.id?.provider == ProviderKind.APPLE_MUSIC },
                onFavourite = { artist?.let { actions?.toggleFavourite(it) } },
                onShuffle = { d?.topSongs?.takeIf { it.isNotEmpty() }?.let { c.playback.shuffle(it) } },
                onPlay = { d?.topSongs?.takeIf { it.isNotEmpty() }?.let { c.playback.play(it, 0) } },
            )
        }
        if (d == null) {
            item { Empty(loaded.listing(), "", "") }
            return@DetailPage
        }
        d.latest?.let { latest -> item { Latest(latest, onAdd = { actions?.addAlbumToLibrary(latest) }) { c.nav.push(Route.Album(latest.id)) } } }
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

/**
 * Spotify's artist page: the photo, or video, fading into the page with the
 * name large over its foot; Follow and the menu by Shuffle and Play; the
 * popular songs numbered, the popular releases listed, the discography by
 * chips, then the rest in shelves.
 */
@Composable
private fun SpotifyArtistPage(c: LibraryContext, loaded: Loading<ArtistDetail>, motion: String?, page: Color, following: Boolean) {
    val d = (loaded as? Loading.Ready)?.value
    val artist = d?.artist
    val actions = LocalActions.current
    var more by remember(artist?.id) { mutableStateOf(false) }
    var chosen by remember(artist?.id) { mutableStateOf(0) }
    DetailPage(
        c.nav,
        artist?.name.orEmpty(),
        page,
        listOf(BarAction(Icons.More, "More") { rect -> d?.takeIf { it.topSongs.isNotEmpty() }?.let { actions?.artistMenu(it, rect) } }),
    ) {
        item { BannerHero(artist, motion) }
        if (d == null) {
            item { Empty(loaded.listing(), "", "") }
            return@DetailPage
        }
        item {
            io.github.honeyslatt.music.ui.library.PlayRow(c, d.topSongs) {
                if (d.artist.id.provider == ProviderKind.APPLE_MUSIC) Follow(following) { actions?.toggleFavourite(d.artist) }
            }
        }
        if (d.topSongs.isNotEmpty()) {
            item { SectionHeader("Popular") }
            val shown = d.topSongs.take(if (more) 10 else 5)
            itemsIndexed(shown, key = { i, t -> "popular$i" + t.id.id }) { i, t -> PopularRow(i + 1, t) { c.playback.play(d.topSongs, i) } }
            if (d.topSongs.size > 5) {
                item {
                    Box(Modifier.padding(start = 12.dp, top = 6.dp).press(RoundedCornerShape(8.dp)) { more = !more }.padding(horizontal = 8.dp, vertical = 6.dp)) {
                        BasicText(if (more) "Show less" else "See more", style = geist(14.sp, FontWeight.Bold).copy(color = roles.sub))
                    }
                }
            }
        }
        // The latest, then the essentials, as the desktop's popular releases.
        val popular = (listOfNotNull(d.latest) + d.essentials.filter { it.id != d.latest?.id }).take(4)
        if (popular.isNotEmpty()) {
            item { SectionHeader("Popular releases") }
            itemsIndexed(popular, key = { i, a -> "release$i" + a.id.id }) { i, a ->
                ReleaseRow(a, latest = i == 0 && d.latest != null) { c.nav.push(Route.Album(a.id)) }
            }
        }
        val releases = listOf("Albums" to d.albums, "Singles and EPs" to d.singles, "Compilations" to d.compilations).filter { it.second.isNotEmpty() }
        if (releases.isNotEmpty()) {
            val pick = chosen.coerceAtMost(releases.lastIndex)
            item { SectionHeader("Discography") }
            item {
                Row(
                    Modifier.horizontalScroll(androidx.compose.foundation.rememberScrollState()).padding(start = 16.dp, end = 16.dp, bottom = 12.dp),
                    horizontalArrangement = Arrangement.spacedBy(8.dp),
                ) {
                    releases.forEachIndexed { i, (label, _) -> io.github.honeyslatt.music.ui.components.Chip(label, null, i == pick) { chosen = i } }
                }
            }
            item {
                LazyRow(contentPadding = PaddingValues(horizontal = 16.dp), horizontalArrangement = Arrangement.spacedBy(12.dp)) {
                    items(releases[pick].second) { a ->
                        io.github.honeyslatt.music.ui.components.CoverCard(a.artwork, bare(a.title), releaseLine(a), a.explicit, modifier = Modifier.width(150.dp)) {
                            c.nav.push(Route.Album(a.id))
                        }
                    }
                }
            }
        }
        videoShelf("Music Videos", d.videos)
        shelf("Featuring ${d.artist.name}", d.playlists.map { ShelfCover(it.artwork, it.name, it.curator) { c.nav.push(Route.Playlist(it.id)) } })
        artistShelf(c, "Fans also like", d.similar)
        shelf("Appears On", d.appearsOn.map { ShelfCover(it.artwork, it.title, it.artists.joinToString(", "), it.explicit) { c.nav.push(Route.Album(it.id)) } })
    }
}

/** Spotify's `2023 • Album`. */
private fun releaseLine(album: Album) = listOfNotNull(album.year?.toString(), album.kind()).joinToString(" • ")

/** A title without Apple's ` - Single` or ` - EP`, which the line under it says. */
private fun bare(title: String) = title.removeSuffix(" - Single").removeSuffix(" - EP")

/** The photo, or video, the width across, fading out at its foot into the
 *  page's colour behind; the name over it, as large as it reads. */
@Composable
private fun BannerHero(artist: Artist?, motion: String?) {
    BoxWithConstraints(Modifier.fillMaxWidth().height(400.dp)) {
        val width = maxWidth
        Box(
            Modifier.matchParentSize()
                .graphicsLayer { compositingStrategy = androidx.compose.ui.graphics.CompositingStrategy.Offscreen }
                .drawWithContent {
                    drawContent()
                    drawRect(
                        Brush.verticalGradient(0.55f to Color.Black, 1f to Color.Transparent),
                        blendMode = androidx.compose.ui.graphics.BlendMode.DstIn,
                    )
                },
        ) {
            Cover(artist?.artwork, width, RectangleShape, Modifier.fillMaxSize())
            if (motion != null) MotionArtwork(motion, Modifier.fillMaxSize(), crop = true)
        }
        val name = artist?.name.orEmpty()
        val size = when {
            name.length <= 12 -> 52.sp
            name.length <= 20 -> 42.sp
            else -> 34.sp
        }
        BasicText(
            name,
            Modifier.align(Alignment.BottomStart).padding(start = 16.dp, end = 16.dp, bottom = 8.dp),
            style = geist(size, FontWeight.Bold, -0.02f).copy(
                color = Color.White,
                shadow = androidx.compose.ui.graphics.Shadow(Color.Black.copy(alpha = 0.35f), blurRadius = 16f),
            ),
            maxLines = 2,
            overflow = androidx.compose.ui.text.style.TextOverflow.Ellipsis,
        )
    }
}

/** Follow, or Following: an outlined pill. */
@Composable
private fun Follow(following: Boolean, onClick: () -> Unit) {
    val shape = RoundedCornerShape(16.dp)
    Box(
        Modifier.padding(start = 9.dp, end = 4.dp).height(32.dp)
            .press(shape, shrink = 0.96f, onClick = onClick)
            .border(1.dp, roles.text.copy(alpha = 0.5f), shape)
            .padding(horizontal = 16.dp),
        contentAlignment = Alignment.Center,
    ) {
        BasicText(if (following) "Following" else "Follow", style = geist(14.sp, FontWeight.SemiBold).copy(color = roles.text))
    }
}

/** A popular song: its rank, its cover, its title, the E and its album; the menu. */
@Composable
private fun PopularRow(rank: Int, track: Track, onClick: () -> Unit) {
    val now = io.github.honeyslatt.music.ui.components.LocalNowPlaying.current.id == track.id
    val menu = LocalActions.current
    val anchor = io.github.honeyslatt.music.ui.components.rememberAnchor()
    val more = io.github.honeyslatt.music.ui.components.rememberAnchor()
    Row(
        Modifier.fillMaxWidth().height(64.dp).anchor(anchor)
            .press(onLongClick = { menu?.trackMenu(track, anchor.rect) }, onClick = onClick)
            .padding(start = 16.dp, end = 4.dp),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(14.dp),
    ) {
        BasicText(
            rank.toString(),
            Modifier.width(18.dp),
            style = geist(15.sp).copy(color = if (now) roles.tint else roles.sub, textAlign = androidx.compose.ui.text.style.TextAlign.Center),
        )
        Cover(track.artwork, 48.dp, RoundedCornerShape(2.dp), Modifier.size(48.dp))
        Column(Modifier.weight(1f)) {
            Line(track.title, geist(16.sp).copy(color = if (now) roles.tint else roles.text))
            Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(5.dp)) {
                if (track.explicit) ExplicitBadge()
                track.album?.let { Line(bare(it),geist(13.5.sp).copy(color = roles.sub)) }
            }
        }
        Box(Modifier.size(44.dp).anchor(more).press(CircleShape) { menu?.trackMenu(track, more.rect) }, contentAlignment = Alignment.Center) {
            Glyph(io.github.honeyslatt.music.ui.MoreIcons.MoreVertical, roles.sub, 20.dp)
        }
    }
}

/** A popular release: its cover, its title, `Latest release • Single` or `2023 • Album`. */
@Composable
private fun ReleaseRow(album: Album, latest: Boolean, onClick: () -> Unit) {
    Row(
        Modifier.fillMaxWidth().press(onClick = onClick).padding(horizontal = 16.dp, vertical = 8.dp),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(14.dp),
    ) {
        Cover(album.artwork, 76.dp, RoundedCornerShape(2.dp), Modifier.size(76.dp))
        Column(Modifier.weight(1f)) {
            BasicText(
                bare(album.title),
                style = geist(16.sp, FontWeight.Bold).copy(color = roles.text),
                maxLines = 2,
                overflow = androidx.compose.ui.text.style.TextOverflow.Ellipsis,
            )
            Row(Modifier.padding(top = 3.dp), verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(5.dp)) {
                if (album.explicit) ExplicitBadge()
                Line(if (latest) "Latest release • ${album.kind()}" else releaseLine(album), geist(13.5.sp).copy(color = roles.sub))
            }
        }
    }
}

/**
 * The artist's video, or photo, the page's width across and fading into
 * the page; their name centred over its foot, and under it Shuffle, Play
 * and Favourite, as Apple Music has them.
 */
@Composable
private fun Hero(
    artist: Artist?,
    motion: String?,
    favourite: Boolean?,
    onFavourite: () -> Unit,
    onShuffle: () -> Unit,
    onPlay: () -> Unit,
) {
    val page = roles.base
    BoxWithConstraints(Modifier.fillMaxWidth()) {
        // A video stands taller than a photo, cropped to its middle.
        val width = maxWidth
        val height = if (motion != null) width * 1.3f else 420.dp
        Box(Modifier.fillMaxWidth().height(height)) {
            Box(
                Modifier.matchParentSize().drawWithContent {
                    drawContent()
                    drawRect(
                        Brush.verticalGradient(
                            0f to Color.Black.copy(alpha = 0.22f),
                            0.22f to Color.Transparent,
                            0.5f to Color.Transparent,
                            0.78f to page.copy(alpha = 0.55f),
                            0.93f to page,
                            1f to page,
                        ),
                    )
                },
            ) {
                Cover(artist?.artwork, width, RectangleShape, Modifier.fillMaxSize())
                if (motion != null) MotionArtwork(motion, Modifier.fillMaxSize(), crop = true)
            }
            Column(
                Modifier.align(Alignment.BottomCenter).fillMaxWidth().padding(horizontal = 20.dp).padding(bottom = 6.dp),
                horizontalAlignment = Alignment.CenterHorizontally,
            ) {
                Line(artist?.name.orEmpty(), geist(40.sp, FontWeight.Bold, -0.025f).copy(color = roles.text))
                Row(
                    Modifier.padding(top = 18.dp),
                    verticalAlignment = Alignment.CenterVertically,
                    horizontalArrangement = Arrangement.spacedBy(26.dp),
                ) {
                    Round(Icons.Shuffle, 54.dp, roles.text.copy(alpha = 0.14f), roles.text, 22.dp, onShuffle)
                    Round(Icons.Play, 78.dp, Color.White, Color.Black, 34.dp, onPlay)
                    if (favourite != null) {
                        Round(if (favourite) Icons.Starf else Icons.Star, 54.dp, roles.text.copy(alpha = 0.14f), roles.text, 22.dp, onFavourite)
                    } else {
                        Spacer(Modifier.size(54.dp))
                    }
                }
            }
        }
    }
}

/** A round button: `icon` on `fill`. */
@Composable
private fun Round(icon: androidx.compose.ui.graphics.vector.ImageVector, size: androidx.compose.ui.unit.Dp, fill: Color, tint: Color, glyph: androidx.compose.ui.unit.Dp, onClick: () -> Unit) {
    Box(
        Modifier.size(size).press(CircleShape, shrink = 0.92f, onClick = onClick).background(fill, CircleShape),
        contentAlignment = Alignment.Center,
    ) { Glyph(icon, tint, glyph) }
}

/** The latest release in a card: its date, title and size, and + to add it. */
@Composable
private fun Latest(album: Album, onAdd: () -> Unit, onClick: () -> Unit) {
    val card = RoundedCornerShape(18.dp)
    Row(
        Modifier
            .fillMaxWidth()
            .padding(horizontal = 20.dp)
            .padding(top = 22.dp)
            .press(card, shrink = 0.98f, onClick = onClick)
            .background(roles.text.copy(alpha = 0.07f), card)
            .padding(14.dp),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(14.dp),
    ) {
        Cover(album.artwork, 100.dp, RoundedCornerShape(10.dp), Modifier.size(100.dp))
        Column(Modifier.weight(1f)) {
            BasicText(shortDate(album.released) ?: album.year?.toString().orEmpty(), style = geist(13.sp, FontWeight.Medium).copy(color = roles.sub))
            BasicText(
                album.title,
                Modifier.padding(vertical = 2.dp),
                style = geist(16.5.sp, FontWeight.SemiBold).copy(color = roles.text),
                maxLines = 2,
                overflow = androidx.compose.ui.text.style.TextOverflow.Ellipsis,
            )
            Line(album.trackCount?.let { songs(it.toInt()) } ?: album.kind(), geist(13.5.sp).copy(color = roles.sub))
        }
        Round(Icons.Plus, 38.dp, roles.text.copy(alpha = 0.12f), roles.text, 18.dp, onAdd)
    }
}

/** `8 Jul 2026`, as Apple Music dates a release on an artist's page. */
private fun shortDate(released: String?): String? = released?.let {
    runCatching {
        java.time.LocalDate.parse(it).format(java.time.format.DateTimeFormatter.ofPattern("d MMM yyyy", java.util.Locale.ENGLISH))
    }.getOrNull()
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
