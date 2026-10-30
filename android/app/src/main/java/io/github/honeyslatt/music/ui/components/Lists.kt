package io.github.honeyslatt.music.ui.components

import androidx.compose.foundation.Image
import androidx.compose.foundation.background
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.aspectRatio
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.BasicText
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.drawBehind
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import io.github.honeyslatt.music.R
import io.github.honeyslatt.music.core.model.Album
import io.github.honeyslatt.music.core.model.Artwork
import io.github.honeyslatt.music.core.model.ProviderKind
import io.github.honeyslatt.music.core.model.Track
import io.github.honeyslatt.music.ui.Icons
import io.github.honeyslatt.music.ui.theme.geist
import io.github.honeyslatt.music.ui.theme.roles

/** A root page's 32 sp title, level with the bar's buttons. */
@Composable
fun LargeTitle(text: String, modifier: Modifier = Modifier, inset: androidx.compose.ui.unit.Dp = 20.dp) {
    Box(modifier.fillMaxWidth().height(52.dp).padding(horizontal = inset), contentAlignment = Alignment.CenterStart) {
        Line(text, geist(32.sp, FontWeight.Bold, -0.02f).copy(color = roles.text))
    }
}

/** A section's 21 sp title, with a chevron when it opens the whole of it. */
@Composable
fun SectionHeader(title: String, onOpen: (() -> Unit)? = null) {
    Row(
        Modifier
            .padding(start = 20.dp, end = 20.dp, top = 26.dp, bottom = 12.dp)
            .then(if (onOpen != null) Modifier.press(onClick = onOpen) else Modifier),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(2.dp),
    ) {
        Line(title, geist(21.sp, FontWeight.Bold, -0.01f).copy(color = roles.text))
        if (onOpen != null) Glyph(Icons.Right, roles.muted, 20.dp)
    }
}

/** A thin line under a row, from `inset` to the edge. */
fun Modifier.underline(color: androidx.compose.ui.graphics.Color) = drawBehind {
    drawLine(color, Offset(0f, size.height - 0.5f), Offset(size.width, size.height - 0.5f), 1.dp.toPx())
}

/** A service's logo. */
@Composable
fun ServiceLogo(provider: ProviderKind, size: androidx.compose.ui.unit.Dp) {
    val res = when (provider) {
        ProviderKind.APPLE_MUSIC -> R.drawable.logo_apple_music
        ProviderKind.SOUND_CLOUD -> R.drawable.logo_soundcloud
        ProviderKind.NAVIDROME -> R.drawable.logo_navidrome
    }
    Image(painterResource(res), contentDescription = null, modifier = Modifier.size(size))
}

fun ProviderKind.label() = when (this) {
    ProviderKind.APPLE_MUSIC -> "Apple Music"
    ProviderKind.SOUND_CLOUD -> "SoundCloud"
    ProviderKind.NAVIDROME -> "Navidrome"
}

/** The source filter: All, then each service. `null` is All. */
@Composable
fun SourceChips(selected: ProviderKind?, services: List<ProviderKind>, onSelect: (ProviderKind?) -> Unit) {
    Row(
        Modifier
            .horizontalScroll(rememberScrollState())
            .padding(horizontal = 20.dp, vertical = 4.dp),
        horizontalArrangement = Arrangement.spacedBy(8.dp),
    ) {
        Chip("All", null, selected == null) { onSelect(null) }
        services.forEach { kind -> Chip(kind.label(), kind, selected == kind) { onSelect(kind) } }
    }
}

@Composable
private fun Chip(label: String, logo: ProviderKind?, on: Boolean, onClick: () -> Unit) {
    val shape = RoundedCornerShape(17.dp)
    Row(
        Modifier
            .height(34.dp)
            .press(shape, shrink = 0.96f, onClick = onClick)
            .background(if (on) roles.accent else roles.surface, shape)
            .padding(horizontal = 14.dp),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(7.dp),
    ) {
        if (logo != null) ServiceLogo(logo, 18.dp)
        Line(label, geist(14.sp, FontWeight.Medium).copy(color = if (on) roles.onAccent else roles.text))
    }
}

/** A row of the library's root: an icon in `tint`, a label, a chevron. */
@Composable
fun LibraryRow(icon: ImageVector, label: String, last: Boolean, onClick: () -> Unit) {
    Row(
        Modifier.fillMaxWidth().height(52.dp).press(onClick = onClick).padding(start = 20.dp),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(14.dp),
    ) {
        Box(Modifier.width(26.dp), contentAlignment = Alignment.Center) { Glyph(icon, roles.tint, 22.dp) }
        Row(
            Modifier
                .weight(1f)
                .fillMaxHeight()
                .then(if (last) Modifier else Modifier.underline(roles.line))
                .padding(end = 18.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Line(label, geist(18.sp).copy(color = roles.text), Modifier.weight(1f))
            Glyph(Icons.Right, roles.muted, 18.dp)
        }
    }
}

/**
 * A row with artwork (round for artists), a title, a subtitle that may
 * start with a service's logo, and a chevron.
 */
@Composable
fun ItemRow(
    artwork: Artwork?,
    title: String,
    subtitle: String? = null,
    logo: ProviderKind? = null,
    round: Boolean = false,
    star: Boolean = false,
    leading: (@Composable () -> Unit)? = null,
    onLongClick: (() -> Unit)? = null,
    onClick: () -> Unit,
) {
    val art = if (round) 48.dp else 56.dp
    Row(
        Modifier.fillMaxWidth().heightIn(min = 68.dp).press(onLongClick = onLongClick, onClick = onClick).padding(start = 20.dp),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(14.dp),
    ) {
        if (leading != null) {
            Box(Modifier.size(art), contentAlignment = Alignment.Center) { leading() }
        } else {
            Cover(artwork, art, if (round) CircleShape else RoundedCornerShape(8.dp), Modifier.size(art))
        }
        Row(
            Modifier.weight(1f).heightIn(min = 68.dp).underline(roles.line).padding(end = 16.dp),
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.spacedBy(8.dp),
        ) {
            Column(Modifier.weight(1f)) {
                Line(title, geist(16.sp).copy(color = roles.text))
                if (subtitle != null) {
                    Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(5.dp)) {
                        if (logo != null) ServiceLogo(logo, 13.dp)
                        Line(subtitle, geist(13.5.sp).copy(color = roles.sub))
                    }
                }
            }
            if (star) Glyph(Icons.Starf, roles.tint, 14.dp)
            Glyph(Icons.Right, roles.muted, 18.dp)
        }
    }
}

/** A cover card: square artwork, then its title and a line under it. */
@Composable
fun CoverCard(
    artwork: Artwork?,
    title: String,
    subtitle: String?,
    explicit: Boolean = false,
    round: Boolean = false,
    modifier: Modifier = Modifier,
    onLongClick: (() -> Unit)? = null,
    onClick: () -> Unit,
) {
    Column(modifier.press(shrink = 0.97f, onLongClick = onLongClick, onClick = onClick)) {
        BoxWithConstraints(Modifier.fillMaxWidth()) {
            Cover(
                artwork,
                maxWidth,
                if (round) CircleShape else RoundedCornerShape(10.dp),
                Modifier.fillMaxWidth().aspectRatio(1f),
            )
        }
        Spacer(Modifier.height(7.dp))
        Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(5.dp)) {
            Line(title, geist(14.sp, FontWeight.Medium).copy(color = roles.text), Modifier.weight(1f, fill = false))
            if (explicit) ExplicitBadge()
        }
        if (subtitle != null) Line(subtitle, geist(13.sp).copy(color = roles.sub))
    }
}

/** The small E of explicit songs and albums. */
@Composable
fun ExplicitBadge() {
    Box(
        Modifier.size(15.dp).background(roles.muted.copy(alpha = 0.35f), RoundedCornerShape(3.dp)),
        contentAlignment = Alignment.Center,
    ) {
        BasicText("E", style = geist(9.5.sp, FontWeight.Bold).copy(color = roles.text))
    }
}

/** An album's line under its title: its artist and year. */
fun Album.byline() = listOfNotNull(artists.joinToString(", ").ifEmpty { null }, year?.toString()).joinToString(" · ")

/** A song's row in a list: artwork, title, artist. */
@Composable
fun TrackRow(track: Track, onLongClick: (() -> Unit)? = null, onClick: () -> Unit) {
    Row(
        Modifier.fillMaxWidth().height(60.dp).press(onLongClick = onLongClick, onClick = onClick).padding(start = 20.dp),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        Cover(track.artwork, 48.dp, RoundedCornerShape(7.dp), Modifier.size(48.dp))
        Row(
            Modifier.weight(1f).fillMaxHeight().underline(roles.line).padding(end = 12.dp),
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.spacedBy(6.dp),
        ) {
            Column(Modifier.weight(1f)) {
                Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(5.dp)) {
                    Line(track.title, geist(16.sp).copy(color = roles.text), Modifier.weight(1f, fill = false))
                    if (track.explicit) ExplicitBadge()
                }
                Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(5.dp)) {
                    if (track.id.provider != ProviderKind.APPLE_MUSIC) ServiceLogo(track.id.provider, 12.dp)
                    Line(track.artists.joinToString(", "), geist(13.5.sp).copy(color = roles.sub))
                }
            }
            Box(Modifier.size(44.dp), contentAlignment = Alignment.Center) { Glyph(Icons.More, roles.text, 20.dp) }
        }
    }
}
