package io.github.honeyslatt.music.ui.components

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.runtime.Composable
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.runtime.getValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Shape
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import coil3.compose.AsyncImage
import io.github.honeyslatt.music.core.model.Artwork
import io.github.honeyslatt.music.ui.theme.roles

/** The artwork's address at the pixel size it is drawn at, a few sizes only. */
fun Artwork.at(pixels: Int): String {
    val size = listOf(120, 240, 400, 600, 1000, 1500).firstOrNull { it >= pixels } ?: 1500
    return urlTemplate.replace("{size}", size.toString())
}

/** A cover, `size` wide, on `surface` until it loads, with a hairline edge. */
@Composable
fun Cover(artwork: Artwork?, size: Dp, shape: Shape, modifier: Modifier = Modifier) {
    val pixels = with(LocalDensity.current) { size.roundToPx() }
    Box(
        modifier
            .clip(shape)
            .background(roles.surface)
            .border(1.dp, roles.line, shape),
    ) {
        if (artwork != null) {
            AsyncImage(
                model = artwork.at(pixels),
                contentDescription = null,
                contentScale = ContentScale.Crop,
                modifier = Modifier.matchParentSize(),
            )
        }
    }
}

fun corners(radius: Dp) = RoundedCornerShape(radius)

/**
 * A playlist's cover; one without, the covers of its first four albums in
 * a square (one cover alone when it has fewer).
 */
@androidx.annotation.OptIn(androidx.media3.common.util.UnstableApi::class)
@Composable
fun PlaylistCover(playlist: io.github.honeyslatt.music.core.model.Playlist, size: Dp, shape: Shape, modifier: Modifier = Modifier) {
    if (playlist.artwork != null) return Cover(playlist.artwork, size, shape, modifier)
    val actions = io.github.honeyslatt.music.ui.actions.LocalActions.current
    val covers by androidx.compose.runtime.produceState(emptyList<Artwork>(), playlist.id) {
        value = actions?.mosaic(playlist.id).orEmpty()
    }
    if (covers.size < 4) return Cover(covers.firstOrNull(), size, shape, modifier)
    Box(modifier.clip(shape).background(roles.surface).border(1.dp, roles.line, shape)) {
        androidx.compose.foundation.layout.Column(Modifier.matchParentSize()) {
            covers.chunked(2).forEach { row ->
                androidx.compose.foundation.layout.Row(Modifier.weight(1f)) {
                    row.forEach { art ->
                        AsyncImage(
                            model = art.at(with(LocalDensity.current) { (size / 2).roundToPx() }),
                            contentDescription = null,
                            contentScale = ContentScale.Crop,
                            modifier = Modifier.weight(1f).fillMaxHeight(),
                        )
                    }
                }
            }
        }
    }
}
