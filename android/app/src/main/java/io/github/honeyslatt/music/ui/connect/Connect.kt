package io.github.honeyslatt.music.ui.connect

import android.annotation.SuppressLint
import android.media.MediaDrm
import android.webkit.CookieManager
import android.webkit.WebView
import android.webkit.WebViewClient
import androidx.activity.compose.BackHandler
import androidx.compose.animation.core.RepeatMode
import androidx.compose.animation.core.animateFloat
import androidx.compose.animation.core.infiniteRepeatable
import androidx.compose.animation.core.rememberInfiniteTransition
import androidx.compose.animation.core.tween
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.safeDrawing
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.windowInsetsPadding
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.BasicText
import androidx.compose.foundation.text.BasicTextField
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateListOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.scale
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.SolidColor
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.text.input.PasswordVisualTransformation
import androidx.compose.ui.text.input.VisualTransformation
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.compose.ui.viewinterop.AndroidView
import io.github.honeyslatt.music.core.model.ProviderKind
import io.github.honeyslatt.music.data.Connection
import io.github.honeyslatt.music.data.Core
import io.github.honeyslatt.music.data.SoundCloudWeb
import io.github.honeyslatt.music.ui.Icons
import io.github.honeyslatt.music.ui.components.Glyph
import io.github.honeyslatt.music.ui.components.Group
import io.github.honeyslatt.music.ui.components.GroupRow
import io.github.honeyslatt.music.ui.components.ServiceLogo
import io.github.honeyslatt.music.ui.components.Sheet
import io.github.honeyslatt.music.ui.components.SheetButton
import io.github.honeyslatt.music.ui.components.label
import io.github.honeyslatt.music.ui.components.press
import io.github.honeyslatt.music.ui.theme.geist
import io.github.honeyslatt.music.ui.theme.roles
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch

/** A check of a sign-in, done or not yet, and what it found. */
private data class Check(val title: String, val detail: String, val state: State) {
    enum class State { Waiting, Done, Failed }
}

internal enum class Step { Intro, Web, Checks }

/** Which service is being connected, and how far it has got. */
class Connecting {
    var service by mutableStateOf<ProviderKind?>(null)
        private set
    internal var step by mutableStateOf(Step.Intro)

    fun start(kind: ProviderKind) {
        step = Step.Intro
        service = kind
    }

    fun close() {
        service = null
    }
}

/** A service's sign-in page and where its token is left: on a phone, SoundCloud's mobile site keeps it on its own host. */
private class Site(val url: String, val origins: List<String>, val token: String, val also: List<String> = emptyList())

private val ProviderKind.site
    get() = when (this) {
        ProviderKind.APPLE_MUSIC -> Site("https://music.apple.com/signin", listOf("https://music.apple.com"), "media-user-token")
        ProviderKind.SOUND_CLOUD -> Site(
            "https://soundcloud.com/signin",
            listOf("https://soundcloud.com", "https://m.soundcloud.com"),
            "oauth_token",
            listOf("oauth_refresh_token"),
        )
        ProviderKind.NAVIDROME -> error("Navidrome signs in with a form")
        ProviderKind.MUSIC -> error("Music's playlists need no sign-in")
    }

/** The sheets and the sign-in page of the service being connected, over everything. */
@Composable
fun ConnectHost(connecting: Connecting, core: Core) {
    val kind = connecting.service
    // The last one stays drawn while its sheet slides away.
    var shown by remember { mutableStateOf<ProviderKind?>(null) }
    if (kind != null) shown = kind
    val k = shown ?: return
    val checks = remember(k) { mutableStateListOf<Check>() }
    val scope = rememberCoroutineScope()
    val open = kind != null && connecting.step != Step.Web
    val title = k.label()
    val done = checks.isNotEmpty() && checks.all { it.state == Check.State.Done }
    val failed = checks.any { it.state == Check.State.Failed }
    Sheet(
        open,
        title,
        connecting::close,
        left = { SheetButton("Cancel", onClick = connecting::close) },
        right = if (connecting.step == Step.Checks) {
            { SheetButton("Done", primary = true, enabled = done, onClick = connecting::close) }
        } else {
            null
        },
    ) {
        when (connecting.step) {
            Step.Checks -> {
                Spacer(Modifier.height(10.dp))
                Checks(checks)
                if (failed) {
                    Spacer(Modifier.height(14.dp))
                    BigButton("Try Again") {
                        checks.clear()
                        connecting.step = Step.Intro
                    }
                }
            }
            else -> when (k) {
                ProviderKind.NAVIDROME -> NavidromeForm { url, user, password ->
                    checks.clear()
                    checks += Check("Signed in", "Asking the server…", Check.State.Waiting)
                    connecting.step = Step.Checks
                    scope.launch {
                        val got = core.signInToNavidrome(url, user, password)
                        checks.clear()
                        if (got is Connection.Connected) {
                            checks += Check("Signed in", "As ${got.name}, with a salted token: the password isn’t kept", Check.State.Done)
                            checks += Check("Original files", "Files play as they are (format=raw), with no transcoding", Check.State.Done)
                        } else {
                            checks += Check("Couldn’t sign in", (got as? Connection.Failed)?.reason ?: "", Check.State.Failed)
                        }
                    }
                }
                else -> Intro(k) { connecting.step = Step.Web }
            }
        }
    }
    if (kind != null && connecting.step == Step.Web) {
        WebSignIn(k, onCancel = { connecting.step = Step.Intro }) { cookies ->
            connecting.step = Step.Checks
            checks.clear()
            scope.launch { verify(k, cookies, core, checks) }
        }
    }
}

/** What happens after the page: each check ticked as it is done. */
private suspend fun verify(kind: ProviderKind, cookies: Map<String, String>, core: Core, checks: MutableList<Check>) {
    fun set(i: Int, c: Check) {
        if (i < checks.size) checks[i] = c else checks += c
    }
    val token = cookies.getValue(kind.site.token)
    when (kind) {
        ProviderKind.APPLE_MUSIC -> {
            set(0, Check("Account token saved", "Kept in Android Keystore, readable by Music only", Check.State.Waiting))
            set(1, Check("Web player token", "Asking Apple…", Check.State.Waiting))
            set(2, Check("Storefront", "", Check.State.Waiting))
            set(3, Check("Widevine ready", "MediaDrm on this phone", Check.State.Waiting))
            val got = core.signedInToApple(token)
            set(0, checks[0].copy(state = Check.State.Done))
            if (got is Connection.Connected) {
                set(1, Check("Web player token", "Accepted by Apple Music", Check.State.Done))
                set(2, Check("Storefront", got.name, Check.State.Done))
            } else {
                set(1, Check("Web player token", (got as? Connection.Failed)?.reason ?: "Refused", Check.State.Failed))
                set(2, Check("Storefront", "Not known", Check.State.Failed))
            }
            delay(300)
            set(3, widevine())
        }
        ProviderKind.SOUND_CLOUD -> {
            set(0, Check("Tokens saved", "Kept in Android Keystore, readable by Music only", Check.State.Waiting))
            set(1, Check("Signed in", "Asking SoundCloud…", Check.State.Waiting))
            val got = core.signedInToSoundCloud(token, cookies["oauth_refresh_token"])
            set(0, checks[0].copy(state = Check.State.Done))
            if (got is Connection.Connected) {
                set(1, Check("Signed in", "As ${got.name}", Check.State.Done))
                set(2, Check("Stream quality", "AAC 160 kb/s, 256 with Go+; the uploader’s own file where they allow downloads", Check.State.Done))
            } else {
                set(1, Check("Signed in", (got as? Connection.Failed)?.reason ?: "Refused", Check.State.Failed))
            }
        }
        ProviderKind.NAVIDROME, ProviderKind.MUSIC -> {}
    }
}

/** Whether the phone's Widevine is there, and at which level. */
private fun widevine(): Check = try {
    val drm = MediaDrm(androidx.media3.common.C.WIDEVINE_UUID)
    val level = runCatching { drm.getPropertyString("securityLevel") }.getOrDefault("")
    drm.close()
    Check("Widevine ready", "MediaDrm on this phone${if (level.isNotEmpty()) ", $level" else ""}", Check.State.Done)
} catch (e: Exception) {
    Check("Widevine ready", "This phone has no Widevine: Apple Music's songs can't play", Check.State.Failed)
}

@Composable
private fun Checks(checks: List<Check>) {
    Group {
        checks.forEachIndexed { i, c ->
            GroupRow(first = i == 0) {
                val fill = when (c.state) {
                    Check.State.Done -> roles.green
                    Check.State.Failed -> roles.red
                    Check.State.Waiting -> roles.surface3
                }
                val breath by rememberInfiniteTransition(label = "wait").animateFloat(0.6f, 1f, infiniteRepeatable(tween(700), RepeatMode.Reverse), label = "breath")
                Box(
                    Modifier.size(26.dp).scale(if (c.state == Check.State.Waiting) breath else 1f).background(fill, CircleShape),
                    contentAlignment = Alignment.Center,
                ) {
                    when (c.state) {
                        Check.State.Done -> Glyph(Icons.Check, Color.White, 14.dp)
                        Check.State.Failed -> Glyph(Icons.X, Color.White, 14.dp)
                        Check.State.Waiting -> {}
                    }
                }
                Column(Modifier.weight(1f)) {
                    BasicText(c.title, style = geist(16.sp).copy(color = roles.text))
                    if (c.detail.isNotEmpty()) BasicText(c.detail, style = geist(13.sp).copy(color = roles.sub, lineHeight = 17.5.sp))
                }
            }
        }
    }
}

@Composable
private fun Intro(kind: ProviderKind, onGo: () -> Unit) {
    val (heading, words, button) = when (kind) {
        ProviderKind.APPLE_MUSIC -> Triple(
            "Sign in to Apple Music",
            "Apple’s sign-in page opens inside Music. Once you’re in, it closes on its own and your token stays on this phone.",
            "Continue to Apple",
        )
        else -> Triple(
            "Sign in to SoundCloud",
            "SoundCloud’s sign-in page opens inside Music. Once you’re in, it closes on its own and your tokens stay on this phone. Google’s sign-in doesn’t work inside apps: use your email, or Apple.",
            "Continue to SoundCloud",
        )
    }
    Column(Modifier.fillMaxWidth(), horizontalAlignment = Alignment.CenterHorizontally) {
        Spacer(Modifier.height(18.dp))
        ServiceLogo(kind, 64.dp)
        Spacer(Modifier.height(14.dp))
        BasicText(heading, style = geist(18.sp, FontWeight.SemiBold).copy(color = roles.text, textAlign = TextAlign.Center))
        BasicText(words, Modifier.padding(horizontal = 12.dp, vertical = 4.dp), style = geist(14.5.sp).copy(color = roles.sub, textAlign = TextAlign.Center, lineHeight = 21.sp))
        Spacer(Modifier.height(26.dp))
        BigButton(button, onClick = onGo)
    }
}

/** The design's large accent button; grey and still until it can be used. */
@Composable
fun BigButton(label: String, enabled: Boolean = true, onClick: () -> Unit) {
    val shape = RoundedCornerShape(14.dp)
    Box(
        Modifier
            .fillMaxWidth()
            .height(52.dp)
            .then(if (enabled) Modifier.press(shape, shrink = 0.97f, onClick = onClick) else Modifier)
            .background(if (enabled) roles.accent else roles.surface2, shape),
        contentAlignment = Alignment.Center,
    ) {
        BasicText(label, style = geist(16.5.sp, FontWeight.SemiBold).copy(color = if (enabled) roles.onAccent else roles.muted))
    }
}

/**
 * Navidrome: the address, the user and the password. The password goes to
 * the core when Connect is tapped, which keeps only a salted token.
 */
@Composable
private fun NavidromeForm(onConnect: (String, String, String) -> Unit) {
    var url by remember { mutableStateOf("") }
    var user by remember { mutableStateOf("") }
    var password by remember { mutableStateOf("") }
    var error by remember { mutableStateOf<String?>(null) }
    Column(Modifier.fillMaxWidth(), horizontalAlignment = Alignment.CenterHorizontally) {
        Spacer(Modifier.height(18.dp))
        ServiceLogo(ProviderKind.NAVIDROME, 64.dp)
        Spacer(Modifier.height(18.dp))
    }
    Group(footer = error ?: "Away from home, Music falls back to the Apple Music or SoundCloud copy when the server can’t be reached.") {
        GroupRow(first = true) { Field(url, "Server address, e.g. http://192.168.1.20:4533", KeyboardType.Uri) { url = it.trim(); error = null } }
        GroupRow(first = false) { Field(user, "Username", KeyboardType.Text) { user = it } }
        GroupRow(first = false) { Field(password, "Password", KeyboardType.Password, secret = true) { password = it } }
    }
    BigButton("Connect", enabled = url.isNotBlank() && user.isNotBlank() && password.isNotEmpty()) {
        if (!url.startsWith("http://") && !url.startsWith("https://")) {
            error = "Start the address with http:// or https://"
            return@BigButton
        }
        val typed = password
        // Not kept here either, once handed over.
        password = ""
        onConnect(url, user.trim(), typed)
    }
}

@Composable
private fun androidx.compose.foundation.layout.RowScope.Field(
    value: String,
    placeholder: String,
    keyboard: KeyboardType,
    secret: Boolean = false,
    onChange: (String) -> Unit,
) {
    val style = geist(16.sp).copy(color = roles.text)
    BasicTextField(
        value,
        onChange,
        Modifier.weight(1f),
        textStyle = style,
        singleLine = true,
        cursorBrush = SolidColor(roles.accent),
        keyboardOptions = KeyboardOptions(keyboardType = keyboard, autoCorrectEnabled = false),
        visualTransformation = if (secret) PasswordVisualTransformation() else VisualTransformation.None,
        decorationBox = { inner ->
            Box {
                if (value.isEmpty()) BasicText(placeholder, style = style.copy(color = roles.muted))
                inner()
            }
        },
    )
}

/**
 * A service's own sign-in page, full screen, until its cookies hold the
 * token: then it closes on its own, with what it found.
 */
@SuppressLint("SetJavaScriptEnabled")
@Composable
private fun WebSignIn(kind: ProviderKind, onCancel: () -> Unit, onDone: (Map<String, String>) -> Unit) {
    val site = kind.site
    BackHandler(onBack = onCancel)
    LaunchedEffect(kind) {
        // Ten minutes, then it gives up.
        repeat(1_200) {
            for (origin in site.origins) {
                val cookies = CookieManager.getInstance().getCookie(origin).orEmpty()
                    .split(';')
                    .map { it.trim() }
                    .mapNotNull { it.split('=', limit = 2).takeIf { p -> p.size == 2 }?.let { (k, v) -> k to v } }
                    .toMap()
                if (cookies[site.token].orEmpty().isNotEmpty()) {
                    CookieManager.getInstance().flush()
                    return@LaunchedEffect onDone(cookies.filterKeys { it == site.token || it in site.also })
                }
            }
            delay(500)
        }
        onCancel()
    }
    Column(Modifier.fillMaxSize().background(roles.base).windowInsetsPadding(WindowInsets.safeDrawing)) {
        Row(
            Modifier.fillMaxWidth().height(56.dp).padding(horizontal = 14.dp),
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.spacedBy(12.dp),
        ) {
            SheetButton("Cancel", onClick = onCancel)
            BasicText(kind.label(), Modifier.weight(1f), style = geist(16.5.sp, FontWeight.SemiBold).copy(color = roles.text, textAlign = TextAlign.Center))
            Spacer(Modifier.size(72.dp, 1.dp))
        }
        AndroidView(
            factory = {
                WebView(it).apply {
                    settings.javaScriptEnabled = true
                    settings.domStorageEnabled = true
                    if (kind == ProviderKind.SOUND_CLOUD) settings.userAgentString = SoundCloudWeb.desktopAgent(it)
                    webViewClient = WebViewClient()
                    loadUrl(site.url)
                }
            },
            modifier = Modifier.fillMaxSize(),
        )
    }
}

/**
 * SoundCloud, full screen, for its bot guard's check: it shows it when a
 * song is liked there. Done (or back) sends the change again.
 */
@SuppressLint("SetJavaScriptEnabled")
@Composable
fun SoundCloudCheck(onDone: () -> Unit) {
    BackHandler(onBack = onDone)
    Column(Modifier.fillMaxSize().background(roles.base).windowInsetsPadding(WindowInsets.safeDrawing)) {
        Row(
            Modifier.fillMaxWidth().height(56.dp).padding(horizontal = 14.dp),
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.spacedBy(12.dp),
        ) {
            Spacer(Modifier.size(72.dp, 1.dp))
            BasicText("Bot check", Modifier.weight(1f), style = geist(16.5.sp, FontWeight.SemiBold).copy(color = roles.text, textAlign = TextAlign.Center))
            SheetButton("Done", onClick = onDone)
        }
        BasicText(
            "SoundCloud wants a check: like a song here and pass it, then tap Done.",
            Modifier.fillMaxWidth().padding(horizontal = 20.dp, vertical = 8.dp),
            style = geist(14.sp).copy(color = roles.muted, textAlign = TextAlign.Center),
        )
        AndroidView(
            factory = {
                WebView(it).apply {
                    settings.javaScriptEnabled = true
                    settings.domStorageEnabled = true
                    settings.userAgentString = SoundCloudWeb.desktopAgent(it)
                    webViewClient = WebViewClient()
                    loadUrl("https://soundcloud.com/")
                }
            },
            modifier = Modifier.fillMaxSize(),
        )
    }
}

/** A service's cookies go when it is signed out of, so the next sign-in starts afresh. */
fun forgetCookies(kind: ProviderKind) {
    if (kind == ProviderKind.NAVIDROME) return
    val manager = CookieManager.getInstance()
    for (origin in kind.site.origins) {
        val domain = origin.removePrefix("https://").removePrefix("music.").removePrefix("m.")
        manager.getCookie(origin).orEmpty().split(';').map { it.trim().substringBefore('=') }.filter { it.isNotEmpty() }.forEach { name ->
            manager.setCookie(origin, "$name=; Max-Age=0; Path=/")
            manager.setCookie(origin, "$name=; Max-Age=0; Path=/; Domain=$domain")
        }
    }
    manager.flush()
}
