package io.github.honeyslatt.opium.data

import android.content.Context
import android.util.Base64
import io.github.honeyslatt.opium.core.DownloadListener
import io.github.honeyslatt.opium.core.DownloadState
import io.github.honeyslatt.opium.core.KeptAudio
import io.github.honeyslatt.opium.core.KeptDownload
import io.github.honeyslatt.opium.core.OpiumException
import io.github.honeyslatt.opium.core.model.ItemId
import io.github.honeyslatt.opium.core.model.ProviderKind
import io.github.honeyslatt.opium.core.model.Track
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
    /** The store, and what is told it of the services' subscriptions, in order, even while it opens. */
    private val orders = StoreOrders<io.github.honeyslatt.opium.core.Downloads, ProviderKind> { store, provider ->
        store.forget(provider)
    }

    /** The store, once open; none while it opens, or if it couldn't. */
    private val store get() = orders.open

    /** Opium Connect, through which the computer lends what can't be downloaded here. */
    private var lender: io.github.honeyslatt.opium.core.Connect? = null

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
            // A song coming shows 5 % at a time, not each quarter second:
            // every row on screen reads these.
            val before = _states.value[id]
            if (state is DownloadState.Coming && before is DownloadState.Coming && state.part < 1f && state.part - before.part < 0.05f) return
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
            try {
                val store = openStore()
                // A sign-out said while the store couldn't open is told it now: every service whose
                // songs come through DRM and isn't signed in is. Off the main thread, as each one
                // told is a write of the store's manifest.
                withContext(Dispatchers.IO) {
                    orders.opened(store, notSignedIn = WITH_SUBSCRIPTION.filterNot(core::signedIn))
                }
                lender?.let { store?.borrowThrough(it) }
            } finally {
                opened.complete(Unit)
            }
        }
    }

    private val dir = File(context.filesDir, "downloads")

    /** The store, opened with its key; none, and why said, when it can't be this run. Never throws. */
    private suspend fun openStore(): io.github.honeyslatt.opium.core.Downloads? {
        val key = key() ?: return null
        return try {
            io.github.honeyslatt.opium.core.Downloads.open(core.session, dir.path, key, listener)
        } catch (e: OpiumException) {
            _failed.value = e.message
            null
        } catch (e: kotlinx.coroutines.CancellationException) {
            throw e
        } catch (e: Exception) {
            // A fault in the core: said, never a crash.
            _failed.value = "The downloads couldn't open (${e.javaClass.simpleName})"
            null
        }
    }

    /**
     * The store's key, made once and kept in the vault. Gone for good (the
     * Keystore's key lost, as after a restore): what it sealed can't be read
     * again, so it goes, and a new key starts afresh. Not readable now, or a
     * new key not kept: none, and why said; nothing is deleted, and the next
     * start tries again.
     */
    private suspend fun key(): ByteArray? = withContext(Dispatchers.IO) {
        when (val step = downloadsKeyStep(core.vault.read(KEY))) {
            is DownloadsKeyStep.Use -> try {
                Base64.decode(step.kept, Base64.NO_WRAP)
            } catch (e: IllegalArgumentException) {
                _failed.value = "The downloads' key is damaged"
                null
            }
            is DownloadsKeyStep.MakeNew -> {
                if (step.sweepFirst) dir.deleteRecursively()
                val key = ByteArray(32).also { SecureRandom().nextBytes(it) }
                // A key not kept would seal songs that could never be read again.
                core.vault.write(KEY, Base64.encodeToString(key, Base64.NO_WRAP)).fold(
                    { key },
                    {
                        _failed.value = "The downloads couldn't keep their key; tried again at the next start"
                        null
                    },
                )
            }
            is DownloadsKeyStep.Unavailable -> {
                _failed.value = "The downloads couldn't read their key (${step.why}); tried again at the next start"
                null
            }
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
        val settings = (context.applicationContext as io.github.honeyslatt.opium.OpiumApp).settings
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
        scope.launch(Dispatchers.IO) { io.github.honeyslatt.opium.core.keepTracks(playlistFile(id), tracks) }
    }

    /** A downloaded playlist's songs as kept; none if it wasn't. */
    suspend fun keptPlaylist(id: ItemId): List<Track> =
        withContext(Dispatchers.IO) { io.github.honeyslatt.opium.core.keptTracks(playlistFile(id)) }

    private fun playlistFile(id: ItemId) =
        File(core.kept, "downloaded-playlist-${id.provider.name}-${id.id.hashCode().toUInt()}.json").path

    /**
     * Signed out of `provider`: its downloads waiting or under way are called off, and its protected
     * songs no longer play until its subscription is confirmed again; every song kept stays.
     */
    fun forget(provider: ProviderKind) {
        // Said while the store opens, it is told once it opens, after what was said before.
        orders.signedOut(provider)
    }

    /** Apple Music's and SoundCloud's encrypted songs come from the computer, through `connect`. */
    fun borrowThrough(connect: io.github.honeyslatt.opium.core.Connect) {
        lender = connect
        store?.borrowThrough(connect)
    }

    /**
     * `kind` connected: its subscription confirmed (Apple's asked), so that
     * the songs downloaded through it go on playing for 30 days more, or
     * stop if it ended. Told the store in the order said, even while it
     * opens; an answer that comes after a sign-out of `kind` is dropped.
     */
    fun confirm(kind: ProviderKind) {
        when (kind) {
            ProviderKind.APPLE_MUSIC -> {
                val asked = orders.confirming(kind)
                scope.launch {
                    val active = runCatching { core.session.appleSubscribed() }.getOrNull() ?: return@launch
                    asked.answered { it.subscription(kind, active) }
                }
            }
            // SoundCloud's counts as confirmed while signed in.
            ProviderKind.SOUND_CLOUD -> orders.confirming(kind).answered { it.subscription(kind, true) }
            else -> {}
        }
    }

    /** Whether songs are waiting or coming. */
    fun busy() = store?.busy() ?: false

    private companion object {
        const val KEY = "downloads"

        /** The services whose songs come through DRM: the store keeps their subscriptions ([confirm]). */
        val WITH_SUBSCRIPTION = listOf(ProviderKind.APPLE_MUSIC, ProviderKind.SOUND_CLOUD)
    }
}

/** Whether `track` can be downloaded: every service's songs, Opium's playlists having none of their own, a file being here already. */
fun downloadable(track: Track) = track.id.provider != ProviderKind.OPIUM && track.id.provider != ProviderKind.LOCAL
