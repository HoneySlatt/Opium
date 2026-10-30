package io.github.honeyslatt.music.ui

import androidx.activity.compose.BackHandler
import androidx.compose.animation.AnimatedContent
import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.slideInVertically
import androidx.compose.animation.slideOutVertically
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import io.github.honeyslatt.music.core.LyricsSite
import io.github.honeyslatt.music.ui.nowplaying.LyricsSource
import io.github.honeyslatt.music.ui.components.LocalNowPlaying
import io.github.honeyslatt.music.ui.components.Hud
import io.github.honeyslatt.music.ui.components.HudHost
import io.github.honeyslatt.music.ui.components.Menus
import io.github.honeyslatt.music.ui.components.MenuHost
import io.github.honeyslatt.music.ui.components.LocalMenus
import io.github.honeyslatt.music.ui.actions.Actions
import io.github.honeyslatt.music.ui.browse.Browse
import io.github.honeyslatt.music.ui.browse.HomePage
import io.github.honeyslatt.music.ui.browse.NewPage
import io.github.honeyslatt.music.ui.browse.RadioPage
import io.github.honeyslatt.music.ui.browse.SearchPage
import io.github.honeyslatt.music.ui.actions.ActionSheets
import io.github.honeyslatt.music.ui.actions.LocalActions
import io.github.honeyslatt.music.ui.components.NowPlayingMark
import io.github.honeyslatt.music.ui.nowplaying.NowPlaying
import io.github.honeyslatt.music.ui.nowplaying.NpActions
import kotlinx.coroutines.launch
import androidx.compose.animation.ContentTransform
import androidx.compose.animation.core.tween
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.slideInHorizontally
import androidx.compose.animation.slideOutHorizontally
import androidx.compose.animation.togetherWith
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.gestures.detectVerticalDragGestures
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import io.github.honeyslatt.music.data.Connection
import io.github.honeyslatt.music.data.Core
import io.github.honeyslatt.music.data.Library
import io.github.honeyslatt.music.data.Settings
import io.github.honeyslatt.music.core.model.ProviderKind
import io.github.honeyslatt.music.ui.chrome.BottomChrome
import io.github.honeyslatt.music.ui.chrome.MiniState
import io.github.honeyslatt.music.ui.chrome.Standard
import io.github.honeyslatt.music.ui.components.Avatar
import io.github.honeyslatt.music.ui.detail.AlbumPage
import io.github.honeyslatt.music.ui.detail.ArtistPage
import io.github.honeyslatt.music.ui.detail.LibraryArtistPage
import io.github.honeyslatt.music.ui.detail.PlaylistPage
import io.github.honeyslatt.music.ui.library.Albums
import io.github.honeyslatt.music.ui.library.Artists
import io.github.honeyslatt.music.ui.library.LibraryContext
import io.github.honeyslatt.music.ui.library.LibraryRoot
import io.github.honeyslatt.music.ui.library.Playlists
import io.github.honeyslatt.music.ui.library.Songs
import io.github.honeyslatt.music.ui.nav.Navigator
import io.github.honeyslatt.music.ui.nav.Route
import io.github.honeyslatt.music.ui.nav.Tab
import io.github.honeyslatt.music.ui.theme.roles

/** The app: the page of the current tab, under the bottom chrome. */
@Composable
fun App(core: Core, library: Library, playback: io.github.honeyslatt.music.playback.Playback, settings: Settings, connecting: io.github.honeyslatt.music.ui.connect.Connecting) {
    val nav = remember { Navigator() }
    val lists by library.lists.collectAsStateWithLifecycle()
    // The services connected, for the source chips.
    val services = ProviderKind.entries.filter { core.connection(it).collectAsStateWithLifecycle().value is Connection.Connected }
    var settingsOpen by remember { mutableStateOf(false) }
    val account: @Composable () -> Unit = { Avatar("H") { settingsOpen = true } }
    val now by playback.now.collectAsStateWithLifecycle()
    val playing by playback.playing.collectAsStateWithLifecycle()
    val loading by playback.loading.collectAsStateWithLifecycle()
    val context = LibraryContext(core, nav, lists, services, account, animatedArtwork = settings.animatedArtwork, playback = playback, library = library)
    var npOpen by rememberSaveable { mutableStateOf(false) }
    val mini = MiniState(now, playing, loading, playback::toggle, playback::next, playback::previous, onOpen = { npOpen = true })
    val scope = rememberCoroutineScope()
    val appContext = androidx.compose.ui.platform.LocalContext.current.applicationContext
    val hud = remember { Hud(scope) }
    val menus = remember { Menus() }
    val actions = remember { Actions(appContext, core, library, playback, nav, hud, menus, scope) }
    val browse = remember { Browse(core, scope) }
    LaunchedEffect(Unit) { playback.devices.notices.collect { hud.show(it) } }
    actions.beforeLeaving = { npOpen = false }
    LyricsSource.find = { track ->
        // A station's stream has no lyrics to look for.
        if (playback.isStation(track)) {
            null
        } else {
            runCatching { core.session.lyrics(track, settings.lyricsSites, settings.sitesFirst) }.getOrNull()
        }
    }
    // An album's animated cover, asked once.
    val motions = remember { java.util.concurrent.ConcurrentHashMap<String, String>() }
    val npActions = NpActions(
        close = { npOpen = false },
        openAlbum = { t -> t.albumId?.let { npOpen = false; nav.push(Route.Album(it)) } },
        openArtist = { t ->
            scope.launch {
                runCatching { core.session.songLinks(t.id).artist }.getOrNull()?.let {
                    npOpen = false
                    nav.push(Route.Artist(it))
                }
            }
        },
        motion = { t ->
            val album = t.albumId
            when {
                !context.animatedArtwork || album == null -> null
                else -> motions[album.id] ?: runCatching { core.session.albumMotion(album) }.getOrNull()?.also { motions[album.id] = it }
            }
        },
    )

    BackHandler(enabled = nav.canPop && !npOpen) { nav.pop() }
    BackHandler(enabled = npOpen) { npOpen = false }
    androidx.compose.runtime.CompositionLocalProvider(
        LocalNowPlaying provides NowPlayingMark(now?.id, playing),
        io.github.honeyslatt.music.ui.detail.LocalMotionQuality provides settings.motionQuality,
        LocalActions provides actions,
        LocalMenus provides menus,
    ) {
    Box(Modifier.fillMaxSize().background(roles.base)) {
        AnimatedContent(
            targetState = Triple(nav.tab, nav.stack.size, nav.page),
            transitionSpec = { push(initialState, targetState) },
            label = "pages",
        ) { (tab, _, page) ->
            Page(tab, page, context, account, browse)
        }
        BottomChrome(nav, mini, Modifier.align(Alignment.BottomCenter))
        // Now Playing rises from the mini player over 440 ms, and goes back
        // down with a swipe.
        AnimatedVisibility(
            npOpen && now != null,
            enter = slideInVertically(tween(440, easing = Standard)) { it },
            exit = slideOutVertically(tween(400, easing = Standard)) { it },
        ) {
            SwipeDown({ npOpen = false }) { NowPlaying(playback, npActions) }
        }
        io.github.honeyslatt.music.ui.settings.SettingsSheet(settingsOpen, core, settings, connecting::start) { settingsOpen = false }
        ActionSheets(actions)
        io.github.honeyslatt.music.ui.nowplaying.PairingHost(playback.devices)
        MenuHost(menus)
        HudHost(hud)
    }
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
private fun Page(tab: Tab, page: Route, c: LibraryContext, account: @Composable () -> Unit, browse: Browse) {
    when (page) {
        Route.Root -> when (tab) {
            Tab.Home -> HomePage(c, browse, account)
            Tab.New -> NewPage(c, browse, account)
            Tab.Radio -> RadioPage(c, browse, account)
            Tab.Library -> LibraryRoot(c)
            Tab.Search -> SearchPage(c)
        }
        Route.Playlists -> Playlists(c)
        Route.Albums -> Albums(c)
        Route.Artists -> Artists(c)
        Route.Songs -> Songs(c)
        is Route.Album -> AlbumPage(c, page.id)
        is Route.Playlist -> PlaylistPage(c, page.id, page.known)
        is Route.LibraryArtist -> LibraryArtistPage(c, page.artist)
        is Route.Artist -> ArtistPage(c, page.id)
        is Route.Shelf -> io.github.honeyslatt.music.ui.browse.ShelfPage(c, page.shelf)
    }
}

/** A sheet that follows a drag down, and closes past a quarter of the screen. */
@Composable
private fun SwipeDown(onClose: () -> Unit, content: @Composable () -> Unit) {
    var drag by androidx.compose.runtime.remember { androidx.compose.runtime.mutableFloatStateOf(0f) }
    val shown by androidx.compose.animation.core.animateFloatAsState(drag, label = "drag")
    androidx.compose.foundation.layout.BoxWithConstraints(Modifier.fillMaxSize()) {
        val limit = constraints.maxHeight / 4f
        Box(
            Modifier
                .fillMaxSize()
                .graphicsLayer { translationY = shown }
                .pointerInput(Unit) {
                    detectVerticalDragGestures(
                        onDragEnd = { if (drag > limit) onClose() else drag = 0f },
                        onDragCancel = { drag = 0f },
                    ) { change, amount ->
                        drag = (drag + amount).coerceAtLeast(0f)
                        change.consume()
                    }
                },
        ) { content() }
    }
}
