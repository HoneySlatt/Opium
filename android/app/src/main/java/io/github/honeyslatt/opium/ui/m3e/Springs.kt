package io.github.honeyslatt.opium.ui.m3e

import androidx.compose.animation.core.SpringSpec
import androidx.compose.animation.core.spring

/**
 * Material 3 Expressive's springs, the desktop's: spatial ones (size,
 * position, shape, corners) overshoot a little, effects (colour, opacity)
 * don't.
 */
object Springs {
    /** Spatial, fast: stiffness 800, damping ratio 0.6. */
    fun <T> fast(): SpringSpec<T> = spring(dampingRatio = 0.6f, stiffness = 800f)

    /** Spatial, slow: 380, 0.8. */
    fun <T> slow(): SpringSpec<T> = spring(dampingRatio = 0.8f, stiffness = 380f)

    /** Effects: 1600, critically damped. */
    fun <T> effects(): SpringSpec<T> = spring(dampingRatio = 1f, stiffness = 1600f)
}
