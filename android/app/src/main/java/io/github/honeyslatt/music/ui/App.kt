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
import androidx.compose.ui.draw.drawBehind
import androidx.compose.ui.unit.dp
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.input.nestedscroll.nestedScroll
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
import io.github.honeyslatt.music.ui.theme.look
import androidx.compose.animation.core.VisibilityThreshold
import io.github.honeyslatt.music.ui.theme.roles

/** The app: the page of the current tab, under the bottom chrome. */
@Composable
fun App(core: Core, library: Library, playback: io.github.honeyslatt.music.playback.Playback, settings: Settings, connecting: io.github.honeyslatt.music.ui.connect.Connecting) {
    val nav = remember { Navigator() }
    val lists by library.lists.collectAsStateWithLifecycle()
    // The services connected, for the source chips.
    val services = io.github.honeyslatt.music.data.SOURCES.filter { core.connection(it).collectAsStateWithLifecycle().value is Connection.Connected }
    var settingsOpen by remember { mutableStateOf(false) }
    // The devices to play on, from the mini player in Spotify's layout.
    var devicesOpen by remember { mutableStateOf(false) }
    // The account's picture on Apple Music, once connected.
    val profile by core.appleProfile.collectAsStateWithLifecycle()
    val picture = profile?.picture
    val account: @Composable () -> Unit = remember(picture) { { Avatar("H", picture) { settingsOpen = true } } }
    val now by playback.now.collectAsStateWithLifecycle()
    val playing by playback.playing.collectAsStateWithLifecycle()
    val loading by playback.loading.collectAsStateWithLifecycle()
    // The same while what it holds is: the pages skip as the song or a download moves on.
    val context = remember(lists, services, account, settings.animatedArtwork, settings.recentByDate) {
        LibraryContext(core, nav, lists, services, account, animatedArtwork = settings.animatedArtwork, recentByDate = settings.recentByDate, playback = playback, library = library)
    }
    var npOpen by rememberSaveable { mutableStateOf(false) }
    val mini = MiniState(now, playing, loading, playback::toggle, playback::next, playback::previousSong, onOpen = { npOpen = true }, npOpen = npOpen && now != null)
    val scope = rememberCoroutineScope()
    val appContext = androidx.compose.ui.platform.LocalContext.current.applicationContext
    val hud = remember { Hud(scope) }
    val menus = remember { Menus() }
    val actions = remember { Actions(appContext, core, library, playback, nav, hud, menus, scope) }
    val browse = remember { Browse(core, scope, settings) }
    LaunchedEffect(Unit) { playback.devices.notices.collect { hud.show(it) } }
    val downloads = (appContext as io.github.honeyslatt.music.MusicApp).downloads
    val downloadStates by downloads.states.collectAsStateWithLifecycle()
    // A service signed in to but out of reach: its songs not downloaded can't play.
    val connections = io.github.honeyslatt.music.data.SOURCES.associateWith { core.connection(it).collectAsStateWithLifecycle().value }
    val unreachable = connections.filterValues { it is io.github.honeyslatt.music.data.Connection.Failed }.keys
    val offline = unreachable.isNotEmpty() && connections.values.none { it is io.github.honeyslatt.music.data.Connection.Connected }
    LaunchedEffect(Unit) {
        downloads.failed.collect { message ->
            if (message != null) {
                hud.show(message, Icons.Alert)
                downloads.failedSeen()
            }
        }
    }
    LaunchedEffect(Unit) {
        playback.remoteVolume.collect { v -> hud.show("${Math.round(v * 100)} %", if (v == 0f) Icons.Vlow else Icons.Vhigh) }
    }
    actions.beforeLeaving = { npOpen = false }
    // The last songs' lyrics, found once: Now Playing's card, its full
    // lyrics and Lyrics opened again ask for the same. A song not found is
    // asked again next time, as may the network have been.
    val lyricsFound = remember { object : LinkedHashMap<String, io.github.honeyslatt.music.core.model.Lyrics>(16, 0.75f, true) {
        override fun removeEldestEntry(eldest: MutableMap.MutableEntry<String, io.github.honeyslatt.music.core.model.Lyrics>?) = size > 20
    } }
    LyricsSource.find = { track ->
        // A station's stream has no lyrics to look for.
        if (playback.isStation(track)) {
            null
        } else {
            val sites = settings.lyricsSites.filter { it in settings.lyricsSitesOn }
            val key = "${track.id.provider}:${track.id.id}:$sites:${settings.sitesFirst}"
            synchronized(lyricsFound) { lyricsFound[key] }
                ?: runCatching { core.session.lyrics(track, sites, settings.sitesFirst) }.getOrNull()?.also {
                    synchronized(lyricsFound) { lyricsFound[key] = it }
                }
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

    // The page shown, and the transition to it, which the back gesture
    // seeks through: the page follows the finger, the one before it
    // under it, and goes once let go (or comes back, the gesture called off).
    val shown = Triple(nav.tab, nav.stack.size, nav.page)
    val pages = remember { androidx.compose.animation.core.SeekableTransitionState(shown) }
    LaunchedEffect(shown) { pages.animateTo(shown) }
    androidx.activity.compose.PredictiveBackHandler(enabled = nav.canPop && !npOpen) { progress ->
        val back = Triple(nav.tab, nav.stack.size - 1, nav.stack[nav.stack.size - 2])
        try {
            progress.collect { pages.seekTo(it.progress, back) }
            nav.pop()
        } catch (e: kotlinx.coroutines.CancellationException) {
            scope.launch { pages.animateTo(shown) }
            throw e
        }
    }
    BackHandler(enabled = npOpen) { npOpen = false }
    val kept = androidx.compose.runtime.saveable.rememberSaveableStateHolder()
    androidx.compose.runtime.CompositionLocalProvider(
        LocalNowPlaying provides NowPlayingMark(now?.id, playing),
        io.github.honeyslatt.music.ui.detail.LocalMotionQuality provides settings.motionQuality,
        LocalActions provides actions,
        LocalMenus provides menus,
        io.github.honeyslatt.music.ui.components.LocalDownloads provides io.github.honeyslatt.music.ui.components.DownloadsShown(downloadStates, unreachable, offline),
    ) {
    androidx.compose.animation.SharedTransitionLayout(Modifier.fillMaxSize().background(roles.base)) {
        val spotify = look.layout.motion == io.github.honeyslatt.music.core.model.Motion.SPOTIFY
        // Apple Music's: the page recedes, rounded and darkened, while Now
        // Playing rises over it, and comes back as it is swiped down.
        var npDrag by remember { androidx.compose.runtime.mutableFloatStateOf(0f) }
        val risen = androidx.compose.animation.core.animateFloatAsState(
            if (npOpen && now != null) 1f else 0f,
            tween(if (npOpen) 440 else 400, easing = Standard),
            label = "risen",
        )
        // Read as drawn, not composed: the pages aren't recomposed for it.
        val recede = { if (spotify) 0f else risen.value * (1f - npDrag) }
        val corner = with(androidx.compose.ui.platform.LocalDensity.current) { 18.dp.toPx() }
    androidx.compose.runtime.CompositionLocalProvider(io.github.honeyslatt.music.ui.chrome.LocalCoverFlight provides this.takeIf { !spotify }) {
        Box(
            Modifier.fillMaxSize().graphicsLayer {
                val r = recede()
                if (r > 0f) {
                    val s = 1f - 0.07f * r
                    scaleX = s
                    scaleY = s
                    shape = androidx.compose.foundation.shape.RoundedCornerShape(corner * r)
                    clip = true
                } else {
                    scaleX = 1f
                    scaleY = 1f
                    clip = false
                }
            },
        ) {
            androidx.compose.animation.core.rememberTransition(pages, label = "pages").AnimatedContent(
                transitionSpec = { push(initialState, targetState, spotify) },
            ) { (tab, depth, page) ->
                // Gone back to, a page is as it was left: scrolled where it
                // was, its search as typed.
                kept.SaveableStateProvider("$tab/$depth/$page") {
                    androidx.compose.runtime.CompositionLocalProvider(io.github.honeyslatt.music.ui.detail.LocalCovered provides (npOpen && now != null)) {
                        Page(tab, page, context, account, browse)
                    }
                }
            }
            if (look.mobile.tabs == io.github.honeyslatt.music.core.model.Tabs.THREE) {
                io.github.honeyslatt.music.ui.chrome.SpotifyBottom(nav, mini, playback, { devicesOpen = true }, Modifier.align(Alignment.BottomCenter))
            } else if (look.mobile.tabs == io.github.honeyslatt.music.core.model.Tabs.FOUR) {
                io.github.honeyslatt.music.ui.chrome.M3EBottom(nav, mini, playback, Modifier.align(Alignment.BottomCenter))
            } else {
                BottomChrome(nav, mini, Modifier.align(Alignment.BottomCenter))
            }
            Box(
                Modifier.fillMaxSize().drawBehind {
                    val r = recede()
                    if (r > 0f) drawRect(androidx.compose.ui.graphics.Color.Black.copy(alpha = 0.4f * r))
                },
            )
        }
        // Now Playing rises from the mini player over 440 ms, and goes back
        // down with a swipe.
        AnimatedVisibility(
            npOpen && now != null,
            enter = slideInVertically(tween(440, easing = Standard)) { it },
            exit = slideOutVertically(tween(400, easing = Standard)) { it },
        ) {
            androidx.compose.runtime.LaunchedEffect(Unit) { npDrag = 0f }
            androidx.compose.runtime.CompositionLocalProvider(io.github.honeyslatt.music.ui.chrome.LocalNpVisibility provides this) {
            SwipeDown(npOpen, { npOpen = false }, { npDrag = it }) {
                if (look.mobile.nowPlaying == io.github.honeyslatt.music.core.model.MobileNowPlaying.SPOTIFY) {
                    io.github.honeyslatt.music.ui.nowplaying.SpotifyNowPlaying(playback, npActions)
                } else if (look.mobile.nowPlaying == io.github.honeyslatt.music.core.model.MobileNowPlaying.EXPRESSIVE) {
                    io.github.honeyslatt.music.ui.m3e.ExpressiveNowPlaying(playback, npActions)
                } else {
                    NowPlaying(playback, npActions)
                }
            }
            }
        }
        io.github.honeyslatt.music.ui.nowplaying.DevicesSheet(devicesOpen, playback) { devicesOpen = false }
        io.github.honeyslatt.music.ui.settings.SettingsSheet(settingsOpen, core, settings, connecting::start) { settingsOpen = false }
        ActionSheets(actions)
        io.github.honeyslatt.music.ui.nowplaying.PairingHost(playback.devices)
        MenuHost(menus)
        HudHost(hud)
    }
    }
    }
}

/**
 * Apple Music's: pushes slide in from the right on a spring that settles
 * without bouncing, as iOS's, pops back. Spotify's: the page comes in a
 * fade, from a little smaller, or larger going back. Tabs swap at once.
 */
private fun push(from: Triple<Tab, Int, Route>, to: Triple<Tab, Int, Route>, spotify: Boolean): ContentTransform {
    val spec = androidx.compose.animation.core.spring(
        dampingRatio = 1f,
        stiffness = 300f,
        visibilityThreshold = androidx.compose.ui.unit.IntOffset.VisibilityThreshold,
    )
    val deeper = to.second > from.second
    return when {
        from.first != to.first -> fadeIn(tween(120)) togetherWith fadeOut(tween(120))
        spotify -> (fadeIn(tween(220, delayMillis = 40)) + androidx.compose.animation.scaleIn(tween(260, easing = Standard), if (deeper) 0.96f else 1.03f)) togetherWith
            fadeOut(tween(140))
        deeper ->
            slideInHorizontally(spec) { it } togetherWith slideOutHorizontally(spec) { -it * 28 / 100 }
        else -> slideInHorizontally(spec) { -it * 28 / 100 } togetherWith slideOutHorizontally(spec) { it }
    }
}

@Composable
private fun Page(tab: Tab, page: Route, c: LibraryContext, account: @Composable () -> Unit, browse: Browse) {
    when (page) {
        Route.Root -> when (tab) {
            Tab.Home -> if (look.mobile.tabs == io.github.honeyslatt.music.core.model.Tabs.THREE) {
                io.github.honeyslatt.music.ui.browse.SpotifyHome(c, browse, account)
            } else {
                HomePage(c, browse, account)
            }
            Tab.New -> NewPage(c, browse, account)
            Tab.Radio -> RadioPage(c, browse, account)
            Tab.Library -> if (look.mobile.library == io.github.honeyslatt.music.core.model.LibraryView.LIST) {
                io.github.honeyslatt.music.ui.library.YourLibrary(c)
            } else {
                LibraryRoot(c)
            }
            Tab.Search -> SearchPage(c)
        }
        Route.Playlists -> Playlists(c)
        Route.Albums -> Albums(c)
        Route.Artists -> Artists(c)
        Route.Songs -> Songs(c)
        Route.DownloadedSongs -> Songs(c, downloadedOnly = true)
        is Route.Album -> AlbumPage(c, page.id)
        is Route.Playlist -> PlaylistPage(c, page.id, page.known)
        is Route.LibraryArtist -> LibraryArtistPage(c, page.artist)
        is Route.Artist -> ArtistPage(c, page.id)
        is Route.Shelf -> io.github.honeyslatt.music.ui.browse.ShelfPage(c, page.shelf)
        is Route.Genre -> io.github.honeyslatt.music.ui.browse.GenrePage(c, page.genre, page.source)
    }
}

/** A sheet that follows a drag down, and closes past a quarter of the screen. */
@Composable
private fun SwipeDown(open: Boolean, onClose: () -> Unit, onDrag: (Float) -> Unit, content: @Composable () -> Unit) {
    var drag by androidx.compose.runtime.remember { androidx.compose.runtime.mutableFloatStateOf(0f) }
    val shown = androidx.compose.animation.core.animateFloatAsState(drag, label = "drag")
    // Opened again while it was going: back in place, not where the swipe left it.
    androidx.compose.runtime.LaunchedEffect(open) { if (open) drag = 0f }
    androidx.compose.foundation.layout.BoxWithConstraints(Modifier.fillMaxSize()) {
        val limit = constraints.maxHeight / 4f
        // How far down, as a share of the screen, for the page behind: told
        // as it moves, from one effect rather than one per frame.
        val height = constraints.maxHeight.toFloat()
        androidx.compose.runtime.LaunchedEffect(height) {
            androidx.compose.runtime.snapshotFlow { shown.value }.collect { onDrag((it / height).coerceIn(0f, 1f)) }
        }
        // A list or lyrics scrolled to their top hand the rest of a pull
        // down to the swipe, and its release.
        val handOver = androidx.compose.runtime.remember(limit) {
            object : androidx.compose.ui.input.nestedscroll.NestedScrollConnection {
                override fun onPreScroll(available: androidx.compose.ui.geometry.Offset, source: androidx.compose.ui.input.nestedscroll.NestedScrollSource): androidx.compose.ui.geometry.Offset {
                    // Pulled down already, a push up takes it back first.
                    if (drag > 0f && available.y < 0f) {
                        val taken = maxOf(available.y, -drag)
                        drag += taken
                        return androidx.compose.ui.geometry.Offset(0f, taken)
                    }
                    return androidx.compose.ui.geometry.Offset.Zero
                }

                override fun onPostScroll(consumed: androidx.compose.ui.geometry.Offset, available: androidx.compose.ui.geometry.Offset, source: androidx.compose.ui.input.nestedscroll.NestedScrollSource): androidx.compose.ui.geometry.Offset {
                    if (source == androidx.compose.ui.input.nestedscroll.NestedScrollSource.UserInput && available.y > 0f) {
                        drag += available.y
                        return androidx.compose.ui.geometry.Offset(0f, available.y)
                    }
                    return androidx.compose.ui.geometry.Offset.Zero
                }

                override suspend fun onPreFling(available: androidx.compose.ui.unit.Velocity): androidx.compose.ui.unit.Velocity {
                    if (drag <= 0f) return androidx.compose.ui.unit.Velocity.Zero
                    if (drag > limit || available.y > 1500f) onClose() else drag = 0f
                    return available
                }
            }
        }
        Box(
            Modifier
                .fillMaxSize()
                .graphicsLayer { translationY = shown.value }
                .nestedScroll(handOver)
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
