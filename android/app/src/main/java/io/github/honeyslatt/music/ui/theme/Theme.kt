package io.github.honeyslatt.music.ui.theme

import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.Immutable
import androidx.compose.runtime.staticCompositionLocalOf
import androidx.compose.ui.graphics.Color

/** The design's colour roles (DESIGN.md, Theme), one set per appearance. */
@Immutable
data class Roles(
    val dark: Boolean,
    val base: Color,
    val grouped: Color,
    val cell: Color,
    val surface: Color,
    val surface2: Color,
    val surface3: Color,
    val capsule: Color,
    val text: Color,
    val sub: Color,
    val muted: Color,
    val accent: Color,
    val tint: Color,
    val mauve: Color,
    val red: Color,
    val green: Color,
    val yellow: Color,
) {
    /** Hairlines: the text at 9 %. */
    val line get() = text.copy(alpha = 0.09f)

    /** What a press lays over a row or a button. */
    val press get() = text.copy(alpha = 0.08f)

    /** Text on `accent`. */
    val onAccent get() = Color.White
}

val DarkRoles = Roles(
    dark = true,
    base = Color(0xFF161616),
    grouped = Color(0xFF161616),
    cell = Color(0xFF1F1F1F),
    surface = Color(0xFF222222),
    surface2 = Color(0xFF2A2A2A),
    surface3 = Color(0xFF3A3A3A),
    capsule = Color(0xFF232323),
    text = Color(0xFFF2F4F8),
    sub = Color(0xFFB6B8BB),
    muted = Color(0xFF8A8B8E),
    accent = Color(0xFFFA2D48),
    tint = Color(0xFFFA2D48),
    mauve = Color(0xFFFB5C74),
    red = Color(0xFFFF453A),
    green = Color(0xFF4ADE80),
    yellow = Color(0xFFFBBF24),
)

val LightRoles = Roles(
    dark = false,
    base = Color(0xFFFFFFFF),
    grouped = Color(0xFFF2F2F7),
    cell = Color(0xFFFFFFFF),
    surface = Color(0xFFF0F0F2),
    surface2 = Color(0xFFE6E6EA),
    surface3 = Color(0xFFD8D8DD),
    capsule = Color(0xFFFFFFFF),
    text = Color(0xFF161616),
    sub = Color(0xFF6B6B70),
    muted = Color(0xFF8A8A8E),
    accent = Color(0xFFFA2D48),
    tint = Color(0xFFE0193A),
    mauve = Color(0xFFFB5C74),
    red = Color(0xFFFF3B30),
    green = Color(0xFF15803D),
    yellow = Color(0xFFB45309),
)

/** Settings → Appearance. */
enum class Appearance { Automatic, Light, Dark }

val LocalRoles = staticCompositionLocalOf { DarkRoles }

/** The roles in use, as `Theme.roles` would be in Material. */
val roles: Roles
    @Composable get() = LocalRoles.current

@Composable
fun MusicTheme(appearance: Appearance, content: @Composable () -> Unit) {
    val dark = when (appearance) {
        Appearance.Automatic -> isSystemInDarkTheme()
        Appearance.Light -> false
        Appearance.Dark -> true
    }
    CompositionLocalProvider(LocalRoles provides if (dark) DarkRoles else LightRoles, content = content)
}
