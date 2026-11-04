package io.github.honeyslatt.opium.ui.theme

import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.font.Font
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.TextUnit
import androidx.compose.ui.unit.em
import io.github.honeyslatt.opium.R

/** Geist (SIL Open Font License), in the four weights the design uses. */
val Geist = FontFamily(
    Font(R.font.geist_regular, FontWeight.Normal),
    Font(R.font.geist_medium, FontWeight.Medium),
    Font(R.font.geist_semibold, FontWeight.SemiBold),
    Font(R.font.geist_bold, FontWeight.Bold),
)

fun geist(size: TextUnit, weight: FontWeight = FontWeight.Normal, tracking: Float = 0f) = TextStyle(
    fontFamily = Geist,
    fontSize = size,
    fontWeight = weight,
    letterSpacing = tracking.em,
)

/** Times, with figures of one width. */
fun TextStyle.tabular() = copy(fontFeatureSettings = "tnum")
