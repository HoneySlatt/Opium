package io.github.honeyslatt.music.ui

import androidx.activity.compose.BackHandler
import androidx.compose.animation.AnimatedContent
import androidx.compose.animation.ContentTransform
import androidx.compose.animation.core.tween
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.slideInHorizontally
import androidx.compose.animation.slideOutHorizontally
import androidx.compose.animation.togetherWith
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import io.github.honeyslatt.music.data.Connection
import io.github.honeyslatt.music.data.Core
import io.github.honeyslatt.music.data.Library
import io.github.honeyslatt.music.core.model.ProviderKind
import io.github.honeyslatt.music.ui.chrome.BottomChrome
import io.github.honeyslatt.music.ui.chrome.Standard
import io.github.honeyslatt.music.ui.components.Avatar
import io.github.honeyslatt.music.ui.detail.AlbumPage
import io.github.honeyslatt.music.ui.detail.ArtistPage
import io.github.honeyslatt.music.ui.detail.LibraryArtistPage
import io.github.honeyslatt.music.ui.detail.PlaylistPage
import io.github.honeyslatt.music.ui.library.Albums
import io.github.honeyslatt.music.ui.library.Artists
import io.github.honeyslatt.music.ui.library.Coming
import io.github.honeyslatt.music.ui.library.LibraryContext
import io.github.honeyslatt.music.ui.library.LibraryRoot
import io.github.honeyslatt.music.ui.library.Playlists
import io.github.honeyslatt.music.ui.library.RecentlyAdded
import io.github.honeyslatt.music.ui.library.Songs
import io.github.honeyslatt.music.ui.nav.Navigator
import io.github.honeyslatt.music.ui.nav.Route
import io.github.honeyslatt.music.ui.nav.Tab
import io.github.honeyslatt.music.ui.theme.roles

/** The app: the page of the current tab, under the bottom chrome. */
@Composable
fun App(core: Core, library: Library) {
    val nav = remember { Navigator() }
    val apple by core.apple.collectAsStateWithLifecycle()
    val lists by library.lists.collectAsStateWithLifecycle()
    val services = listOfNotNull(ProviderKind.APPLE_MUSIC.takeIf { apple is Connection.Connected })
    val account: @Composable () -> Unit = { Avatar("H") {} }
    val context = LibraryContext(core, nav, lists, services, account)

    BackHandler(enabled = nav.canPop) { nav.pop() }
    Box(Modifier.fillMaxSize().background(roles.base)) {
        AnimatedContent(
            targetState = Triple(nav.tab, nav.stack.size, nav.page),
            transitionSpec = { push(initialState, targetState) },
            label = "pages",
        ) { (tab, _, page) ->
            Page(tab, page, context, account)
        }
        BottomChrome(nav, Modifier.align(Alignment.BottomCenter))
    }
}

/** Pushes slide in from the right over 360 ms, pops back; tabs swap at once. */
private fun push(from: Triple<Tab, Int, Route>, to: Triple<Tab, Int, Route>): ContentTransform {
    val spec = tween<androidx.compose.ui.unit.IntOffset>(360, easing = Standard)
    return when {
        from.first != to.first -> fadeIn(tween(120)) togetherWith fadeOut(tween(120))
        to.second > from.second ->
            slideInHorizontally(spec) { it } togetherWith slideOutHorizontally(spec) { -it * 28 / 100 }
        else -> slideInHorizontally(spec) { -it * 28 / 100 } togetherWith slideOutHorizontally(spec) { it }
    }
}

@Composable
private fun Page(tab: Tab, page: Route, c: LibraryContext, account: @Composable () -> Unit) {
    when (page) {
        Route.Root -> when (tab) {
            Tab.Library -> LibraryRoot(c)
            else -> Coming(c.nav, tab.label, account.takeIf { tab != Tab.Search })
        }
        Route.Playlists -> Playlists(c)
        Route.Albums -> Albums(c)
        Route.Artists -> Artists(c)
        Route.Songs -> Songs(c)
        Route.RecentlyAdded -> RecentlyAdded(c)
        is Route.Album -> AlbumPage(c, page.id)
        is Route.Playlist -> PlaylistPage(c, page.id)
        is Route.LibraryArtist -> LibraryArtistPage(c, page.artist)
        is Route.Artist -> ArtistPage(c, page.id)
    }
}
