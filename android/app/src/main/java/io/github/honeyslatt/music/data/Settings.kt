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
    private val leavePlaylistsState = flag("library-removals-leave-playlists", false)
    private val recentByDateState = flag("recent-by-date", false)
    private val sitesFirstState = flag("lyrics-sites-first", false)
    private val animatedState = flag("animated-artwork", true)
    private val originals = flag("soundcloud-originals", true)
    private val spatialState = flag("spatial-audio", true)
    private val browseState = mutableStateOf(
        prefs.getString("browse-source", null)?.let { runCatching { io.github.honeyslatt.music.core.model.ProviderKind.valueOf(it) }.getOrNull() },
    )
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

    /** A song taken out of the library leaves the user's playlists too: Music's and SoundCloud's, Apple's doing so itself. */
    var removalsLeavePlaylists: Boolean
        get() = leavePlaylistsState.value
        set(v) = save("library-removals-leave-playlists", v).also { leavePlaylistsState.value = v }

    /** Recently Added merges the services' by when each was added, rather than one after the other. */
    var recentByDate: Boolean
        get() = recentByDateState.value
        set(v) = save("recent-by-date", v).also { recentByDateState.value = v }

    /** Where lyrics come from when the service has none, in order. */
    var lyricsSites: List<LyricsSite>
        get() = sitesState.value
        set(v) {
            prefs.edit().putString("lyrics-sites", v.joinToString(",") { it.name }).apply()
            sitesState.value = v
            shareSettings()
        }

    private val sitesOnState = mutableStateOf(
        prefs.getString("lyrics-sites-on", null)
            ?.split(',')
            ?.mapNotNull { runCatching { LyricsSite.valueOf(it) }.getOrNull() }
            ?.toSet()
            ?: setOf(LyricsSite.LRCLIB),
    )

    /**
     * The sites asked: LRCLIB alone unless set, as the others are told,
     * from China, of every song looked up.
     */
    var lyricsSitesOn: Set<LyricsSite>
        get() = sitesOnState.value
        set(v) {
            prefs.edit().putString("lyrics-sites-on", v.joinToString(",") { it.name }).apply()
            sitesOnState.value = v
            shareSettings()
        }

    /** Ask the sites before the service. */
    var sitesFirst: Boolean
        get() = sitesFirstState.value
        set(v) = save("lyrics-sites-first", v).also { sitesFirstState.value = v }

    /** SoundCloud → Play the uploader's own file, where they allow downloads. */
    var soundcloudOriginals: Boolean
        get() = originals.value
        set(v) = save("soundcloud-originals", v).also { originals.value = v }

    /** Apple Music → Dolby Atmos: a song mixed in Atmos plays as Spatial Audio. */
    var spatialAudio: Boolean
        get() = spatialState.value
        set(v) = save("spatial-audio", v).also { spatialState.value = v }

    var animatedArtwork: Boolean
        get() = animatedState.value
        set(v) = save("animated-artwork", v).also { animatedState.value = v }

    /**
     * The service Home, New, Radio and the genres come from, when several are
     * connected; none chosen: Apple Music's, else the first connected.
     */
    var browseSource: io.github.honeyslatt.music.core.model.ProviderKind?
        get() = browseState.value
        set(v) {
            prefs.edit().putString("browse-source", v?.name).apply()
            browseState.value = v
        }

    var motionQuality: MotionQuality
        get() = qualityState.value
        set(v) {
            prefs.edit().putString("animated-quality", v.name).apply()
            qualityState.value = v
        }

    private fun save(key: String, value: Boolean) {
        prefs.edit().putBoolean(key, value).apply()
        shareSettings()
    }

    private var devices: Devices? = null

    /** The settings alike on every device, by the names the devices know them by. */
    private val sharedFlags = mapOf(
        "favourites-to-library" to favourites,
        "playlist-songs-to-library" to playlistSongs,
        "library-removals-leave-playlists" to leavePlaylistsState,
        "recent-by-date" to recentByDateState,
        "lyrics-sites-first" to sitesFirstState,
        "soundcloud-originals" to originals,
    )

    /**
     * The settings the devices share taken, and those set here before they
     * did offered them; as another device changes one, taken here too.
     */
    fun withDevices(devices: Devices) {
        this.devices = devices
        take(devices.settings())
        val known = devices.settings()
        shared().forEach { (name, value) -> if (name !in known) devices.offerSetting(name, value) }
        devices.settingsShared = { take(devices.settings()) }
    }

    /** A setting changed here: the devices are told. */
    fun shareSettings() {
        val devices = devices ?: return
        shared().forEach { (name, value) -> devices.setSetting(name, value) }
    }

    /** As set here; those never set aren't, the same by default everywhere. Spatial Audio and the rest stay the phone's. */
    private fun shared(): Map<String, String> = buildMap {
        sharedFlags.forEach { (name, state) -> if (prefs.contains(name)) put(name, state.value.toString()) }
        if (prefs.contains("lyrics-sites")) put("lyrics-sites", lyricsSites.joinToString(",") { it.shared })
        if (prefs.contains("lyrics-sites-on")) {
            put("lyrics-sites-on", LyricsSite.entries.filter { it in lyricsSitesOn }.joinToString(",") { it.shared })
        }
        prefs.getString("relay", null)?.let { put("relay", it) }
    }

    /** The settings as the devices have them, taken here. */
    private fun take(shared: Map<String, String>) {
        val edit = prefs.edit()
        sharedFlags.forEach { (name, state) ->
            val on = shared[name]?.let { it == "true" } ?: return@forEach
            if (state.value != on || !prefs.contains(name)) {
                state.value = on
                edit.putBoolean(name, on)
            }
        }
        val sites = { text: String -> text.split(',').mapNotNull { n -> LyricsSite.entries.firstOrNull { it.shared == n } } }
        shared["lyrics-sites"]?.let(sites)?.takeIf { it.toSet() == LyricsSite.entries.toSet() && it != sitesState.value }?.let {
            sitesState.value = it
            edit.putString("lyrics-sites", it.joinToString(",") { s -> s.name })
        }
        shared["lyrics-sites-on"]?.let(sites)?.toSet()?.takeIf { it != sitesOnState.value }?.let {
            sitesOnState.value = it
            edit.putString("lyrics-sites-on", it.joinToString(",") { s -> s.name })
        }
        // Taken at the next start.
        shared["relay"]?.let { edit.putString("relay", it) }
        edit.apply()
    }
}

/** A lyrics site as the devices name it to each other. */
private val LyricsSite.shared
    get() = when (this) {
        LyricsSite.LRCLIB -> "lrclib"
        LyricsSite.NET_EASE -> "netease"
        LyricsSite.KU_GOU -> "kugou"
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
