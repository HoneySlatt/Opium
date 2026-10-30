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
import io.github.honeyslatt.music.ui.theme.geist
import io.github.honeyslatt.music.ui.theme.roles

@Composable
fun PlaylistPage(c: LibraryContext, id: ItemId) {
    val playlist: Playlist? = c.lists.playlists.items.firstOrNull { it.id == id }
    // Every page of its songs, one after another.
    val loaded by produceState<Loading<List<Track>>>(Loading.Pending, id) {
        value = load {
            val all = mutableListOf<Track>()
            var page: String? = null
            do {
                val got = c.core.session.playlistTracks(id, page)
                all += got.items
                value = Loading.Ready(all.toList())
                page = got.next
            } while (page != null)
            all.toList()
        }
    }
    val tracks = (loaded as? Loading.Ready)?.value.orEmpty()
    val favourites = playlist?.favourites == true
    val page = if (favourites) favouritesPage(roles.dark) else rememberPageColour(playlist?.artwork)
    val name = playlist?.name ?: "Playlist"
    DetailPage(
        c.nav,
        name,
        page,
        listOf(
            BarAction(Icons.Search, "Search in Playlist") {},
            BarAction(if (playlist?.inFavourites == true) Icons.Starf else Icons.Star, "Favourite") {},
            BarAction(Icons.More, "More") {},
        ),
    ) {
        item {
            DetailHeader(
                artwork = playlist?.artwork,
                motion = null,
                title = name,
                byline = playlist?.curator ?: playlist?.id?.provider?.takeIf { it != ProviderKind.APPLE_MUSIC }?.label(),
                onByline = null,
                meta = { if (tracks.isNotEmpty()) Meta(countAndLength(tracks)) },
                art = if (favourites) {
                    {
                        Box(
                            Modifier.fillMaxSize().background(Brush.linearGradient(listOf(roles.accent, roles.mauve)), RoundedCornerShape(12.dp)),
                            contentAlignment = Alignment.Center,
                        ) { Glyph(Icons.Starf, roles.onAccent, 110.dp) }
                    }
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
        item { PlayShuffle(enabled = tracks.isNotEmpty()) }
        item { Spacer(Modifier.height(8.dp)) }
        if (tracks.isEmpty()) item { Empty(loaded.listing(), "No songs", "This playlist is empty.") }
        items(tracks.withIndex().toList(), key = { (i, t) -> "$i" + t.id.id }) { (_, track) -> TrackRow(track) {} }
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
