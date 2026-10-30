package io.github.honeyslatt.music.ui.settings

import androidx.compose.animation.core.animateDpAsState
import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.animation.core.tween
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.BasicText
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.draw.shadow
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.media3.common.util.UnstableApi
import io.github.honeyslatt.music.BuildConfig
import io.github.honeyslatt.music.core.model.ProviderKind
import io.github.honeyslatt.music.data.Connection
import io.github.honeyslatt.music.data.Core
import io.github.honeyslatt.music.data.MotionQuality
import io.github.honeyslatt.music.data.Settings
import io.github.honeyslatt.music.data.about
import io.github.honeyslatt.music.data.label
import io.github.honeyslatt.music.ui.Icons
import io.github.honeyslatt.music.ui.actions.LocalActions
import io.github.honeyslatt.music.ui.actions.SheetSpec
import io.github.honeyslatt.music.ui.components.Glyph
import io.github.honeyslatt.music.ui.components.Group
import io.github.honeyslatt.music.ui.components.GroupRow
import io.github.honeyslatt.music.ui.components.ServiceLogo
import io.github.honeyslatt.music.ui.components.Sheet
import io.github.honeyslatt.music.ui.components.SheetButton
import io.github.honeyslatt.music.ui.components.label
import io.github.honeyslatt.music.ui.components.press
import io.github.honeyslatt.music.ui.theme.Appearance
import io.github.honeyslatt.music.ui.theme.geist
import io.github.honeyslatt.music.ui.theme.roles
import kotlinx.coroutines.launch

private sealed interface SettingsPage {
    data object Root : SettingsPage
    data class Service(val kind: ProviderKind) : SettingsPage
    data object LyricsSites : SettingsPage
    data object Motion : SettingsPage
}

/** Settings, from the avatar: the services, then how the app behaves. */
@androidx.annotation.OptIn(UnstableApi::class)
@Composable
fun SettingsSheet(open: Boolean, core: Core, settings: Settings, onSignIn: (ProviderKind) -> Unit, onClose: () -> Unit) {
    var page by remember { mutableStateOf<SettingsPage>(SettingsPage.Root) }
    val close = {
        onClose()
        page = SettingsPage.Root
    }
    val back = { page = SettingsPage.Root }
    val backButton: @Composable () -> Unit = {
        Box(
            Modifier.size(40.dp).press(CircleShape, shrink = 0.92f, onClick = back).background(roles.surface, CircleShape),
            contentAlignment = Alignment.Center,
        ) { Glyph(Icons.Left, roles.text, 22.dp) }
    }
    when (val p = page) {
        SettingsPage.Root -> Sheet(open, "Account", close, left = { Spacer(Modifier.size(40.dp)) }, right = { SheetButton("Done", onClick = close) }) {
            Root(core, settings) { page = it }
        }
        is SettingsPage.Service -> Sheet(open, p.kind.label(), close, left = backButton) { Service(core, settings, p.kind, onSignIn, close) }
        SettingsPage.LyricsSites -> Sheet(open, "Lyrics Sites", close, left = backButton) { LyricsSites(settings) }
        SettingsPage.Motion -> Sheet(open, "Animated Artwork", close, left = backButton) { Motion(settings) }
    }
    // Back from a page of Settings goes to its root.
    androidx.activity.compose.BackHandler(open && page != SettingsPage.Root, back)
}

@Composable
private fun Label(text: String) {
    BasicText(text, Modifier.padding(start = 16.dp, bottom = 7.dp), style = geist(13.sp, FontWeight.Medium).copy(color = roles.sub))
}

/** A row's words: its name, and a line under it. */
@Composable
private fun androidx.compose.foundation.layout.RowScope.Words(title: String, detail: String? = null, colour: Color = roles.text) {
    Column(Modifier.weight(1f)) {
        BasicText(title, style = geist(16.sp).copy(color = colour))
        detail?.let { BasicText(it, Modifier.padding(top = 1.dp), style = geist(13.sp).copy(color = roles.sub, lineHeight = 17.5.sp)) }
    }
}

/** A switch: the accent when on, its knob sliding over. */
@Composable
fun Switch(on: Boolean, onChange: (Boolean) -> Unit) {
    val knob by animateDpAsState(if (on) 20.dp else 0.dp, tween(220, easing = io.github.honeyslatt.music.ui.chrome.Standard), label = "switch")
    val fill by animateFloatAsState(if (on) 1f else 0f, tween(200), label = "fill")
    Box(
        Modifier
            .size(52.dp, 32.dp)
            .press(RoundedCornerShape(16.dp)) { onChange(!on) }
            .background(androidx.compose.ui.graphics.lerp(roles.surface3, roles.accent, fill), RoundedCornerShape(16.dp)),
    ) {
        Box(
            Modifier
                .padding(2.dp)
                .offset(x = knob)
                .size(28.dp)
                .shadow(3.dp, CircleShape)
                .background(Color.White, CircleShape),
        )
    }
}

@Composable
private fun Root(core: Core, settings: Settings, go: (SettingsPage) -> Unit) {
    val apple by core.apple.collectAsStateWithLifecycle()
    val profile by core.appleProfile.collectAsStateWithLifecycle()
    Column(Modifier.fillMaxWidth().padding(top = 6.dp, bottom = 22.dp), horizontalAlignment = Alignment.CenterHorizontally) {
        // The account's picture on Apple Music, over its initial until it comes.
        Box(
            Modifier.size(76.dp).background(Brush.linearGradient(listOf(roles.accent, roles.mauve)), CircleShape),
            contentAlignment = Alignment.Center,
        ) {
            BasicText("H", style = geist(30.sp, FontWeight.SemiBold).copy(color = Color.White))
            profile?.picture?.let { io.github.honeyslatt.music.ui.components.Cover(it, 76.dp, CircleShape, Modifier.size(76.dp)) }
        }
        // Its handle there, else its name.
        val p = profile
        BasicText(
            when {
                apple !is Connection.Connected -> "Apple Music isn’t connected"
                p?.handle != null -> "@${p.handle}"
                p?.name != null -> p.name!!
                else -> "Apple Music"
            },
            Modifier.padding(top = 10.dp),
            style = geist(14.sp).copy(color = roles.sub),
        )
    }

    Label("Services")
    Group("Each service keeps its own sign-in, on this phone only. A song you have on several services plays from your server’s file first, then Apple Music, then SoundCloud.") {
        ProviderKind.entries.forEachIndexed { i, kind ->
            val state by core.connection(kind).collectAsStateWithLifecycle()
            val status = when {
                state is Connection.Connected -> "Connected"
                state is Connection.Connecting -> "Connecting…"
                state is Connection.Failed -> "Couldn’t connect"
                core.signedIn(kind) && !core.enabled(kind) -> "Off"
                else -> "Not signed in"
            }
            GroupRow(first = i == 0, inset = 58.dp, onClick = { go(SettingsPage.Service(kind)) }) {
                Box(Modifier.alpha(if (status == "Connected") 1f else 0.5f)) { ServiceLogo(kind, 30.dp) }
                Words(kind.label(), status)
                Glyph(Icons.Right, roles.muted, 18.dp)
            }
        }
    }

    Label("Library")
    Group {
        GroupRow(first = true) {
            Words("Add favourites to the library", "A song you favourite joins your library if it isn’t there yet.")
            Switch(settings.favouritesToLibrary) { settings.favouritesToLibrary = it }
        }
        GroupRow(first = false) {
            Words("Add playlist songs to the library", "A song you add to a playlist joins your library if it isn’t there yet.")
            Switch(settings.playlistSongsToLibrary) { settings.playlistSongsToLibrary = it }
        }
    }

    Label("Lyrics")
    Group {
        GroupRow(first = true, onClick = { go(SettingsPage.LyricsSites) }) {
            Words("Lyrics sites", settings.lyricsSites.joinToString(", ") { it.label })
            Glyph(Icons.Right, roles.muted, 18.dp)
        }
        GroupRow(first = false) {
            Words("Ask the sites first", "Look for the sites’ lyrics first, even where the service has some.")
            Switch(settings.sitesFirst) { settings.sitesFirst = it }
        }
    }

    Label("Artwork")
    Group(
        if (settings.animatedArtwork && settings.motionQuality == MotionQuality.Automatic) {
            "Automatic lets the player pick each video's size, and takes the lowest one on mobile data."
        } else {
            null
        },
    ) {
        GroupRow(first = true) {
            Words(
                "Animated artwork",
                "Covers and artist headers that move, where Apple made them: on album and artist pages and in Now Playing. It takes a little more battery.",
            )
            Switch(settings.animatedArtwork) { settings.animatedArtwork = it }
        }
        if (settings.animatedArtwork) {
            GroupRow(first = false, onClick = { go(SettingsPage.Motion) }) {
                Words("Quality")
                BasicText(settings.motionQuality.label, style = geist(15.sp).copy(color = roles.sub))
                Glyph(Icons.Right, roles.muted, 18.dp)
            }
        }
    }

    Label("Appearance")
    val appearance by core.appearance.collectAsStateWithLifecycle()
    Group {
        listOf(Appearance.Automatic to "Follows the system", Appearance.Light to null, Appearance.Dark to null).forEachIndexed { i, (a, detail) ->
            GroupRow(first = i == 0, onClick = { core.setAppearance(a) }) {
                Words(a.name, detail)
                if (appearance == a) Glyph(Icons.Check, roles.tint, 18.dp)
            }
        }
    }

    Label("About")
    Group {
        GroupRow(first = true) {
            Words("Music for Android", "GPL-3.0-or-later. The way to Apple Music’s web player comes from Kopuz (EUPL-1.2).")
            BasicText(BuildConfig.VERSION_NAME, style = geist(15.sp).copy(color = roles.sub))
        }
    }
}

/** A service's page: using it, what it gives, and its sign-in. */
@Composable
private fun Service(core: Core, settings: Settings, kind: ProviderKind, onSignIn: (ProviderKind) -> Unit, close: () -> Unit) {
    val state by core.connection(kind).collectAsStateWithLifecycle()
    val actions = LocalActions.current
    val scope = rememberCoroutineScope()
    val connected = state as? Connection.Connected
    Column(Modifier.fillMaxWidth().padding(top = 6.dp, bottom = 22.dp), horizontalAlignment = Alignment.CenterHorizontally) {
        ServiceLogo(kind, 64.dp)
        BasicText(kind.label(), Modifier.padding(top = 10.dp), style = geist(20.sp, FontWeight.Bold).copy(color = roles.text))
        BasicText(
            when (kind) {
                ProviderKind.APPLE_MUSIC -> connected?.let { "Storefront ${it.name} · up to AAC 256 kb/s" } ?: "Your library, playlists and the catalogue, at AAC 256 kb/s."
                ProviderKind.SOUND_CLOUD -> connected?.let { "${it.name} · AAC 160 kb/s, 256 with Go+" } ?: "Your likes and playlists, and the uploaders’ own files where they allow downloads."
                ProviderKind.NAVIDROME -> connected?.let { "${it.name} · files as they are" } ?: "Your server’s files, as they are."
            },
            Modifier.padding(top = 2.dp, start = 20.dp, end = 20.dp),
            style = geist(14.sp).copy(color = roles.sub, textAlign = TextAlign.Center),
        )
        (state as? Connection.Failed)?.let {
            BasicText(it.reason, Modifier.padding(top = 6.dp, start = 20.dp, end = 20.dp), style = geist(13.sp).copy(color = roles.red, textAlign = TextAlign.Center))
        }
    }
    val limits = when (kind) {
        ProviderKind.APPLE_MUSIC -> "Apple Music stops at AAC 256 kb/s for apps other than Apple’s: ALAC is FairPlay-only."
        ProviderKind.SOUND_CLOUD -> "Where the artist allows downloads, Music plays the uploader’s own file instead of the stream."
        ProviderKind.NAVIDROME -> "Files play as they are, up to 24-bit, with no transcoding (format=raw)."
    }
    val signedIn = core.signedIn(kind)
    if (signedIn) {
        var on by remember { mutableStateOf(core.enabled(kind)) }
        Group(limits) {
            GroupRow(first = true) {
                Words("Use ${kind.label()}", "Off: still signed in, and used again once turned on.")
                Switch(on) {
                    on = it
                    scope.launch { core.setEnabled(kind, it) }
                }
            }
            if (kind == ProviderKind.SOUND_CLOUD) {
                GroupRow(first = false) {
                    Words("Play the uploader’s own file")
                    Switch(settings.soundcloudOriginals) { settings.soundcloudOriginals = it }
                }
            }
        }
    }
    Group(if (signedIn) null else limits) {
        GroupRow(first = true, onClick = {
            close()
            onSignIn(kind)
        }) { Words(if (signedIn) "Sign In Again" else "Sign In…", colour = roles.tint) }
        if (signedIn) {
            GroupRow(first = false, onClick = {
                close()
                actions?.sheet = SheetSpec.Confirm(
                    "Sign Out of ${kind.label()}?",
                    if (kind == ProviderKind.NAVIDROME) {
                        "The server’s address and token go from this phone. Your files stay on the server."
                    } else {
                        "Your sign-in goes from this phone. Your library stays on ${kind.label()}."
                    },
                    "Sign Out",
                ) {
                    core.signOut(kind)
                    io.github.honeyslatt.music.ui.connect.forgetCookies(kind)
                }
            }) { Words("Sign Out", colour = roles.red) }
        }
    }
}

/** The lyrics sites, in the order they are asked; each moves up with its arrow. */
@Composable
private fun LyricsSites(settings: Settings) {
    Group("Where a song’s lyrics come from when its service has none, in this order. Apple Music’s own lyrics, word by word, come first for every song it has.") {
        settings.lyricsSites.forEachIndexed { i, site ->
            GroupRow(first = i == 0) {
                Words(site.label, site.about)
                Box(
                    Modifier
                        .size(40.dp)
                        .alpha(if (i > 0) 1f else 0.3f)
                        .then(
                            if (i > 0) {
                                Modifier.press(CircleShape, shrink = 0.92f) {
                                    settings.lyricsSites = settings.lyricsSites.toMutableList().apply { add(i - 1, removeAt(i)) }
                                }
                            } else {
                                Modifier
                            },
                        )
                        .background(roles.surface, CircleShape),
                    contentAlignment = Alignment.Center,
                ) { Glyph(Icons.Up, roles.text, 18.dp) }
            }
        }
    }
}

@Composable
private fun Motion(settings: Settings) {
    Group {
        MotionQuality.entries.forEachIndexed { i, q ->
            GroupRow(first = i == 0, onClick = { settings.motionQuality = q }) {
                Words(q.label)
                if (settings.motionQuality == q) Glyph(Icons.Check, roles.tint, 18.dp)
            }
        }
    }
}
