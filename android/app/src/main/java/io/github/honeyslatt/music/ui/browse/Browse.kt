package io.github.honeyslatt.music.ui.browse

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.aspectRatio
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
import androidx.compose.foundation.gestures.snapping.SnapPosition
import androidx.compose.foundation.gestures.snapping.rememberSnapFlingBehavior
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.BasicText
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.luminance
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.media3.common.util.UnstableApi
import coil3.compose.AsyncImage
import io.github.honeyslatt.music.core.model.Collection
import io.github.honeyslatt.music.core.model.Pick
import io.github.honeyslatt.music.core.model.Shelf
import io.github.honeyslatt.music.core.model.Station
import io.github.honeyslatt.music.core.model.StationKind
import io.github.honeyslatt.music.data.Core
import io.github.honeyslatt.music.data.Listing
import io.github.honeyslatt.music.ui.actions.LocalActions
import io.github.honeyslatt.music.ui.components.CoverCard
import io.github.honeyslatt.music.ui.components.Line
import io.github.honeyslatt.music.ui.components.ListPage
import io.github.honeyslatt.music.ui.components.SectionHeader
import io.github.honeyslatt.music.ui.components.at
import io.github.honeyslatt.music.ui.components.gridCell
import io.github.honeyslatt.music.ui.components.press
import io.github.honeyslatt.music.ui.detail.Loading
import io.github.honeyslatt.music.ui.detail.listing
import io.github.honeyslatt.music.ui.detail.load
import io.github.honeyslatt.music.ui.detail.songColumns
import io.github.honeyslatt.music.ui.library.Empty
import io.github.honeyslatt.music.ui.library.LibraryContext
import io.github.honeyslatt.music.ui.nav.Route
import io.github.honeyslatt.music.ui.theme.geist
import io.github.honeyslatt.music.ui.theme.roles
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.launch

/** Apple's Home, New and Radio pages: asked for once, and again after half an hour. */
class Browse(private val core: Core, private val scope: CoroutineScope) {
    enum class Page { Home, New, Radio }

    private val pages = Page.entries.associateWith { MutableStateFlow<Loading<List<Shelf>>>(Loading.Pending) }
    private val asked = mutableMapOf<Page, Long>()

    fun shelves(page: Page): StateFlow<Loading<List<Shelf>>> {
        val now = System.currentTimeMillis()
        val stale = asked[page]?.let { now - it > 30 * 60_000 } ?: true
        if (stale || pages.getValue(page).value is Loading.Failed) {
            asked[page] = now
            scope.launch {
                val got = load {
                    when (page) {
                        Page.Home -> core.session.home()
                        Page.New -> core.session.newMusic()
                        Page.Radio -> core.session.radio()
                    }
                }
                // A failure keeps what was there.
                if (got is Loading.Ready || pages.getValue(page).value !is Loading.Ready) pages.getValue(page).value = got
            }
        }
        return pages.getValue(page)
    }
}

/** How a page shows its hero shelf. */
private enum class Hero { Picks, Editorial, Stations }

/** A genre's charts: its top songs, albums and playlists. */
@Composable
fun GenrePage(c: LibraryContext, genre: io.github.honeyslatt.music.core.model.Genre) {
    val charts by androidx.compose.runtime.produceState<Loading<List<Shelf>>>(Loading.Pending, genre.id) {
        value = load { c.core.session.genreCharts(genre.id) }
    }
    ListPage(c.nav, genre.name) { shelves(c, charts, Hero.Editorial) }
}

/**
 * Spotify's Home: the account and chips at the top, Home, New and Radio
 * being one tab, as its All, Music and Podcasts.
 */
@Composable
fun SpotifyHome(c: LibraryContext, browse: Browse, account: @Composable () -> Unit) {
    var which by androidx.compose.runtime.saveable.rememberSaveable { androidx.compose.runtime.mutableIntStateOf(0) }
    val chips: @Composable () -> Unit = {
        androidx.compose.foundation.layout.Row(
            Modifier.padding(start = 16.dp, end = 16.dp, top = 8.dp, bottom = 14.dp),
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = androidx.compose.foundation.layout.Arrangement.spacedBy(8.dp),
        ) {
            account()
            listOf("Home", "New", "Radio").forEachIndexed { i, label ->
                val on = which == i
                Box(
                    Modifier
                        .height(34.dp)
                        .press(androidx.compose.foundation.shape.RoundedCornerShape(17.dp)) { which = i }
                        .background(if (on) roles.accent else roles.surface2, androidx.compose.foundation.shape.RoundedCornerShape(17.dp))
                        .padding(horizontal = 16.dp),
                    contentAlignment = Alignment.Center,
                ) {
                    BasicText(label, style = geist(14.sp, FontWeight.Medium).copy(color = if (on) roles.onAccent else roles.text))
                }
            }
        }
    }
    androidx.compose.runtime.CompositionLocalProvider(io.github.honeyslatt.music.ui.components.LocalTitleChips provides chips) {
        when (which) {
            0 -> HomePage(c, browse, account)
            1 -> NewPage(c, browse, account)
            else -> RadioPage(c, browse, account)
        }
    }
}

@Composable
fun HomePage(c: LibraryContext, browse: Browse, account: @Composable () -> Unit) {
    val shelves by browse.shelves(Browse.Page.Home).collectAsStateWithLifecycle()
    ListPage(c.nav, "Home", account = account) { shelves(c, shelves, Hero.Picks) }
}

@Composable
fun NewPage(c: LibraryContext, browse: Browse, account: @Composable () -> Unit) {
    val shelves by browse.shelves(Browse.Page.New).collectAsStateWithLifecycle()
    ListPage(c.nav, "New", account = account) { shelves(c, shelves, Hero.Editorial) }
}

@Composable
fun RadioPage(c: LibraryContext, browse: Browse, account: @Composable () -> Unit) {
    val shelves by browse.shelves(Browse.Page.Radio).collectAsStateWithLifecycle()
    ListPage(c.nav, "Radio", account = account) {
        item { Spacer(Modifier.height(10.dp)) }
        shelves(c, shelves, Hero.Stations)
        item {
            BasicText(
                "Stations are Apple Music only. SoundCloud and Navidrome have no radio.",
                Modifier.padding(start = 20.dp, end = 20.dp, top = 18.dp),
                style = geist(13.5.sp).copy(color = roles.sub, lineHeight = 19.sp),
            )
        }
    }
}

private fun LazyListScope.shelves(c: LibraryContext, loading: Loading<List<Shelf>>, hero: Hero) {
    val shelves = (loading as? Loading.Ready)?.value
    if (shelves.isNullOrEmpty()) {
        item { Empty(if (shelves == null) loading.listing() else Listing(done = true), "Nothing here yet", "Apple Music has nothing to show here.", io.github.honeyslatt.music.ui.components.Shapes.Shelves) }
        return
    }
    for (shelf in shelves) {
        when {
            shelf.songs.isNotEmpty() -> songColumns(shelf.title, shelf.songs, { c.nav.push(Route.Shelf(shelf)) }) { i -> c.playback.play(shelf.songs, i) }
            shelf.hero && hero == Hero.Stations -> {
                if (shelf.title.isNotBlank()) item { SectionHeader(shelf.title) }
                items(shelf.items.filterIsInstance<Collection.Station>()) { StationCard(c, it.v1) }
            }
            shelf.hero -> {
                if (shelf.title.isNotBlank()) item { SectionHeader(shelf.title) }
                item { HeroRow(c, shelf, hero) }
            }
            else -> {
                item { SectionHeader(shelf.title) { c.nav.push(Route.Shelf(shelf)) } }
                item { CoverRow(c, shelf.items) }
            }
        }
    }
}

/** What a tap on an album, a playlist or a station does. */
@androidx.annotation.OptIn(UnstableApi::class)
private fun open(c: LibraryContext, item: Collection, scope: CoroutineScope, failed: () -> Unit) {
    when (item) {
        is Collection.Album -> c.nav.push(Route.Album(item.v1.id))
        is Collection.Playlist -> c.nav.push(Route.Playlist(item.v1.id, item.v1))
        is Collection.Station -> scope.launch { runCatching { c.playback.playStation(item.v1) }.onFailure { failed() } }
    }
}

@androidx.annotation.OptIn(UnstableApi::class)
@Composable
private fun CoverRow(c: LibraryContext, items: List<Collection>) {
    val actions = LocalActions.current
    val scope = androidx.compose.runtime.rememberCoroutineScope()
    val state = rememberLazyListState()
    LazyRow(
        state = state,
        flingBehavior = rememberSnapFlingBehavior(state, SnapPosition.Start),
        contentPadding = PaddingValues(horizontal = 20.dp),
        horizontalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        items(items) { item ->
            val (artwork, title, subtitle) = when (item) {
                is Collection.Album -> Triple(item.v1.artwork, item.v1.title, item.v1.artists.joinToString(", "))
                is Collection.Playlist -> Triple(item.v1.artwork, item.v1.name, item.v1.curator ?: "Playlist")
                is Collection.Station -> Triple(item.v1.artwork, item.v1.name, item.v1.description ?: kindLabel(item.v1.kind))
            }
            Box {
                CoverCard(
                    artwork,
                    title,
                    subtitle,
                    explicit = (item as? Collection.Album)?.v1?.explicit == true,
                    modifier = Modifier.width(164.dp),
                    onMenu = when (item) {
                        is Collection.Album -> { r -> actions?.albumMenu(item.v1, r) }
                        is Collection.Playlist -> { r -> actions?.playlistMenu(item.v1, r) }
                        is Collection.Station -> null
                    },
                ) { open(c, item, scope) { actions?.hud?.show("Couldn't tune in", io.github.honeyslatt.music.ui.Icons.Alert) } }
                if (item is Collection.Station && item.v1.kind == StationKind.LIVE) LiveBadge(Modifier.padding(10.dp))
            }
        }
    }
}

private fun kindLabel(kind: StationKind) = when (kind) {
    StationKind.LIVE -> "Live Radio"
    StationKind.EPISODE -> "Episode"
    StationKind.TRACKS -> "Station"
}

/** The page's large shelf: Top Picks on Home, the editors' cards on New. */
@Composable
private fun HeroRow(c: LibraryContext, shelf: Shelf, hero: Hero) {
    val actions = LocalActions.current
    val scope = androidx.compose.runtime.rememberCoroutineScope()
    val state = rememberLazyListState()
    BoxWithConstraints(Modifier.fillMaxWidth()) {
        val wide = maxWidth - 52.dp
        LazyRow(
            state = state,
            flingBehavior = rememberSnapFlingBehavior(state, SnapPosition.Start),
            contentPadding = PaddingValues(horizontal = 20.dp),
            horizontalArrangement = Arrangement.spacedBy(12.dp),
        ) {
            items(shelf.items.withIndex().toList()) { (i, item) ->
                val pick = shelf.picks.getOrNull(i)
                val tap = { open(c, item, scope) { actions?.hud?.show("Couldn't tune in", io.github.honeyslatt.music.ui.Icons.Alert) } }
                if (hero == Hero.Editorial) Editorial(item, pick, wide, tap) else PickCard(item, pick, tap)
            }
        }
    }
}

private fun Collection.texts(): Triple<io.github.honeyslatt.music.core.model.Artwork?, String, String?> = when (this) {
    is Collection.Album -> Triple(v1.artwork, v1.title, v1.artists.joinToString(", "))
    is Collection.Playlist -> Triple(v1.artwork, v1.name, v1.description ?: v1.curator)
    is Collection.Station -> Triple(v1.artwork, v1.name, v1.description)
}

/** A Top Pick: the cover, then a footer in the cover's own colour. */
@Composable
private fun PickCard(item: Collection, pick: Pick?, onClick: () -> Unit) {
    val (artwork, title, subtitle) = item.texts()
    val fill = artwork?.background?.let { Color(0xFF000000.toInt() or it.toInt()) } ?: roles.surface2
    val ink = if (fill.luminance() > 0.5f) Color(0xFF161616) else Color.White
    val shape = RoundedCornerShape(16.dp)
    Column(Modifier.width(252.dp).height(344.dp).press(shape, shrink = 0.97f, onClick = onClick).background(fill, shape)) {
        AsyncImage(
            artwork?.at(600),
            contentDescription = null,
            contentScale = ContentScale.Crop,
            modifier = Modifier.size(252.dp).background(roles.surface, RoundedCornerShape(topStart = 16.dp, topEnd = 16.dp)),
        )
        Column(Modifier.padding(horizontal = 14.dp, vertical = 12.dp), verticalArrangement = Arrangement.spacedBy(2.dp)) {
            val badge = pick?.badge?.takeIf { it.isNotBlank() } ?: (item as? Collection.Playlist)?.v1?.curator
            badge?.let { BasicText(it.uppercase(), style = geist(12.sp, FontWeight.SemiBold, 0.04f).copy(color = ink.copy(alpha = 0.75f)), maxLines = 1) }
            Line(title, geist(17.sp, FontWeight.Bold).copy(color = ink))
            (pick?.blurb ?: subtitle)?.let { Line(it, geist(13.sp).copy(color = ink.copy(alpha = 0.8f))) }
        }
    }
}

/** An editors' card: what it is, its name and who made it, over a wide picture. */
@Composable
private fun Editorial(item: Collection, pick: Pick?, width: androidx.compose.ui.unit.Dp, onClick: () -> Unit) {
    val (artwork, title, subtitle) = item.texts()
    Column(Modifier.width(width).press(shrink = 0.98f, onClick = onClick)) {
        pick?.badge?.takeIf { it.isNotBlank() }?.let {
            BasicText(it.uppercase(), style = geist(11.5.sp, FontWeight.SemiBold, 0.05f).copy(color = roles.sub), maxLines = 1)
        }
        Line(title, geist(21.sp, FontWeight.SemiBold).copy(color = roles.text), Modifier.padding(top = 2.dp))
        Line(subtitle ?: "", geist(16.sp).copy(color = roles.sub), Modifier.padding(bottom = 10.dp))
        Box(Modifier.fillMaxWidth().aspectRatio(16f / 10f).background(roles.surface, RoundedCornerShape(12.dp))) {
            AsyncImage(
                (pick?.wide ?: artwork)?.at(1000),
                contentDescription = null,
                contentScale = ContentScale.Crop,
                modifier = Modifier.fillMaxSize().clipRounded(12),
            )
            pick?.blurb?.let {
                Box(
                    Modifier.fillMaxSize().clipRounded(12).background(Brush.verticalGradient(0.55f to Color.Transparent, 1f to Color.Black.copy(alpha = 0.6f))),
                    contentAlignment = Alignment.BottomStart,
                ) {
                    BasicText(it, Modifier.padding(14.dp), style = geist(14.sp, FontWeight.Medium).copy(color = Color.White, lineHeight = 19.sp), maxLines = 2)
                }
            }
        }
    }
}

/** A station, full width: its picture, LIVE when it airs, and its name. */
@androidx.annotation.OptIn(UnstableApi::class)
@Composable
private fun StationCard(c: LibraryContext, station: Station) {
    val actions = LocalActions.current
    val scope = androidx.compose.runtime.rememberCoroutineScope()
    val shape = RoundedCornerShape(16.dp)
    Box(
        Modifier
            .padding(start = 20.dp, end = 20.dp, bottom = 14.dp)
            .fillMaxWidth()
            .aspectRatio(16f / 10f)
            .press(shape, shrink = 0.98f) { open(c, Collection.Station(station), scope) { actions?.hud?.show("Couldn't tune in", io.github.honeyslatt.music.ui.Icons.Alert) } }
            .background(roles.surface, shape)
            .clipRounded(16),
    ) {
        AsyncImage(station.artwork?.at(1000), contentDescription = null, contentScale = ContentScale.Crop, modifier = Modifier.fillMaxSize())
        Box(Modifier.fillMaxSize().background(Brush.verticalGradient(0.45f to Color.Transparent, 1f to Color.Black.copy(alpha = 0.7f))))
        if (station.kind == StationKind.LIVE) LiveBadge(Modifier.padding(12.dp))
        Column(Modifier.align(Alignment.BottomStart).padding(start = 16.dp, bottom = 14.dp, end = 16.dp)) {
            BasicText(
                (station.description ?: kindLabel(station.kind)).uppercase(),
                style = geist(12.sp, FontWeight.SemiBold, 0.05f).copy(color = Color.White.copy(alpha = 0.8f)),
                maxLines = 1,
            )
            Line(station.name, geist(20.sp, FontWeight.Bold).copy(color = Color.White))
        }
    }
}

@Composable
private fun LiveBadge(modifier: Modifier = Modifier) {
    BasicText(
        "LIVE",
        modifier.background(roles.red, RoundedCornerShape(5.dp)).padding(horizontal = 7.dp, vertical = 3.dp),
        style = geist(11.sp, FontWeight.Bold, 0.04f).copy(color = Color.White),
    )
}

private fun Modifier.clipRounded(radius: Int) = clip(RoundedCornerShape(radius.dp))

/** A shelf whole: its songs in a list, or its covers in a grid. */
@androidx.annotation.OptIn(UnstableApi::class)
@Composable
fun ShelfPage(c: LibraryContext, shelf: Shelf) {
    if (shelf.songs.isNotEmpty()) {
        ListPage(c.nav, shelf.title) {
            item {
                io.github.honeyslatt.music.ui.library.PlayRow(c, shelf.songs)
            }
            items(shelf.songs.size) { i -> io.github.honeyslatt.music.ui.components.TrackRow(shelf.songs[i]) { c.playback.play(shelf.songs, i) } }
        }
        return
    }
    val actions = LocalActions.current
    val scope = androidx.compose.runtime.rememberCoroutineScope()
    io.github.honeyslatt.music.ui.components.GridPage(c.nav, shelf.title) {
        items(shelf.items.size) { i ->
            val item = shelf.items[i]
            val (artwork, title, subtitle) = when (item) {
                is Collection.Album -> Triple(item.v1.artwork, item.v1.title, item.v1.artists.joinToString(", "))
                is Collection.Playlist -> Triple(item.v1.artwork, item.v1.name, item.v1.curator ?: "Playlist")
                is Collection.Station -> Triple(item.v1.artwork, item.v1.name, item.v1.description ?: kindLabel(item.v1.kind))
            }
            CoverCard(
                artwork,
                title,
                subtitle,
                explicit = (item as? Collection.Album)?.v1?.explicit == true,
                modifier = Modifier.gridCell(),
                onMenu = when (item) {
                    is Collection.Album -> { r -> actions?.albumMenu(item.v1, r) }
                    is Collection.Playlist -> { r -> actions?.playlistMenu(item.v1, r) }
                    is Collection.Station -> null
                },
            ) { open(c, item, scope) { actions?.hud?.show("Couldn't tune in", io.github.honeyslatt.music.ui.Icons.Alert) } }
        }
    }
}
