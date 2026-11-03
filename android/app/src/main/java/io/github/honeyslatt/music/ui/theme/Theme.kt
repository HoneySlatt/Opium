package io.github.honeyslatt.music.ui.theme

import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.Immutable
import androidx.compose.runtime.staticCompositionLocalOf
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.lerp
import androidx.compose.ui.graphics.luminance
import io.github.honeyslatt.music.core.model.Layout
import io.github.honeyslatt.music.core.model.MobileLayout
import io.github.honeyslatt.music.core.model.Palette
import io.github.honeyslatt.music.core.model.ThemeFile

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
    /** Material's tonal roles, which M3E draws with. */
    val primary: Color,
    val onPrimary: Color,
    val secondaryContainer: Color,
    val onSecondaryContainer: Color,
    val tertiaryContainer: Color,
    val onTertiaryContainer: Color,
) {
    /** Hairlines: the text at 9 %. */
    val line get() = text.copy(alpha = 0.09f)

    /** What a press lays over a row or a button. */
    val press get() = text.copy(alpha = 0.08f)

    /** Text on `accent`: white, black past a quarter of white's luminance,
     *  as on Spotify's green. */
    val onAccent get() = if (accent.luminance() > 0.25f) Color.Black else Color.White
}

/** A theme's colours as the phone's roles: its own when it gives them,
 *  else derived from the desktop's. */
fun rolesOf(p: Palette): Roles {
    fun c(v: UInt) = Color(0xFF000000.toInt() or v.toInt())
    val base = c(p.base)
    val sub = c(p.subtext0)
    val tonal = io.github.honeyslatt.music.core.tonal(p)
    return Roles(
        dark = !p.light,
        base = base,
        grouped = p.grouped?.let(::c) ?: base,
        cell = p.cell?.let(::c) ?: c(p.mantle),
        surface = c(p.surface0),
        surface2 = c(p.surface1),
        surface3 = c(p.surface2),
        capsule = p.capsule?.let(::c) ?: c(p.surface0),
        text = c(p.text),
        sub = sub,
        muted = p.muted?.let(::c) ?: lerp(sub, base, 0.22f),
        accent = c(p.accent),
        tint = p.tint?.let(::c) ?: c(p.accent),
        mauve = c(p.mauve),
        red = c(p.red),
        green = c(p.green),
        yellow = c(p.yellow),
        primary = c(tonal.primary),
        onPrimary = c(tonal.onPrimary),
        secondaryContainer = c(tonal.secondaryContainer),
        onSecondaryContainer = c(tonal.onSecondaryContainer),
        tertiaryContainer = c(tonal.tertiaryContainer),
        onTertiaryContainer = c(tonal.onTertiaryContainer),
    )
}

/** The theme's layout: the choices both apps make, and the phone's own. */
@Immutable
data class Look(val layout: Layout, val mobile: MobileLayout)

val LocalLook = staticCompositionLocalOf<Look> { error("no theme") }

/** The layout in use. */
val look: Look
    @Composable get() = LocalLook.current

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
    // As the core derives them of the pink.
    primary = Color(0xFFFFA6A4),
    onPrimary = Color(0xFF490007),
    secondaryContainer = Color(0xFF59302F),
    onSecondaryContainer = Color(0xFFFFDBD9),
    tertiaryContainer = Color(0xFF553E0E),
    onTertiaryContainer = Color(0xFFF2E2C7),
)

/** Settings → Appearance. */
enum class Appearance { Automatic, Light, Dark }

val LocalRoles = staticCompositionLocalOf { DarkRoles }

/** The roles in use, as `Theme.roles` would be in Material. */
val roles: Roles
    @Composable get() = LocalRoles.current

/** Whether a theme follows Settings → Appearance: only one with light
 *  colours beside its dark ones does. */
val ThemeFile.hasBoth get() = lightColors != null

@Composable
fun MusicTheme(appearance: Appearance, theme: ThemeFile, content: @Composable () -> Unit) {
    val light = theme.lightColors
    val dark = when {
        // A theme of one set keeps it.
        light == null -> !theme.colors.light
        appearance == Appearance.Automatic -> isSystemInDarkTheme()
        else -> appearance == Appearance.Dark
    }
    val roles = rolesOf(if (dark || light == null) theme.colors else light)
    CompositionLocalProvider(
        LocalRoles provides roles,
        LocalLook provides Look(theme.layout, theme.mobile),
        content = content,
    )
}
