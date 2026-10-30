package io.github.honeyslatt.music.ui.detail

import androidx.annotation.OptIn
import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.animation.core.tween
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.foundation.layout.Box
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clipToBounds
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.platform.LocalContext
import androidx.media3.common.MediaItem
import androidx.media3.common.Player
import androidx.media3.common.util.UnstableApi
import androidx.media3.exoplayer.ExoPlayer
import androidx.media3.ui.compose.PlayerSurface
import androidx.media3.ui.compose.SURFACE_TYPE_TEXTURE_VIEW
import androidx.media3.ui.compose.modifiers.resizeWithContentScale
import androidx.media3.ui.compose.state.rememberPresentationState

/**
 * An animated cover: Apple's video, looping without sound, faded in over
 * the still cover once its first frame is drawn. It never asks for audio
 * focus, so the music plays on.
 */
@OptIn(UnstableApi::class)
@Composable
fun MotionArtwork(url: String, modifier: Modifier = Modifier, crop: Boolean = false) {
    val context = LocalContext.current
    var shown by remember(url) { mutableStateOf(false) }
    val player = remember(url) {
        ExoPlayer.Builder(context).build().apply {
            volume = 0f
            repeatMode = Player.REPEAT_MODE_ONE
            setMediaItem(MediaItem.fromUri(url))
            addListener(object : Player.Listener {
                override fun onRenderedFirstFrame() {
                    shown = true
                }
            })
            prepare()
            playWhenReady = true
        }
    }
    DisposableEffect(player) { onDispose { player.release() } }
    val fade by animateFloatAsState(if (shown) 1f else 0f, tween(400), label = "motion")
    if (crop) {
        // Filling the space whatever its shape, as the still cover does.
        val presentation = rememberPresentationState(player)
        Box(modifier.clipToBounds().alpha(fade), contentAlignment = Alignment.Center) {
            PlayerSurface(
                player,
                Modifier.resizeWithContentScale(ContentScale.Crop, presentation.videoSizeDp),
                surfaceType = SURFACE_TYPE_TEXTURE_VIEW,
            )
        }
    } else {
        PlayerSurface(player, modifier.alpha(fade), surfaceType = SURFACE_TYPE_TEXTURE_VIEW)
    }
}
