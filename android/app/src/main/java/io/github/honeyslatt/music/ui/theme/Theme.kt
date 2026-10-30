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
    base = Color(0xFF0B0A0F),
    grouped = Color(0xFF0B0A0F),
    cell = Color(0xFF17151E),
    surface = Color(0xFF1A1822),
    surface2 = Color(0xFF25222F),
    surface3 = Color(0xFF322E3D),
    capsule = Color(0xFF1C1A24),
    text = Color(0xFFF5F3FF),
    sub = Color(0xFFAAA5BA),
    muted = Color(0xFF8A8598),
    accent = Color(0xFF7C3AED),
    tint = Color(0xFFA78BFA),
    mauve = Color(0xFFA78BFA),
    red = Color(0xFFF43F5E),
    green = Color(0xFF4ADE80),
    yellow = Color(0xFFFBBF24),
)

val LightRoles = Roles(
    dark = false,
    base = Color(0xFFFFFFFF),
    grouped = Color(0xFFF2F1F6),
    cell = Color(0xFFFFFFFF),
    surface = Color(0xFFF0EFF5),
    surface2 = Color(0xFFE7E5EE),
    surface3 = Color(0xFFD9D6E2),
    capsule = Color(0xFFFFFFFF),
    text = Color(0xFF17141E),
    sub = Color(0xFF6A6575),
    muted = Color(0xFF8B8696),
    accent = Color(0xFF7C3AED),
    tint = Color(0xFF6D28D9),
    mauve = Color(0xFFA78BFA),
    red = Color(0xFFE11D48),
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
