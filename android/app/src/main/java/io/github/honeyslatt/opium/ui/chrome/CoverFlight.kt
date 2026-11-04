package io.github.honeyslatt.opium.ui.chrome

import androidx.compose.animation.AnimatedVisibilityScope
import androidx.compose.animation.BoundsTransform
import androidx.compose.animation.SharedTransitionScope
import androidx.compose.animation.core.spring
import androidx.compose.runtime.Composable
import androidx.compose.runtime.compositionLocalOf
import androidx.compose.ui.Modifier

/**
 * Apple Music's opening of Now Playing: the mini player's cover grows
 * into the large one, and shrinks back into it on closing. Null in
 * Spotify's, whose Now Playing rises whole.
 */
val LocalCoverFlight = compositionLocalOf<SharedTransitionScope?> { null }

/** Now Playing's showing, for its cover to fly from the mini player's. */
val LocalNpVisibility = compositionLocalOf<AnimatedVisibilityScope?> { null }

/** The cover that flies between the mini player and Now Playing, shown in `visibility`. */
@Composable
fun Modifier.coverFlight(visibility: AnimatedVisibilityScope?): Modifier {
    val scope = LocalCoverFlight.current ?: return this
    if (visibility == null) return this
    return with(scope) {
        this@coverFlight.sharedElement(
            rememberSharedContentState("now-playing-cover"),
            visibility,
            boundsTransform = BoundsTransform { _, _ -> spring(dampingRatio = 0.86f, stiffness = 260f) },
        )
    }
}
