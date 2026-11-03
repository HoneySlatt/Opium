package io.github.honeyslatt.music.ui.m3e

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.interaction.collectIsPressedAsState
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.statusBars
import androidx.compose.foundation.layout.windowInsetsTopHeight
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.BasicText
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.drawBehind
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.LineBreak
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import io.github.honeyslatt.music.core.model.Artist
import io.github.honeyslatt.music.core.model.Artwork
import io.github.honeyslatt.music.core.model.Track
import io.github.honeyslatt.music.ui.Icons
import io.github.honeyslatt.music.ui.MoreIcons
import io.github.honeyslatt.music.ui.chrome.ShapedPlay
import io.github.honeyslatt.music.ui.components.BarAction
import io.github.honeyslatt.music.ui.components.Cover
import io.github.honeyslatt.music.ui.components.Glyph
import io.github.honeyslatt.music.ui.components.Line
import io.github.honeyslatt.music.ui.components.anchor
import io.github.honeyslatt.music.ui.components.press
import io.github.honeyslatt.music.ui.components.rememberAnchor
import io.github.honeyslatt.music.ui.library.LibraryContext
import io.github.honeyslatt.music.ui.theme.geist
import io.github.honeyslatt.music.ui.theme.roles

/**
 * M3E's album and playlist header: the cover at 248 dp, radius 28, centred;
 * the eyebrow chip (what it is, its year, its quality); the title at 36 sp,
 * balanced; the byline, its artist a link.
 */
@Composable
fun ExpressiveHeader(
    artwork: Artwork?,
    title: String,
    byline: String?,
    onByline: (() -> Unit)?,
    meta: @Composable () -> Unit,
    art: (@Composable () -> Unit)?,
) {
    Column(Modifier.fillMaxWidth().padding(horizontal = 24.dp), horizontalAlignment = Alignment.CenterHorizontally) {
        Spacer(Modifier.windowInsetsTopHeight(WindowInsets.statusBars))
        Spacer(Modifier.height(64.dp))
        val shape = RoundedCornerShape(28.dp)
        Box(Modifier.size(248.dp).clip(shape)) {
            if (art != null) art() else Cover(artwork, 248.dp, shape, Modifier.size(248.dp))
        }
        Spacer(Modifier.height(20.dp))
        Eyebrow { meta() }
        BasicText(
            title,
            Modifier.padding(top = 12.dp),
            style = geist(36.sp, FontWeight.Bold, -0.035f).copy(
                color = roles.text,
                textAlign = TextAlign.Center,
                lineHeight = 40.sp,
                lineBreak = LineBreak.Heading,
            ),
        )
        if (byline != null) {
            Box(
                Modifier.padding(top = 6.dp)
                    .then(if (onByline != null) Modifier.press(RoundedCornerShape(8.dp), onClick = onByline) else Modifier),
            ) { Line(byline, geist(16.sp, FontWeight.Medium).copy(color = if (onByline != null) roles.primary else roles.sub)) }
        }
    }
}

/** M3E's eyebrow: what a page is, in a chip over its title; nothing when it has nothing to say. */
@Composable
fun Eyebrow(content: @Composable () -> Unit) {
    val fill = roles.cell.copy(alpha = 0.85f)
    androidx.compose.ui.layout.Layout(
        content = {
            Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(6.dp)) { content() }
        },
        modifier = Modifier.drawBehind {
            if (size.width > 0f) drawRoundRect(fill, cornerRadius = androidx.compose.ui.geometry.CornerRadius(8.dp.toPx()))
        },
    ) { measurables, constraints ->
        val row = measurables.first().measure(constraints.copy(minWidth = 0, minHeight = 0))
        if (row.width == 0) return@Layout layout(0, 0) {}
        val pad = 12.dp.roundToPx()
        val height = 32.dp.roundToPx()
        layout(row.width + 2 * pad, height) { row.place(pad, (height - row.height) / 2) }
    }
}

/** A 56 dp tonal round button: secondary-container, its glyph on it. */
@Composable
fun Tonal(icon: ImageVector, size: Dp = 56.dp, on: Boolean = false, onClick: () -> Unit) {
    Box(
        Modifier.size(size).press(CircleShape, shrink = 0.92f, onClick = onClick)
            .background(if (on) roles.primary else roles.secondaryContainer, CircleShape),
        contentAlignment = Alignment.Center,
    ) { Glyph(icon, if (on) roles.onPrimary else roles.onSecondaryContainer, size * 0.42f) }
}

/**
 * M3E's actions under a header: the 72 dp shaped Play (pausing while the
 * list plays), Shuffle as a tonal button, then the page's actions on the
 * right (the heart, the download button, More).
 */
@androidx.annotation.OptIn(androidx.media3.common.util.UnstableApi::class)
@Composable
fun ExpressivePlayRow(c: LibraryContext, tracks: List<Track>, actions: List<BarAction>, lead: @Composable () -> Unit = {}) {
    val playback = c.playback
    val now by playback.now.collectAsStateWithLifecycle()
    val playing by playback.playing.collectAsStateWithLifecycle()
    val here = now != null && tracks.any { it.id == now?.id }
    Row(
        Modifier.fillMaxWidth().padding(start = 20.dp, end = 12.dp, top = 20.dp, bottom = 12.dp),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        ShapedPlay(here && playing, false, 72.dp, {
            when {
                tracks.isEmpty() -> {}
                here -> playback.toggle()
                else -> playback.play(tracks, 0)
            }
        })
        Tonal(Icons.Shuffle) { if (tracks.isNotEmpty()) playback.shuffle(tracks) }
        lead()
        Spacer(Modifier.weight(1f))
        actions.forEach { a ->
            val anchor = rememberAnchor()
            Box(Modifier.size(48.dp).anchor(anchor).press(CircleShape) { a.onClick(anchor.rect) }, contentAlignment = Alignment.Center) {
                val glyph = a.glyph
                if (glyph != null) glyph(26.dp, roles.text) else Glyph(a.icon, if (a.icon == Icons.Starf) roles.primary else roles.text, 24.dp)
            }
        }
    }
}

/**
 * M3E's artist header, as the desktop's since 57e9d4f: an artist with a
 * video or a wide picture shows it across the page on a card at radius 28,
 * darkening at its foot under the chip and the name; one with only a photo
 * has it in the Sunny shape, turning as a cookie while pressed. The shaped
 * Play, Shuffle and the heart under it.
 */
@Composable
fun ExpressiveArtistHero(
    artist: Artist?,
    motion: String?,
    wide: Artwork?,
    eyebrow: String,
    favourite: Boolean?,
    onFavourite: () -> Unit,
    onShuffle: () -> Unit,
    onPlay: () -> Unit,
    /** The name as a link with a chevron: a library artist's to their profile. */
    onName: (() -> Unit)? = null,
) {
    Column(Modifier.fillMaxWidth()) {
        Spacer(Modifier.windowInsetsTopHeight(WindowInsets.statusBars))
        Spacer(Modifier.height(56.dp))
        val name = artist?.name.orEmpty()
        if (motion != null || wide != null) {
            BoxWithConstraints(Modifier.fillMaxWidth().padding(horizontal = 12.dp)) {
                val shape = RoundedCornerShape(28.dp)
                val width = maxWidth
                Box(Modifier.fillMaxWidth().height(width * 1.1f).clip(shape)) {
                    Cover(wide ?: artist?.artwork, width, androidx.compose.ui.graphics.RectangleShape, Modifier.fillMaxSize())
                    if (motion != null) io.github.honeyslatt.music.ui.detail.MotionArtwork(motion, Modifier.fillMaxSize(), crop = true)
                    Box(Modifier.fillMaxSize().background(Brush.verticalGradient(0.55f to Color.Transparent, 1f to Color.Black.copy(alpha = 0.7f))))
                    Column(Modifier.align(Alignment.BottomStart).padding(20.dp)) {
                        BasicText(eyebrow, style = geist(13.sp, FontWeight.Medium).copy(color = Color.White.copy(alpha = 0.85f)))
                        BasicText(name, Modifier.padding(top = 4.dp), style = geist(40.sp, FontWeight.Bold, -0.03f).copy(color = Color.White, lineHeight = 44.sp))
                    }
                }
            }
        } else {
            val pressed = androidx.compose.runtime.remember { androidx.compose.foundation.interaction.MutableInteractionSource() }
            val down by pressed.collectIsPressedAsState()
            val radii = rememberMorph(if (down) M3Shape.Cookie9 else M3Shape.Sunny)
            val turn = io.github.honeyslatt.music.ui.chrome.rememberTurn(down, 6_000)
            Column(Modifier.fillMaxWidth(), horizontalAlignment = Alignment.CenterHorizontally) {
                Box(
                    Modifier.size(232.dp)
                        .graphicsLayer { rotationZ = turn() }
                        .clip(RadialShape(radii))
                        .clickable(pressed, indication = null) {},
                ) { Cover(artist?.artwork, 232.dp, androidx.compose.ui.graphics.RectangleShape, Modifier.fillMaxSize()) }
                Spacer(Modifier.height(18.dp))
                Eyebrow { BasicText(eyebrow, style = geist(13.sp, FontWeight.Medium).copy(color = roles.sub)) }
                Row(
                    Modifier.padding(top = 10.dp, start = 24.dp, end = 24.dp)
                        .then(if (onName != null) Modifier.press(RoundedCornerShape(12.dp), onClick = onName) else Modifier),
                    verticalAlignment = Alignment.CenterVertically,
                ) {
                    BasicText(
                        name,
                        Modifier.weight(1f, fill = false),
                        style = geist(36.sp, FontWeight.Bold, -0.035f).copy(color = roles.text, textAlign = TextAlign.Center, lineHeight = 40.sp),
                    )
                    if (onName != null) Glyph(Icons.Right, roles.sub, 28.dp)
                }
            }
        }
        Row(
            Modifier.fillMaxWidth().padding(start = 20.dp, end = 12.dp, top = 20.dp, bottom = 8.dp),
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.spacedBy(12.dp),
        ) {
            ShapedPlay(false, false, 72.dp, onPlay)
            Tonal(Icons.Shuffle, onClick = onShuffle)
            Spacer(Modifier.weight(1f))
            if (favourite != null) {
                Box(Modifier.size(48.dp).press(CircleShape, onClick = onFavourite), contentAlignment = Alignment.Center) {
                    Glyph(if (favourite) MoreIcons.HeartFill else MoreIcons.Heart, if (favourite) roles.primary else roles.text, 26.dp)
                }
            }
        }
    }
}
