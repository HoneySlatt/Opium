package io.github.honeyslatt.music.ui.detail

import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.getValue
import androidx.compose.runtime.setValue
import androidx.compose.runtime.produceState
import androidx.compose.ui.graphics.Color
import io.github.honeyslatt.music.core.model.Artwork
import io.github.honeyslatt.music.core.pageColour
import io.github.honeyslatt.music.core.washColours
import io.github.honeyslatt.music.ui.components.at
import io.github.honeyslatt.music.ui.theme.LocalRoles
import io.github.honeyslatt.music.ui.theme.Roles
import io.github.honeyslatt.music.ui.theme.roles
import java.util.concurrent.ConcurrentHashMap
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import okhttp3.OkHttpClient
import okhttp3.Request

/** Each cover's colours, per theme, worked out once from a small copy. */
object CoverColours {
    /** The small copies kept on disk, as the services allow: known again at the next launch without the network. */
    var cache: java.io.File? = null

    private val http by lazy {
        OkHttpClient.Builder().apply { cache?.let { cache(okhttp3.Cache(java.io.File(it, "cover-colours"), 16L * 1024 * 1024)) } }.build()
    }
    private val pages = ConcurrentHashMap<Pair<String, Boolean>, Int>()
    private val washes = ConcurrentHashMap<Pair<String, Boolean>, List<Int>>()

    private fun bytes(url: String): ByteArray? = runCatching {
        http.newCall(Request.Builder().url(io.github.honeyslatt.music.data.Covers.sign(url)).build()).execute().use { it.body?.bytes() }
    }.getOrNull()

    private fun colour(rgb: Int) = Color(0xFF000000.toInt() or rgb)

    /** The page colour, if worked out already. */
    fun knownPage(url: String?, light: Boolean): Color? = url?.let { pages[it to light] }?.let(::colour)

    /** The wash colours, if worked out already. */
    fun knownWash(url: String?, light: Boolean): List<Color>? = url?.let { washes[it to light] }?.map(::colour)

    suspend fun page(url: String, light: Boolean): Color? {
        pages[url to light]?.let { return colour(it) }
        val rgb = withContext(Dispatchers.IO) { bytes(url)?.let { pageColour(it, light)?.toInt() } } ?: return null
        pages[url to light] = rgb
        return colour(rgb)
    }

    /** Now Playing's three wash colours. */
    suspend fun wash(url: String, light: Boolean): List<Color>? {
        washes[url to light]?.let { return it.map(::colour) }
        val rgb = withContext(Dispatchers.IO) {
            bytes(url)?.let { b -> washColours(b, light)?.map { it.toInt() } }
        } ?: return null
        washes[url to light] = rgb
        return rgb.map(::colour)
    }
}

/**
 * The page colour of `artwork` (the base colour until it is known),
 * fading from the one before as the cover changes rather than through
 * the base.
 */
@Composable
fun rememberPageColour(artwork: Artwork?): Color {
    val base = roles.base
    val light = !roles.dark
    val url = artwork?.at(120)
    var colour by androidx.compose.runtime.remember { androidx.compose.runtime.mutableStateOf(CoverColours.knownPage(url, light) ?: base) }
    androidx.compose.runtime.LaunchedEffect(url, light) {
        colour = url?.let { CoverColours.page(it, light) } ?: base
    }
    return androidx.compose.animation.animateColorAsState(colour, androidx.compose.animation.core.tween(450), label = "page").value
}

/** Favourite Songs has no cover: its page takes Music's pink, dimmed. */
fun favouritesPage(dark: Boolean) = if (dark) Color(0xFF3B1A21) else Color(0xFFFCE9EC)

/**
 * The roles on a page tinted by its cover, as on iOS 26.4: the page's
 * colour as the base, text only white or black (no accent), and fills of
 * text at 9 % instead of the surfaces.
 */
fun Roles.tintedBy(page: Color): Roles {
    val fill = text.copy(alpha = 0.09f)
    return copy(base = page, tint = text, surface = fill, surface2 = fill)
}

@Composable
fun Tinted(page: Color, content: @Composable () -> Unit) {
    CompositionLocalProvider(LocalRoles provides roles.tintedBy(page), content = content)
}
