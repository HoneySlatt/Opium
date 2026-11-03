package io.github.honeyslatt.music.ui.m3e

import androidx.compose.animation.core.Animatable
import androidx.compose.animation.core.AnimationSpec
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Outline
import androidx.compose.ui.graphics.Path
import androidx.compose.ui.unit.Density
import androidx.compose.ui.unit.LayoutDirection
import kotlin.math.PI
import kotlin.math.abs
import kotlin.math.cos
import kotlin.math.pow
import kotlin.math.sin

/** How many points each shape has, as on the desktop. */
const val POINTS = 120

/**
 * Material 3 Expressive's shapes, as the desktop's M3E draws them (its
 * `ui/shapes.rs`): each a polygon of [POINTS] points round its centre, one
 * radius per angle, so that any shape morphs into any other by easing its
 * radii into the other's.
 */
enum class M3Shape {
    Circle,
    /** The Play button while paused. */
    Squircle,
    /** Nine soft scallops: the Play button while playing, the brand. */
    Cookie9,
    Cookie6,
    /** Eight shallow scallops: artists' pictures. */
    Sunny,
    Clover4,
    Pentagon,
    Burst;

    /** The radius at angle `a`, in a unit box (0.5 touches its edge). */
    fun radius(a: Float): Float = when (this) {
        Circle -> 0.5f
        Squircle -> 0.5f / (abs(cos(a)).pow(4) + abs(sin(a)).pow(4)).pow(0.25f)
        Cookie9 -> cookie(9f, 0.13f, a)
        Cookie6 -> cookie(6f, 0.16f, a)
        Sunny -> cookie(8f, 0.09f, a)
        Clover4 -> 0.5f * (0.68f + 0.32f * abs(cos(2 * a)).pow(0.55f))
        Pentagon -> cookie(5f, 0.17f, a)
        Burst -> cookie(12f, 0.12f, a)
    }

    /** Its radii, the first at the top, going clockwise. */
    val radii: Radii by lazy { Radii(FloatArray(POINTS) { radius(angle(it)) }) }
}

/** A cookie of `k` scallops `depth` deep. */
private fun cookie(k: Float, depth: Float, a: Float) = 0.5f * (1 - depth + depth * (0.5f + 0.5f * cos(k * a)))

private fun angle(i: Int) = (i.toFloat() / POINTS * 2 * PI - PI / 2).toFloat()

/** A shape as its radii: what morphs. */
class Radii(val r: FloatArray) {
    /** `t` of the way from this to `to`; past 1 or under 0 as a spring overshoots. */
    fun toward(to: Radii, t: Float) = Radii(FloatArray(POINTS) { r[it] + (to.r[it] - r[it]) * t })

    /** Its outline in `size`, turned by `turn` radians. */
    fun path(size: Size, turn: Float = 0f): Path = Path().apply {
        val (cx, cy) = size.width / 2 to size.height / 2
        for (i in 0 until POINTS) {
            val a = angle(i) + turn
            val x = cx + r[i] * cos(a) * size.width
            val y = cy + r[i] * sin(a) * size.height
            if (i == 0) moveTo(x, y) else lineTo(x, y)
        }
        close()
    }
}

/** A shape to clip to or fill: `radii`, turned by `turn()` radians as it is drawn. */
class RadialShape(private val radii: Radii, private val turn: () -> Float = { 0f }) : androidx.compose.ui.graphics.Shape {
    override fun createOutline(size: Size, layoutDirection: LayoutDirection, density: Density) =
        Outline.Generic(radii.path(size, turn()))
}

/**
 * The radii of `target`, morphing there on `spec` from wherever it was
 * when it changed (a spring overshoots a little, as on the desktop).
 */
@Composable
fun rememberMorph(target: M3Shape, spec: AnimationSpec<Float> = Springs.fast()): Radii {
    var from by remember { mutableStateOf(target.radii) }
    var to by remember { mutableStateOf(target.radii) }
    val t = remember { Animatable(1f) }
    LaunchedEffect(target) {
        if (target.radii === to) return@LaunchedEffect
        from = from.toward(to, t.value)
        to = target.radii
        t.snapTo(0f)
        t.animateTo(1f, spec)
    }
    return from.toward(to, t.value)
}
