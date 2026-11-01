package io.github.honeyslatt.music.ui.detail

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.BasicText
import androidx.compose.runtime.Composable
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.produceState
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import io.github.honeyslatt.music.core.model.ItemId
import io.github.honeyslatt.music.core.model.Playlist
import io.github.honeyslatt.music.core.model.ProviderKind
import io.github.honeyslatt.music.core.model.Track
import io.github.honeyslatt.music.ui.Icons
import io.github.honeyslatt.music.ui.components.BarAction
import io.github.honeyslatt.music.ui.components.Glyph
import io.github.honeyslatt.music.ui.components.ServiceLogo
import io.github.honeyslatt.music.ui.components.TrackRow
import io.github.honeyslatt.music.ui.components.label
import io.github.honeyslatt.music.ui.components.press
import io.github.honeyslatt.music.ui.library.Empty
import io.github.honeyslatt.music.ui.library.LibraryContext
import io.github.honeyslatt.music.ui.library.PlayShuffle
import io.github.honeyslatt.music.ui.actions.LocalActions
import io.github.honeyslatt.music.ui.components.SearchReveal
import io.github.honeyslatt.music.ui.components.matches
import androidx.compose.runtime.saveable.rememberSaveable
import io.github.honeyslatt.music.ui.theme.geist
import io.github.honeyslatt.music.ui.theme.roles

@Composable
fun PlaylistPage(c: LibraryContext, id: ItemId, known: Playlist? = null) {
    val playlist: Playlist? = c.lists.playlists.items.firstOrNull { it.id == id } ?: known
    // Music's own read again as they change, here or on another device.
    val changes by c.playback.devices.changes.collectAsStateWithLifecycle()
    val version = if (id.provider == ProviderKind.MUSIC) changes else 0
    // Every page of its songs, one after another.
    var complete by remember(id) { mutableStateOf(false) }
    val loaded by produceState<Loading<List<Track>>>(Loading.Pending, id, version) {
        value = load {
            val all = mutableListOf<Track>()
            var page: String? = null
            do {
                val got = c.core.session.playlistTracks(id, page)
                all += got.items
                value = Loading.Ready(all.toList())
                page = got.next
            } while (page != null)
            complete = true
            all.toList()
        }
    }
    // Songs taken out here, gone at once.
    var removed by remember(id) { mutableStateOf(setOf<ItemId>()) }
    var searching by rememberSaveable(id) { mutableStateOf(false) }
    var query by rememberSaveable(id) { mutableStateOf("") }
    val every = (loaded as? Loading.Ready)?.value.orEmpty().filter { it.id !in removed }
    val tracks = if (query.isBlank()) every else every.filter { matches(query, it.title, it.artists.joinToString(", "), it.album) }
    val actions = LocalActions.current
    val favourites = playlist?.favourites == true
    val page = if (favourites) favouritesPage(roles.dark) else rememberPageColour(playlist?.artwork)
    val name = playlist?.name ?: "Playlist"
    val banner = io.github.honeyslatt.music.ui.theme.look.layout.header == io.github.honeyslatt.music.core.model.Header.BANNER
    val profile by c.core.appleProfile.collectAsStateWithLifecycle()
    DetailPage(
        c.nav,
        name,
        page,
        listOf(
            BarAction(Icons.Search, "Search in Playlist") { searching = !searching },
            *listOfNotNull(
                if (playlist != null && !favourites && playlist.id.provider == ProviderKind.APPLE_MUSIC) {
                    BarAction(if (actions?.isFavourite(playlist) == true) Icons.Starf else Icons.Star, "Favourite") { actions?.togglePlaylistFavourite(playlist) }
                } else {
                    null
                },
            ).toTypedArray(),
            BarAction(Icons.More, "More") { rect -> playlist?.let { actions?.playlistMenu(it, rect, onDeleted = { c.nav.pop() }) } },
        ),
    ) {
        item {
            DetailHeader(
                artwork = playlist?.artwork,
                motion = null,
                title = name,
                // Spotify's says whose it is, and how long, under the title.
                byline = playlist?.curator
                    ?: (if (banner && playlist?.editable == true) profile?.name else null)
                    ?: playlist?.id?.provider?.takeIf { it != ProviderKind.APPLE_MUSIC }?.label(),
                onByline = null,
                meta = {
                    if (banner && complete && tracks.isNotEmpty()) {
                        BasicText(countAndLength(tracks), style = geist(14.sp).copy(color = roles.sub))
                    }
                },
                art = if (favourites) {
                    {
                        Box(
                            Modifier.fillMaxSize().background(Brush.linearGradient(listOf(roles.accent, roles.mauve)), RoundedCornerShape(12.dp)),
                            contentAlignment = Alignment.Center,
                        ) { Glyph(Icons.Starf, roles.onAccent, 110.dp) }
                    }
                } else if (playlist != null && playlist.artwork == null) {
                    { io.github.honeyslatt.music.ui.components.PlaylistCover(playlist, 264.dp, RoundedCornerShape(12.dp), Modifier.fillMaxSize()) }
                } else {
                    null
                },
            )
        }
        playlist?.description?.takeIf { it.isNotBlank() }?.let { text -> item { Description(text) } }
        if (playlist != null && playlist.id.provider != ProviderKind.APPLE_MUSIC) {
            item {
                Column(Modifier.fillMaxWidth().padding(top = 6.dp), horizontalAlignment = Alignment.CenterHorizontally) {
                    ServiceLogo(playlist.id.provider, 18.dp)
                }
            }
        }
        item { SearchReveal(searching, query, "Search in Playlist") { query = it } }
        item { io.github.honeyslatt.music.ui.library.PlayRow(c, tracks) }
        item { Spacer(Modifier.height(8.dp)) }
        if (tracks.isEmpty()) item { Empty(loaded.listing(), "No songs", "This playlist is empty.") }
        items(tracks.withIndex().toList(), key = { (i, t) -> "$i" + t.id.id }) { (i, track) ->
            TrackRow(track, inPlaylist = playlist, onRemoved = { removed = removed + track.id }) { c.playback.play(tracks, i) }
        }
        // Its count and length go under the songs, once they are all in.
        if (tracks.isNotEmpty() && complete && query.isBlank()) item { Foot(listOf(countAndLength(tracks))) }
    }
}

/** A playlist's description: two lines, then MORE to read the rest. */
@Composable
private fun Description(text: String) {
    var open by remember(text) { mutableStateOf(false) }
    Box(Modifier.fillMaxWidth().padding(start = 20.dp, end = 20.dp, top = 12.dp).press { open = !open }) {
        BasicText(
            text,
            style = geist(14.sp).copy(color = roles.sub, lineHeight = 20.sp),
            maxLines = if (open) Int.MAX_VALUE else 2,
            overflow = TextOverflow.Ellipsis,
        )
        if (!open) {
            BasicText(
                "MORE",
                Modifier.align(Alignment.BottomEnd).background(roles.base).padding(start = 8.dp),
                style = geist(13.sp, FontWeight.SemiBold).copy(color = roles.text),
            )
        }
    }
}
