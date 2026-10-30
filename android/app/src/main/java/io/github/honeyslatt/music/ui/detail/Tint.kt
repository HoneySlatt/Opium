package io.github.honeyslatt.music.ui.detail

import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.getValue
import androidx.compose.runtime.produceState
import androidx.compose.ui.graphics.Color
import io.github.honeyslatt.music.core.model.Artwork
import io.github.honeyslatt.music.core.pageColour
import io.github.honeyslatt.music.ui.components.at
import io.github.honeyslatt.music.ui.theme.LocalRoles
import io.github.honeyslatt.music.ui.theme.Roles
import io.github.honeyslatt.music.ui.theme.roles
import java.util.concurrent.ConcurrentHashMap
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import okhttp3.OkHttpClient
import okhttp3.Request

/** Each cover's page colour, per theme, worked out once. */
private object PageColours {
    private val http = OkHttpClient()
    private val known = ConcurrentHashMap<Pair<String, Boolean>, Int>()

    suspend fun of(url: String, light: Boolean): Color? {
        known[url to light]?.let { return Color(0xFF000000.toInt() or it) }
        val rgb = withContext(Dispatchers.IO) {
            runCatching {
                http.newCall(Request.Builder().url(url).build()).execute().use { it.body?.bytes() }
            }.getOrNull()?.let { pageColour(it, light)?.toInt() }
        } ?: return null
        known[url to light] = rgb
        return Color(0xFF000000.toInt() or rgb)
    }
}

/** The page colour of `artwork` (the base colour until it is known). */
@Composable
fun rememberPageColour(artwork: Artwork?): Color {
    val base = roles.base
    val light = !roles.dark
    val url = artwork?.at(120)
    val colour by produceState(base, url, light) {
        value = url?.let { PageColours.of(it, light) } ?: base
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
