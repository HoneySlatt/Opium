package io.github.honeyslatt.music.ui.detail

import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.aspectRatio
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.statusBars
import androidx.compose.foundation.layout.windowInsetsPadding
import androidx.compose.foundation.layout.windowInsetsTopHeight
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.LazyListScope
import androidx.compose.foundation.lazy.LazyRow
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.lazy.rememberLazyListState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.BasicText
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.derivedStateOf
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.graphicsLayer
import io.github.honeyslatt.music.ui.components.ExplicitBadge
import androidx.compose.ui.draw.clip
import io.github.honeyslatt.music.ui.components.anchor
import io.github.honeyslatt.music.ui.components.rememberAnchor
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.draw.drawWithContent
import androidx.compose.ui.draw.shadow
import androidx.compose.ui.layout.layout
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import io.github.honeyslatt.music.core.model.AudioQuality
import io.github.honeyslatt.music.core.model.Artwork
import io.github.honeyslatt.music.core.model.Codec
import io.github.honeyslatt.music.ui.Icons
import io.github.honeyslatt.music.ui.chrome.ChromeHeight
import io.github.honeyslatt.music.ui.components.BarAction
import io.github.honeyslatt.music.ui.components.Cover
import io.github.honeyslatt.music.ui.components.CoverCard
import io.github.honeyslatt.music.ui.components.Glyph
import io.github.honeyslatt.music.ui.components.Line
import io.github.honeyslatt.music.ui.components.RoundButton
import io.github.honeyslatt.music.ui.components.SectionHeader
import io.github.honeyslatt.music.ui.components.press
import io.github.honeyslatt.music.ui.components.underline
import io.github.honeyslatt.music.ui.nav.Navigator
import io.github.honeyslatt.music.ui.theme.geist
import io.github.honeyslatt.music.ui.theme.roles

/**
 * An album, playlist or artist page: tinted by its cover from top to
 * bottom, its header running under the status bar, and a bar whose round
 * buttons sit on capsules until it turns solid in the page's colour.
 */
@Composable
fun DetailPage(
    nav: Navigator,
    title: String,
    page: Color,
    actions: List<BarAction> = emptyList(),
    content: LazyListScope.() -> Unit,
) {
    if (io.github.honeyslatt.music.ui.theme.look.layout.header == io.github.honeyslatt.music.core.model.Header.BANNER) {
        BannerPage(nav, title, page, actions, content)
        return
    }
    Tinted(page) {
        val state = rememberLazyListState()
        val solid by remember { derivedStateOf { state.firstVisibleItemIndex > 0 } }
        io.github.honeyslatt.music.ui.components.Folding(nav) { state.firstVisibleItemIndex to state.firstVisibleItemScrollOffset }
        Box(Modifier.fillMaxSize().background(roles.base)) {
            LazyColumn(Modifier.fillMaxSize(), state = state) {
                content()
                item { Spacer(Modifier.height(ChromeHeight + 58.dp + 34.dp)) }
            }
            Bar(nav, title, solid, actions)
        }
    }
}

/** A banner page's cover colour, which its header goes by. */
val LocalBanner = androidx.compose.runtime.compositionLocalOf<Color?> { null }

/**
 * How far a banner page's top is scrolled, in px (infinite once gone), for
 * its cover to shrink and fade, as Spotify's; null without Spotify's motion.
 */
val LocalBannerScroll = androidx.compose.runtime.compositionLocalOf<(() -> Float)?> { null }

/** A banner page's actions, which go left of Shuffle and Play rather than in the bar. */
val LocalBannerActions = androidx.compose.runtime.compositionLocalOf<List<BarAction>> { emptyList() }

/**
 * Spotify's page: the theme's colours, the cover's colour fading into
 * them behind the top, scrolling with it; the bar, only Back and the
 * title, taking that colour once scrolled.
 */
@Composable
private fun BannerPage(nav: Navigator, title: String, page: Color, actions: List<BarAction>, content: LazyListScope.() -> Unit) {
    val state = rememberLazyListState()
    val solid by remember { derivedStateOf { state.firstVisibleItemIndex > 0 } }
    val base = roles.base
    val spotify = io.github.honeyslatt.music.ui.theme.look.layout.motion == io.github.honeyslatt.music.core.model.Motion.SPOTIFY
    val scrolled = remember(state) { { if (state.firstVisibleItemIndex == 0) state.firstVisibleItemScrollOffset.toFloat() else Float.POSITIVE_INFINITY } }
    androidx.compose.runtime.CompositionLocalProvider(
        LocalBanner provides page,
        LocalBannerActions provides actions,
        LocalBannerScroll provides scrolled.takeIf { spotify },
    ) {
        Box(Modifier.fillMaxSize().background(base)) {
            val fade = with(androidx.compose.ui.platform.LocalDensity.current) { 4000.dp.toPx() }
            Box(
                Modifier
                    .fillMaxWidth()
                    .height(560.dp)
                    .graphicsLayer {
                        // Up with the page, gone once its top is.
                        translationY = if (state.firstVisibleItemIndex == 0) -state.firstVisibleItemScrollOffset.toFloat() else -fade
                    }
                    .background(androidx.compose.ui.graphics.Brush.verticalGradient(listOf(page, androidx.compose.ui.graphics.lerp(page, base, 0.55f), base))),
            )
            LazyColumn(Modifier.fillMaxSize(), state = state) {
                content()
                item { Spacer(Modifier.height(ChromeHeight + 58.dp + 34.dp)) }
            }
            Bar(nav, title, solid, emptyList(), solidFill = androidx.compose.ui.graphics.lerp(page, Color.Black, 0.45f))
        }
    }
}

@Composable
private fun Bar(nav: Navigator, title: String, solid: Boolean, actions: List<BarAction>, solidFill: Color = roles.base) {
    val fade by animateFloatAsState(if (solid) 1f else 0f, label = "bar")
    // Over the cover, the buttons sit on capsules; on the solid bar, on fills.
    val fill = if (solid) roles.surface else roles.capsule
    Box(
        Modifier
            .fillMaxWidth()
            .background(solidFill.copy(alpha = fade))
            .then(if (solid) Modifier.underline(roles.line) else Modifier)
            .windowInsetsPadding(WindowInsets.statusBars)
            .height(52.dp)
            .padding(horizontal = 12.dp),
    ) {
        if (nav.canPop) {
            Box(Modifier.align(Alignment.CenterStart).shadowIf(!solid)) { RoundButton(Icons.Left, { nav.pop() }, fill) }
        }
        Box(Modifier.align(Alignment.Center).padding(horizontal = 58.dp).alpha(fade)) {
            Line(title, geist(16.sp, FontWeight.SemiBold).copy(color = roles.text))
        }
        Row(Modifier.align(Alignment.CenterEnd), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            actions.forEach { a ->
                val anchor = rememberAnchor()
                Box(Modifier.shadowIf(!solid).anchor(anchor)) {
                    val glyph = a.glyph
                    if (glyph == null) {
                        RoundButton(a.icon, { a.onClick(anchor.rect) }, fill)
                    } else {
                        Box(
                            Modifier.size(40.dp).press(CircleShape, shrink = 0.92f) { a.onClick(anchor.rect) }.background(fill, CircleShape),
                            contentAlignment = Alignment.Center,
                        ) { glyph(22.dp, roles.text) }
                    }
                }
            }
        }
    }
}

private fun Modifier.shadowIf(on: Boolean) =
    if (on) shadow(6.dp, CircleShape, ambientColor = Color.Black.copy(alpha = 0.18f), spotColor = Color.Black.copy(alpha = 0.18f)) else this

/**
 * The header of an album or playlist: a still cover as a 264 dp centred
 * square, or (`motion`) the animated cover across the whole width, fading
 * into the page; then the title, the byline and the meta line.
 */
@Composable
fun DetailHeader(
    artwork: Artwork?,
    motion: String?,
    title: String,
    byline: String?,
    onByline: (() -> Unit)?,
    meta: @Composable () -> Unit,
    art: (@Composable () -> Unit)? = null,
) {
    if (LocalBanner.current != null) {
        BannerHeader(artwork, motion, title, byline, onByline, meta, art)
        return
    }
    Column(Modifier.fillMaxWidth(), horizontalAlignment = Alignment.CenterHorizontally) {
        if (motion != null) {
            FullBleed(artwork, motion)
        } else {
            Spacer(Modifier.windowInsetsTopHeight(WindowInsets.statusBars))
            Spacer(Modifier.height(64.dp))
            val shape = RoundedCornerShape(12.dp)
            Box(Modifier.size(264.dp).shadow(24.dp, shape, ambientColor = Color.Black.copy(alpha = 0.35f), spotColor = Color.Black.copy(alpha = 0.35f))) {
                if (art != null) art() else Cover(artwork, 264.dp, shape, Modifier.size(264.dp))
            }
            Spacer(Modifier.height(18.dp))
        }
        Column(
            Modifier.padding(horizontal = 20.dp).then(if (motion != null) Modifier.pullUp(64.dp) else Modifier),
            horizontalAlignment = Alignment.CenterHorizontally,
        ) {
            BasicText(
                title,
                style = geist(22.sp, FontWeight.Bold, -0.01f).copy(color = roles.text, textAlign = TextAlign.Center),
            )
            if (byline != null) {
                Box(
                    Modifier
                        .padding(top = 2.dp)
                        .then(if (onByline != null) Modifier.press(RoundedCornerShape(6.dp), onClick = onByline) else Modifier),
                ) {
                    Line(byline, geist(20.sp).copy(color = roles.text.copy(alpha = 0.78f)))
                }
            }
            Row(
                Modifier.padding(top = 6.dp),
                verticalAlignment = Alignment.CenterVertically,
                horizontalArrangement = Arrangement.spacedBy(6.dp),
            ) { meta() }
        }
    }
}

/** Spotify's header: the cover centred, moving when it has a video, then
 *  the title, the byline and the details on the left. */
@Composable
private fun BannerHeader(
    artwork: Artwork?,
    motion: String?,
    title: String,
    byline: String?,
    onByline: (() -> Unit)?,
    meta: @Composable () -> Unit,
    art: (@Composable () -> Unit)?,
) {
    Column(Modifier.fillMaxWidth()) {
        Spacer(Modifier.windowInsetsTopHeight(WindowInsets.statusBars))
        Spacer(Modifier.height(56.dp))
        val shape = RoundedCornerShape(4.dp)
        val scrolled = LocalBannerScroll.current
        Box(
            Modifier.align(Alignment.CenterHorizontally)
                .graphicsLayer {
                    // Scrolled, it stays behind a little, shrinks and fades.
                    val y = scrolled?.invoke() ?: 0f
                    val side = 260.dp.toPx()
                    val t = (y / side).coerceIn(0f, 1f)
                    translationY = y.coerceAtMost(side) * 0.55f
                    scaleX = 1f - 0.3f * t
                    scaleY = 1f - 0.3f * t
                    alpha = 1f - t
                }
                .size(260.dp)
                .shadow(28.dp, shape, ambientColor = Color.Black.copy(alpha = 0.5f), spotColor = Color.Black.copy(alpha = 0.5f)),
        ) {
            if (art != null) art() else Cover(artwork, 260.dp, shape, Modifier.size(260.dp))
            if (motion != null) MotionArtwork(motion, Modifier.size(260.dp).clip(shape), crop = true)
        }
        Spacer(Modifier.height(22.dp))
        Column(Modifier.padding(horizontal = 16.dp)) {
            BasicText(title, style = geist(26.sp, FontWeight.Bold, -0.01f).copy(color = roles.text))
            if (byline != null) {
                Box(
                    Modifier.padding(top = 8.dp)
                        .then(if (onByline != null) Modifier.press(RoundedCornerShape(6.dp), onClick = onByline) else Modifier),
                ) {
                    Line(byline, geist(14.sp, FontWeight.Bold).copy(color = roles.text))
                }
            }
            Row(
                Modifier.padding(top = 6.dp),
                verticalAlignment = Alignment.CenterVertically,
                horizontalArrangement = Arrangement.spacedBy(6.dp),
            ) { meta() }
        }
    }
}

/** Drawn and laid out `by` higher: over the bottom of what is above. */
private fun Modifier.pullUp(by: Dp) = layout { measurable, constraints ->
    val placeable = measurable.measure(constraints)
    val up = by.roundToPx()
    layout(placeable.width, (placeable.height - up).coerceAtLeast(0)) { placeable.place(0, -up) }
}

/** An animated cover, square across the width, melting into the page. */
@Composable
private fun FullBleed(artwork: Artwork?, motion: String) {
    val page = roles.base
    Box(
        Modifier
            .fillMaxWidth()
            .aspectRatio(1f)
            .drawWithContent {
                drawContent()
                drawRect(
                    Brush.verticalGradient(
                        0f to Color.Black.copy(alpha = 0.22f),
                        0.22f to Color.Transparent,
                        0.58f to Color.Transparent,
                        0.82f to page.copy(alpha = 0.7f),
                        1f to page,
                    ),
                )
            },
    ) {
        androidx.compose.foundation.layout.BoxWithConstraints(Modifier.fillMaxSize()) {
            Cover(artwork, maxWidth, androidx.compose.ui.graphics.RectangleShape, Modifier.fillMaxSize())
        }
        MotionArtwork(motion, Modifier.fillMaxSize())
    }
}

/** A small tag of the meta line: the quality, or Lossless. */
@Composable
fun Tag(text: String, lossless: Boolean = false) {
    val tinted = roles.tint == roles.text
    val (bg, fg) = when {
        lossless && !tinted -> roles.accent.copy(alpha = 0.18f) to roles.tint
        lossless || tinted -> roles.text.copy(alpha = 0.09f) to roles.text
        else -> roles.press to roles.sub
    }
    Box(Modifier.height(18.dp).background(bg, RoundedCornerShape(5.dp)).padding(horizontal = 6.dp), contentAlignment = Alignment.Center) {
        BasicText(text, style = geist(11.sp, FontWeight.SemiBold, 0.01f).copy(color = fg))
    }
}

/** The meta line's text part. */
@Composable
fun Meta(text: String) {
    Line(text, geist(13.sp).copy(color = roles.sub))
}

/** How a quality reads on a tag: `AAC 256`, `FLAC 24/96`. */
fun AudioQuality.label(): String {
    val name = when (val c = codec) {
        is Codec.Aac -> "AAC"
        is Codec.Alac -> "ALAC"
        is Codec.Flac -> "FLAC"
        is Codec.Mp3 -> "MP3"
        is Codec.Opus -> "Opus"
        is Codec.Vorbis -> "Vorbis"
        is Codec.Wav -> "WAV"
        is Codec.Other -> c.v1
    }
    return if (codec is Codec.Flac || codec is Codec.Alac || codec is Codec.Wav) {
        val bits = bitDepth?.toInt()
        val khz = sampleRate?.let { r -> (r.toDouble() / 1000).let { if (it % 1.0 == 0.0) "${it.toInt()}" else "$it" } }
        listOfNotNull(name, listOfNotNull(bits?.toString(), khz).joinToString("/").ifEmpty { null }).joinToString(" ")
    } else {
        listOfNotNull(name, bitrateKbps?.toString()).joinToString(" ")
    }
}

fun AudioQuality.lossless() = codec is Codec.Flac || codec is Codec.Alac || codec is Codec.Wav

/** Spatial Audio: the Dolby Atmos mix, rendered binaurally. */
fun AudioQuality.atmos() = codec == Codec.Other("Dolby Atmos")

/** The badge a quality earns, as Apple Music gives them: Lossless or Dolby Atmos, none for a lossy stream. */
fun AudioQuality.badge(): String? = when {
    lossless() -> "Lossless"
    atmos() -> "Dolby Atmos"
    else -> null
}

/** Whether the player's input, by its type, is lossless: an uploader's own file says so only once it plays. */
fun losslessMime(mime: String?) = mime in setOf(
    androidx.media3.common.MimeTypes.AUDIO_FLAC,
    androidx.media3.common.MimeTypes.AUDIO_ALAC,
    androidx.media3.common.MimeTypes.AUDIO_RAW,
    androidx.media3.common.MimeTypes.AUDIO_WAV,
)

/** The Dolby double D, `height` high: two half discs whose curves face each other. */
@Composable
fun DolbyLogo(color: androidx.compose.ui.graphics.Color, height: androidx.compose.ui.unit.Dp) {
    androidx.compose.foundation.Canvas(Modifier.size(height * 1.08f, height)) {
        val r = size.height / 2
        val disc = androidx.compose.ui.geometry.Size(2 * r, 2 * r)
        // The left D's flat side at the left edge, the right one's at the right.
        drawArc(color, -90f, 180f, true, topLeft = androidx.compose.ui.geometry.Offset(-r, 0f), size = disc)
        drawArc(color, 90f, 180f, true, topLeft = androidx.compose.ui.geometry.Offset(size.width - r, 0f), size = disc)
    }
}

/** A row of covers that scrolls sideways, under its title. */
fun LazyListScope.shelf(
    title: String,
    covers: List<ShelfCover>,
    onOpen: (() -> Unit)? = null,
) {
    if (covers.isEmpty()) return
    item { SectionHeader(title, onOpen = onOpen) }
    item {
        LazyRow(
            contentPadding = androidx.compose.foundation.layout.PaddingValues(horizontal = 20.dp),
            horizontalArrangement = Arrangement.spacedBy(12.dp),
        ) {
            items(covers) { c ->
                CoverCard(c.artwork, c.title, c.subtitle, c.explicit, c.round, Modifier.width(164.dp), onClick = c.onClick)
            }
        }
    }
}

data class ShelfCover(
    val artwork: Artwork?,
    val title: String,
    val subtitle: String?,
    val explicit: Boolean = false,
    val round: Boolean = false,
    val onClick: () -> Unit,
)

/** Music videos, 16:9: a tap opens the video's page on the service's website, where it plays. */
fun LazyListScope.videoShelf(title: String, videos: List<io.github.honeyslatt.music.core.model.MusicVideo>) {
    if (videos.isEmpty()) return
    item { SectionHeader(title) }
    item {
        val context = androidx.compose.ui.platform.LocalContext.current
        LazyRow(
            contentPadding = androidx.compose.foundation.layout.PaddingValues(horizontal = 20.dp),
            horizontalArrangement = Arrangement.spacedBy(12.dp),
        ) {
            items(videos) { v ->
                Column(
                    Modifier.width(240.dp).press(shrink = 0.97f) {
                        v.url?.let { url ->
                            runCatching {
                                context.startActivity(android.content.Intent(android.content.Intent.ACTION_VIEW, android.net.Uri.parse(url)))
                            }
                        }
                    },
                ) {
                    androidx.compose.foundation.layout.Box(
                        Modifier.fillMaxWidth().aspectRatio(16f / 9f).background(roles.surface, RoundedCornerShape(10.dp)).clip(RoundedCornerShape(10.dp)),
                    ) {
                        coil3.compose.AsyncImage(
                            v.thumbnail,
                            contentDescription = null,
                            contentScale = androidx.compose.ui.layout.ContentScale.Crop,
                            modifier = Modifier.fillMaxSize(),
                        )
                    }
                    androidx.compose.foundation.layout.Spacer(Modifier.height(7.dp))
                    Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(5.dp)) {
                        Line(v.title, geist(14.sp, FontWeight.Medium).copy(color = roles.text), Modifier.weight(1f, fill = false))
                        if (v.explicit) ExplicitBadge()
                    }
                    Line(v.artists.joinToString(", "), geist(13.sp).copy(color = roles.sub))
                }
            }
        }
    }
}
