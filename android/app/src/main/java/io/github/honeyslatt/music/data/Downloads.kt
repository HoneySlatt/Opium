package io.github.honeyslatt.music.data

import android.content.Context
import android.util.Base64
import io.github.honeyslatt.music.core.DownloadListener
import io.github.honeyslatt.music.core.DownloadState
import io.github.honeyslatt.music.core.KeptAudio
import io.github.honeyslatt.music.core.KeptDownload
import io.github.honeyslatt.music.core.MusicException
import io.github.honeyslatt.music.core.model.ItemId
import io.github.honeyslatt.music.core.model.ProviderKind
import io.github.honeyslatt.music.core.model.Track
import java.io.File
import java.security.SecureRandom
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext

/**
 * The songs downloaded to play offline: the core's store in the app's
 * files, sealed with a key the vault keeps. What each song stands at shows
 * as its badge; while songs come, [DownloadService] keeps the app up.
 */
class Downloads(private val context: Context, private val core: Core, private val scope: CoroutineScope) {
    private var store: io.github.honeyslatt.music.core.Downloads? = null

    /** Once the store is open (or couldn't be): a song played at launch waits for it. */
    private val opened = kotlinx.coroutines.CompletableDeferred<Unit>()

    private val _states = MutableStateFlow<Map<ItemId, DownloadState>>(emptyMap())

    /** By song, how it stands: queued, coming, kept… (none: not downloaded). */
    val states: StateFlow<Map<ItemId, DownloadState>> = _states.asStateFlow()

    private val _kept = MutableStateFlow<List<KeptDownload>>(emptyList())

    /** Every song kept, the last downloaded first: the library's Downloaded. */
    val kept: StateFlow<List<KeptDownload>> = _kept.asStateFlow()

    private val _failed = MutableStateFlow<String?>(null)

    /** The last download that failed, said once. */
    val failed: StateFlow<String?> = _failed.asStateFlow()

    fun failedSeen() {
        _failed.value = null
    }

    private val listener = object : DownloadListener {
        override fun state(id: ItemId, state: DownloadState) {
            _states.update { if (state is DownloadState.Gone) it - id else it + (id to state) }
            if (state is DownloadState.Gone) _kept.update { kept -> kept.filter { it.track.id != id } }
            if (state is DownloadState.Queued || state is DownloadState.Coming) DownloadService.keepUp(context)
            if (state is DownloadState.Kept) synchronized(coming) { coming.remove(id) }?.let(::keepCover)
            if (state is DownloadState.Gone) synchronized(coming) { coming.remove(id) }
        }

        override fun kept(kept: List<KeptDownload>) {
            _kept.value = kept.sortedByDescending { it.added }
            _states.update { states ->
                states.filterValues { it is DownloadState.Queued || it is DownloadState.Coming || it is DownloadState.Failed } +
                    kept.associate { it.track.id to it.state }
            }
        }

        override fun failed(message: String) {
            _failed.value = message
        }
    }

    init {
        scope.launch {
            store = try {
                io.github.honeyslatt.music.core.Downloads.open(core.session, dir.path, key(), listener)
            } catch (e: MusicException) {
                _failed.value = e.message
                null
            }
            opened.complete(Unit)
        }
    }

    private val dir = File(context.filesDir, "downloads")

    /**
     * The store's key, made once. Kept but unreadable (the Keystore's key
     * gone, as after a restore): what it sealed can't be read again, so it
     * goes, and a new key starts afresh.
     */
    private suspend fun key(): ByteArray = withContext(Dispatchers.IO) {
        core.vault.read(KEY)?.let { return@withContext Base64.decode(it, Base64.NO_WRAP) }
        if (core.vault.has(KEY)) dir.deleteRecursively()
        ByteArray(32).also {
            SecureRandom().nextBytes(it)
            core.vault.write(KEY, Base64.encodeToString(it, Base64.NO_WRAP))
        }
    }

    /** The song kept as `track` (or its library's copy), to play, if it is kept and plays. */
    suspend fun song(track: Track): KeptAudio? {
        opened.await()
        val store = store ?: return null
        return store.song(track.id) ?: track.libraryId?.takeIf { it != track.id }?.let { store.song(it) }
    }

    /** The songs asked for, until kept: their covers are fetched then. */
    private val coming = mutableMapOf<ItemId, Track>()

    /**
     * A song kept: its cover fetched at the sizes the rows, the pages and
     * Now Playing draw it, into the covers' cache, to show offline.
     */
    private fun keepCover(track: Track) {
        val artwork = track.artwork ?: return
        val loader = coil3.SingletonImageLoader.get(context)
        listOf(120, 240, 600, 1000, 1500).forEach { size ->
            loader.enqueue(
                coil3.request.ImageRequest.Builder(context)
                    .data(artwork.urlTemplate.replace("{size}", size.toString()))
                    .memoryCachePolicy(coil3.request.CachePolicy.DISABLED)
                    .build(),
            )
        }
    }

    /** Download `tracks`, after those waiting, as the settings say (SoundCloud's originals). */
    fun add(tracks: List<Track>) {
        val settings = (context.applicationContext as io.github.honeyslatt.music.MusicApp).settings
        val own = tracks.filter(::downloadable)
        synchronized(coming) { own.forEach { coming[it.id] = it } }
        store?.add(own, settings.soundcloudOriginals)
    }

    fun remove(ids: List<ItemId>) {
        store?.remove(ids)
    }

    fun removeAll() {
        store?.removeAll()
    }

    /** A playlist downloaded: its songs kept beside the library's lists, for its page offline. */
    fun keepPlaylist(id: ItemId, tracks: List<Track>) {
        scope.launch(Dispatchers.IO) { io.github.honeyslatt.music.core.keepTracks(playlistFile(id), tracks) }
    }

    /** A downloaded playlist's songs as kept; none if it wasn't. */
    suspend fun keptPlaylist(id: ItemId): List<Track> =
        withContext(Dispatchers.IO) { io.github.honeyslatt.music.core.keptTracks(playlistFile(id)) }

    private fun playlistFile(id: ItemId) =
        File(core.kept, "downloaded-playlist-${id.provider.name}-${id.id.hashCode().toUInt()}.json").path

    /** Signed out of `provider`: its songs go. */
    fun forget(provider: ProviderKind) {
        store?.forget(provider)
    }

    /** Whether songs are waiting or coming. */
    fun busy() = store?.busy() ?: false

    private companion object {
        const val KEY = "downloads"
    }
}

/** Whether `track` can be downloaded on the phone: Apple Music's and SoundCloud's encrypted songs not yet. */
fun downloadable(track: Track) =
    track.id.provider == ProviderKind.NAVIDROME || track.id.provider == ProviderKind.SOUND_CLOUD
