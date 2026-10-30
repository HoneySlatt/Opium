package io.github.honeyslatt.music.data

import android.content.Context
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import io.github.honeyslatt.music.core.LyricsSite

/** How large animated covers play. */
enum class MotionQuality(val label: String) { Automatic("Automatic"), High("High"), Low("Low") }

/** Settings: kept on the phone, and read by the screens as they change. */
class Settings(context: Context) {
    private val prefs = context.getSharedPreferences("music", Context.MODE_PRIVATE)

    private fun flag(key: String, default: Boolean) = mutableStateOf(prefs.getBoolean(key, default))

    private val favourites = flag("favourites-to-library", true)
    private val playlistSongs = flag("playlist-songs-to-library", true)
    private val sitesFirstState = flag("lyrics-sites-first", false)
    private val animatedState = flag("animated-artwork", true)
    private val originals = flag("soundcloud-originals", true)
    private val qualityState = mutableStateOf(
        runCatching { MotionQuality.valueOf(prefs.getString("animated-quality", null) ?: "") }.getOrDefault(MotionQuality.Automatic),
    )
    private val sitesState = mutableStateOf(
        prefs.getString("lyrics-sites", null)
            ?.split(',')
            ?.mapNotNull { runCatching { LyricsSite.valueOf(it) }.getOrNull() }
            ?.takeIf { it.toSet() == LyricsSite.entries.toSet() }
            ?: listOf(LyricsSite.LRCLIB, LyricsSite.NET_EASE, LyricsSite.KU_GOU),
    )

    /** A song favourited joins the library if it isn't there. */
    var favouritesToLibrary: Boolean
        get() = favourites.value
        set(v) = save("favourites-to-library", v).also { favourites.value = v }

    /** A song added to a playlist joins the library if it isn't there. */
    var playlistSongsToLibrary: Boolean
        get() = playlistSongs.value
        set(v) = save("playlist-songs-to-library", v).also { playlistSongs.value = v }

    /** Where lyrics come from when the service has none, in order. */
    var lyricsSites: List<LyricsSite>
        get() = sitesState.value
        set(v) {
            prefs.edit().putString("lyrics-sites", v.joinToString(",") { it.name }).apply()
            sitesState.value = v
        }

    /** Ask the sites before the service. */
    var sitesFirst: Boolean
        get() = sitesFirstState.value
        set(v) = save("lyrics-sites-first", v).also { sitesFirstState.value = v }

    /** SoundCloud → Play the uploader's own file, where they allow downloads. */
    var soundcloudOriginals: Boolean
        get() = originals.value
        set(v) = save("soundcloud-originals", v).also { originals.value = v }

    var animatedArtwork: Boolean
        get() = animatedState.value
        set(v) = save("animated-artwork", v).also { animatedState.value = v }

    var motionQuality: MotionQuality
        get() = qualityState.value
        set(v) {
            prefs.edit().putString("animated-quality", v.name).apply()
            qualityState.value = v
        }

    private fun save(key: String, value: Boolean) {
        prefs.edit().putBoolean(key, value).apply()
    }
}

/** A site's name, as Settings lists it, and a word about it. */
val LyricsSite.label: String
    get() = when (this) {
        LyricsSite.LRCLIB -> "LRCLIB"
        LyricsSite.NET_EASE -> "NetEase Cloud Music"
        LyricsSite.KU_GOU -> "KuGou"
    }

val LyricsSite.about: String
    get() = when (this) {
        LyricsSite.LRCLIB -> "Open and free, made for synced lyrics."
        LyricsSite.NET_EASE -> "A large catalogue, from China."
        LyricsSite.KU_GOU -> "Another large catalogue, much from Asia."
    }
