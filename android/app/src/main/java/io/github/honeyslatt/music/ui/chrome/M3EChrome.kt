package io.github.honeyslatt.music.ui.chrome

import androidx.compose.animation.core.LinearEasing
import androidx.compose.animation.core.animateDpAsState
import androidx.compose.animation.core.animateFloat
import androidx.compose.animation.core.infiniteRepeatable
import androidx.compose.animation.core.rememberInfiniteTransition
import androidx.compose.animation.core.tween
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.interaction.collectIsPressedAsState
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.navigationBars
import androidx.compose.foundation.layout.statusBars
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.layout.windowInsetsBottomHeight
import androidx.compose.foundation.layout.windowInsetsPadding
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.BasicText
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.shadow
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import io.github.honeyslatt.music.playback.Playback
import io.github.honeyslatt.music.ui.Icons
import io.github.honeyslatt.music.ui.MoreIcons
import io.github.honeyslatt.music.ui.actions.LocalActions
import io.github.honeyslatt.music.ui.components.Cover
import io.github.honeyslatt.music.ui.components.ExplicitBadge
import io.github.honeyslatt.music.ui.components.Glyph
import io.github.honeyslatt.music.ui.components.Line
import io.github.honeyslatt.music.ui.components.anchor
import io.github.honeyslatt.music.ui.components.press
import io.github.honeyslatt.music.ui.components.rememberAnchor
import io.github.honeyslatt.music.ui.m3e.M3Shape
import io.github.honeyslatt.music.ui.m3e.RadialShape
import io.github.honeyslatt.music.ui.m3e.WavyRing
import io.github.honeyslatt.music.ui.m3e.animationsOff
import io.github.honeyslatt.music.ui.m3e.rememberMorph
import io.github.honeyslatt.music.ui.nav.Navigator
import io.github.honeyslatt.music.ui.nav.Tab
import io.github.honeyslatt.music.ui.nowplaying.rememberPosition
import io.github.honeyslatt.music.ui.theme.geist
import io.github.honeyslatt.music.ui.theme.roles

/** M3E's four destinations; search is the bar over the root pages. */
val M3ETabs = listOf(Tab.Home, Tab.New, Tab.Radio, Tab.Library)

/**
 * A turn every `period` ms while `on`, in degrees, read as drawn; still
 * when off or when the system turned animations off.
 */
@Composable
fun rememberTurn(on: Boolean, period: Int): () -> Float {
    if (!on || animationsOff()) return { 0f }
    val turn = rememberInfiniteTransition(label = "turn")
    val angle = turn.animateFloat(0f, 360f, infiniteRepeatable(tween(period, easing = LinearEasing)), label = "angle")
    return { angle.value }
}

/**
 * M3E's Play: a squircle at rest, a cookie of nine scallops turning once
 * in 14 s while its music plays, morphing between them on the fast spring;
 * primary-container, its glyph on it.
 */
@Composable
fun ShapedPlay(playing: Boolean, loading: Boolean, size: Dp, onClick: () -> Unit, modifier: Modifier = Modifier) {
    val radii = rememberMorph(if (playing) M3Shape.Cookie9 else M3Shape.Squircle)
    val turn = rememberTurn(playing, 14_000)
    val fill = roles.accent
    Box(modifier.size(size).press(CircleShape, shrink = 0.92f, onClick = onClick), contentAlignment = Alignment.Center) {
        Box(Modifier.size(size).graphicsLayer { rotationZ = turn() }.clip(RadialShape(radii)).background(fill))
        if (loading) {
            Spinner(size * 0.4f)
        } else {
            io.github.honeyslatt.music.ui.components.PlayPause(playing, roles.onAccent, size * 0.46f)
        }
    }
}

/**
 * M3E's bottom: the floating toolbar 8 dp over the navigation bar, which
 * slides away as the page scrolls down (the toolbar then goes down to the
 * gesture bar) and back as it scrolls up.
 */
@androidx.annotation.OptIn(androidx.media3.common.util.UnstableApi::class)
@Composable
fun M3EBottom(nav: Navigator, mini: MiniState, playback: Playback, modifier: Modifier = Modifier) {
    val folded = nav.folded
    val inset = with(LocalDensity.current) { WindowInsets.navigationBars.getBottom(this).toDp() }
    val bar = 64.dp + inset
    val drop by animateDpAsState(if (folded) bar else 0.dp, tween(400, easing = Standard), label = "nav-drop")
    val lift by animateDpAsState(if (folded) inset + 12.dp else bar + 8.dp, tween(400, easing = Standard), label = "mini-lift")
    Box(modifier.fillMaxWidth().height(bar + 64.dp + 8.dp)) {
        M3EMiniPlayer(mini, playback, Modifier.align(Alignment.BottomCenter).padding(horizontal = 12.dp).offset(y = -lift))
        Column(Modifier.align(Alignment.BottomCenter).fillMaxWidth().offset(y = drop).background(roles.cell)) {
            Row(Modifier.fillMaxWidth().height(64.dp)) {
                M3ETabs.forEach { tab -> Destination(tab, nav.tab == tab, Modifier.weight(1f)) { nav.select(tab) } }
            }
            androidx.compose.foundation.layout.Spacer(Modifier.windowInsetsBottomHeight(WindowInsets.navigationBars))
        }
    }
}

/** A destination: its icon in a 56 × 32 pill while it is the one shown (64 wide pressed), its label under it. */
@Composable
private fun Destination(tab: Tab, on: Boolean, modifier: Modifier, onClick: () -> Unit) {
    val touches = remember { MutableInteractionSource() }
    val pressed by touches.collectIsPressedAsState()
    val width by animateDpAsState(if (pressed) 64.dp else 56.dp, io.github.honeyslatt.music.ui.m3e.Springs.fast(), label = "indicator")
    Column(
        modifier.fillMaxHeight().clickable(touches, indication = null, onClick = onClick).padding(top = 6.dp),
        horizontalAlignment = Alignment.CenterHorizontally,
        verticalArrangement = Arrangement.spacedBy(4.dp),
    ) {
        Box(
            Modifier.width(width).height(32.dp).background(if (on) roles.secondaryContainer else if (pressed) roles.press else Color.Transparent, CircleShape),
            contentAlignment = Alignment.Center,
        ) { Glyph(tab.icon, if (on) roles.onSecondaryContainer else roles.sub, 24.dp) }
        BasicText(tab.label, style = geist(12.sp, if (on) FontWeight.Bold else FontWeight.Medium).copy(color = if (on) roles.text else roles.sub))
    }
}

/**
 * M3E's floating toolbar: the cover turning once in 24 s while it plays,
 * in a wavy ring that is the position; the title with the E, the artist;
 * the shaped Play and Next. A tap opens Now Playing, a long press the
 * song's menu, a swipe sideways changes song.
 */
@androidx.annotation.OptIn(androidx.media3.common.util.UnstableApi::class)
@Composable
fun M3EMiniPlayer(state: MiniState, playback: Playback, modifier: Modifier = Modifier) {
    val track = state.track ?: return
    val actions = LocalActions.current
    val anchor = rememberAnchor()
    val swipe = io.github.honeyslatt.music.ui.nowplaying.rememberSongSwipe(track.id)
    val position by rememberPosition(playback, state.playing, everyMs = 500)
    val duration = playback.duration?.takeIf { it > 0 }
    val progress = duration?.let { (position.toFloat() / it).coerceIn(0f, 1f) }
    val turn = rememberTurn(state.playing, 24_000)
    Row(
        modifier
            .fillMaxWidth()
            .height(64.dp)
            .anchor(anchor)
            .shadow(16.dp, CircleShape, ambientColor = Color.Black.copy(alpha = 0.5f), spotColor = Color.Black.copy(alpha = 0.5f))
            .background(roles.surface, CircleShape)
            .press(CircleShape, onLongClick = { actions?.trackMenu(track, anchor.rect) }, onClick = state.onOpen)
            .padding(start = 5.dp, end = 8.dp),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(10.dp),
    ) {
        SwipedSong(swipe, state, Modifier.weight(1f)) {
            Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(10.dp)) {
                Box(contentAlignment = Alignment.Center) {
                    WavyRing(progress, state.playing)
                    Cover(track.artwork, 38.dp, CircleShape, Modifier.size(38.dp).graphicsLayer { rotationZ = turn() })
                }
                Column(Modifier.weight(1f)) {
                    Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(5.dp)) {
                        Line(track.title, geist(15.sp, FontWeight.SemiBold).copy(color = roles.text), Modifier.weight(1f, fill = false))
                        if (track.explicit) ExplicitBadge()
                    }
                    Line(track.artists.joinToString(", "), geist(13.sp).copy(color = roles.sub))
                }
            }
        }
        ShapedPlay(state.playing, state.loading, 48.dp, state.onToggle)
        Box(Modifier.size(44.dp).press(CircleShape, shrink = 0.9f, onClick = state.onNext), contentAlignment = Alignment.Center) {
            Glyph(MoreIcons.SkipForward, roles.text, 24.dp)
        }
    }
}

/**
 * M3E's search bar over the root pages: a 56 dp pill (28 dp corners while
 * pressed) that opens Search, the account's avatar at its end; the band
 * behind it fills once the page has scrolled.
 */
@Composable
fun M3ESearchBar(solid: Boolean, onSearch: () -> Unit, account: (@Composable () -> Unit)?, modifier: Modifier = Modifier) {
    val fill by androidx.compose.animation.animateColorAsState(if (solid) roles.cell else roles.cell.copy(alpha = 0f), label = "search-band")
    val touches = remember { MutableInteractionSource() }
    val pressed by touches.collectIsPressedAsState()
    val corner by animateDpAsState(if (pressed) 16.dp else 28.dp, io.github.honeyslatt.music.ui.m3e.Springs.fast(), label = "search-corner")
    Box(
        modifier
            .fillMaxWidth()
            .background(fill)
            .windowInsetsPadding(WindowInsets.statusBars)
            .padding(start = 16.dp, end = 16.dp, top = 4.dp, bottom = 4.dp),
    ) {
        val shape = RoundedCornerShape(corner)
        Row(
            Modifier.fillMaxWidth().height(56.dp).clip(shape).background(roles.surface, shape)
                .clickable(touches, indication = null, onClick = onSearch).padding(start = 16.dp, end = 8.dp),
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.spacedBy(12.dp),
        ) {
            Glyph(Icons.Search, roles.text, 24.dp)
            BasicText("Search Music", Modifier.weight(1f), style = geist(16.sp).copy(color = roles.sub))
            account?.invoke()
        }
    }
}
