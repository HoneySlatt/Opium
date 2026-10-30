package io.github.honeyslatt.music.ui.detail

import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.getValue
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
    private val http = OkHttpClient()
    private val pages = ConcurrentHashMap<Pair<String, Boolean>, Int>()
    private val washes = ConcurrentHashMap<Pair<String, Boolean>, List<Int>>()

    private fun bytes(url: String): ByteArray? = runCatching {
        http.newCall(Request.Builder().url(url).build()).execute().use { it.body?.bytes() }
    }.getOrNull()

    private fun colour(rgb: Int) = Color(0xFF000000.toInt() or rgb)

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

/** The page colour of `artwork` (the base colour until it is known). */
@Composable
fun rememberPageColour(artwork: Artwork?): Color {
    val base = roles.base
    val light = !roles.dark
    val url = artwork?.at(120)
    val colour by produceState(base, url, light) {
        value = url?.let { CoverColours.page(it, light) } ?: base
    }
    return colour
}

/** Favourite Songs has no cover: its page takes Music's violet, dimmed. */
fun favouritesPage(dark: Boolean) = if (dark) Color(0xFF231A3B) else Color(0xFFF1EDFA)

/**
 * The roles on a page tinted by its cover, as on iOS 26.4: the page's
 * colour as the base, text only white or black (no violet), and fills of
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
