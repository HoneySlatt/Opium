package io.github.honeyslatt.music.ui

import androidx.compose.ui.graphics.vector.ImageVector

/** Icons beyond the design's set, for themes that draw as Spotify: Lucide's
 *  paths, its circles and rectangles written as paths. */
object MoreIcons {
    /** Previous and next: a triangle against a bar. */
    val SkipBack: ImageVector by lazy {
        filled("skip-back", false, "M19 4.8v14.4a.8.8 0 0 1-1.24.67L7.6 13.1a1.3 1.3 0 0 1 0-2.2l10.16-6.77A.8.8 0 0 1 19 4.8z", "M5.2 4h1.2a1.2 1.2 0 0 1 1.2 1.2v13.6a1.2 1.2 0 0 1-1.2 1.2H5.2A1.2 1.2 0 0 1 4 18.8V5.2A1.2 1.2 0 0 1 5.2 4z")
    }
    val SkipForward: ImageVector by lazy {
        filled("skip-forward", true, "M19 4.8v14.4a.8.8 0 0 1-1.24.67L7.6 13.1a1.3 1.3 0 0 1 0-2.2l10.16-6.77A.8.8 0 0 1 19 4.8z", "M5.2 4h1.2a1.2 1.2 0 0 1 1.2 1.2v13.6a1.2 1.2 0 0 1-1.2 1.2H5.2A1.2 1.2 0 0 1 4 18.8V5.2A1.2 1.2 0 0 1 5.2 4z")
    }
    val HeartFill: ImageVector by lazy {
        filled("heart-fill", false, "M19 14c1.49-1.46 3-3.21 3-5.5A5.5 5.5 0 0 0 16.5 3c-1.76 0-3 .5-4.5 2-1.5-1.5-2.74-2-4.5-2A5.5 5.5 0 0 0 2 8.5c0 2.3 1.5 4.05 3 5.5l7 7Z")
    }
    /** Spotify's Your Library: books on a shelf. */
    val Shelf: ImageVector by lazy { stroked("shelf", "m16 6 4 14", "M12 6v14", "M8 8v12", "M4 4v16") }
    /** Play on another device: a screen and a speaker. */
    val Devices: ImageVector by lazy {
        stroked(
            "devices",
            "M5.5 20H8",
            "M14 4h6a2 2 0 0 1 2 2v12a2 2 0 0 1-2 2h-6a2 2 0 0 1-2-2V6a2 2 0 0 1 2-2z",
            "M8 6H4a2 2 0 0 0-2 2v6a2 2 0 0 0 2 2h4",
            "M16 15a1 1 0 1 0 2 0a1 1 0 1 0-2 0",
            "M17 9h.01",
        )
    }
    val Share: ImageVector by lazy {
        stroked(
            "share",
            "M15 5a3 3 0 1 0 6 0a3 3 0 1 0-6 0",
            "M3 12a3 3 0 1 0 6 0a3 3 0 1 0-6 0",
            "M15 19a3 3 0 1 0 6 0a3 3 0 1 0-6 0",
            "m8.59 13.51 6.83 3.98",
            "m15.41 6.51-6.82 3.98",
        )
    }
    val Download: ImageVector by lazy { stroked("download", "M12 15V3", "M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4", "m7 10 5 5 5-5") }
    /** Three dots, upright. */
    val MoreVertical: ImageVector by lazy {
        stroked(
            "more-vertical",
            "M11 12a1 1 0 1 0 2 0a1 1 0 1 0-2 0",
            "M11 5a1 1 0 1 0 2 0a1 1 0 1 0-2 0",
            "M11 19a1 1 0 1 0 2 0a1 1 0 1 0-2 0",
        )
    }
    /** Pinned to the top of Your Library. */
    val Pin: ImageVector by lazy {
        filled("pin", false, "M11 17h2v4.2a1 1 0 0 1-2 0z", "M9 10.76a2 2 0 0 1-1.11 1.79l-1.78.9A2 2 0 0 0 5 15.24V16a1 1 0 0 0 1 1h12a1 1 0 0 0 1-1v-.76a2 2 0 0 0-1.11-1.79l-1.78-.9A2 2 0 0 1 15 10.76V7a1 1 0 0 1 1-1 2 2 0 0 0 0-4H8a2 2 0 0 0 0 4 1 1 0 0 1 1 1z")
    }
    val Expand: ImageVector by lazy { stroked("expand", "M15 3h6v6", "M9 21H3v-6", "m21 3-7 7", "m3 21 7-7") }
    /** A download's arrow; heavier for the small badge it sits in. */
    val ArrowDown: ImageVector by lazy { stroked("arrow-down", "M12 5v14", "m19 12-7 7-7-7") }
    val ArrowDownHeavy: ImageVector by lazy { stroked("arrow-down-heavy", 3.2f, "M12 5v14", "m19 12-7 7-7-7") }
    val CloudOff: ImageVector by lazy {
        stroked("cloud-off", "m2 2 20 20", "M5.782 5.782A7 7 0 0 0 9 19h8.5a4.5 4.5 0 0 0 1.307-.193", "M21.532 16.5A4.5 4.5 0 0 0 17.5 10h-1.79A7.008 7.008 0 0 0 10 5.07")
    }
    /** The heart's outline, M3E's favourite unset. */
    val Heart: ImageVector by lazy {
        stroked("heart", "M19 14c1.49-1.46 3-3.21 3-5.5A5.5 5.5 0 0 0 16.5 3c-1.76 0-3 .5-4.5 2-1.5-1.5-2.74-2-4.5-2A5.5 5.5 0 0 0 2 8.5c0 2.3 1.5 4.05 3 5.5l7 7Z")
    }
}
