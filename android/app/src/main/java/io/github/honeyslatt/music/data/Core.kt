package io.github.honeyslatt.music.data

import android.content.Context
import io.github.honeyslatt.music.core.MusicException
import io.github.honeyslatt.music.core.Session
import io.github.honeyslatt.music.core.TokenKeeper
import io.github.honeyslatt.music.core.model.ProviderKind
import io.github.honeyslatt.music.ui.theme.Appearance
import java.io.File
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch
import org.json.JSONObject

/** How a service stands. */
sealed interface Connection {
    data object Off : Connection
    data object Connecting : Connection

    /** `name`: Apple's storefront, the SoundCloud or Navidrome user. */
    data class Connected(val name: String) : Connection
    data class Failed(val reason: String) : Connection
}

/**
 * The Rust core's session, and what the app keeps between launches: the
 * sign-ins (in the vault) and the settings.
 */
class Core(context: Context) {
    val session = Session()
    private val prefs = context.getSharedPreferences("music", Context.MODE_PRIVATE)
    private val vault = Vault(context)

    /** Sends SoundCloud's changes, as only a browser's are taken. */
    val soundCloudWeb = SoundCloudWeb(context.applicationContext)

    private val connections = ProviderKind.entries.associateWith { MutableStateFlow<Connection>(Connection.Off) }

    private val shown = connections.mapValues { it.value.asStateFlow() }

    /** The same flow each time: collected anew at each recomposition otherwise. */
    fun connection(kind: ProviderKind): StateFlow<Connection> = shown.getValue(kind)

    val apple: StateFlow<Connection> = connection(ProviderKind.APPLE_MUSIC)

    private val _appleProfile = MutableStateFlow<io.github.honeyslatt.music.core.AppleProfile?>(null)

    /** The account's profile on Apple Music, once connected and read. */
    val appleProfile: StateFlow<io.github.honeyslatt.music.core.AppleProfile?> = _appleProfile.asStateFlow()

    init {
        // The account token from before the vault: moved in.
        val old = File(context.filesDir, "apple-token")
        if (old.exists()) {
            vault.write(APPLE, old.readText())
            old.delete()
        }
    }

    private val _appearance = MutableStateFlow(
        runCatching { Appearance.valueOf(prefs.getString("appearance", null) ?: "") }
            .getOrDefault(Appearance.Automatic),
    )
    val appearance: StateFlow<Appearance> = _appearance.asStateFlow()

    fun setAppearance(appearance: Appearance) {
        _appearance.value = appearance
        prefs.edit().putString("appearance", appearance.name).apply()
    }

    /** A choice the app remembers: a sort order, a switch. */
    fun setting(key: String): String? = prefs.getString(key, null)

    fun setSetting(key: String, value: String) {
        prefs.edit().putString(key, value).apply()
    }

    /** Whether a sign-in to `kind` is kept. */
    fun signedIn(kind: ProviderKind) = vault.has(kind.vaultName)

    /** Settings → a service → Use: off keeps the sign-in. */
    fun enabled(kind: ProviderKind) = prefs.getBoolean("enabled-${kind.name}", true)

    /** The first launch's screen, until one service is connected and Continue is tapped. */
    var welcomed: Boolean
        get() = prefs.getBoolean("welcomed", false) || ProviderKind.entries.any { signedIn(it) }
        set(v) = prefs.edit().putBoolean("welcomed", v).apply()

    /**
     * Sign in again with what was kept, every service at once; those that
     * are already, or on their way, are left be (the screen made anew calls
     * this again).
     */
    suspend fun start() {
        kotlinx.coroutines.coroutineScope {
            ProviderKind.entries.filter { signedIn(it) && enabled(it) }.forEach { kind ->
                val now = connections.getValue(kind).value
                if (now !is Connection.Connected && now != Connection.Connecting) launch { connect(kind) }
            }
        }
    }

    /** When `kind`'s library was last read whole, 0 if not lately. */
    fun libraryReadAt(kind: ProviderKind) = prefs.getLong("library-read-${kind.name}", 0)

    fun libraryRead(kind: ProviderKind, at: Long) = prefs.edit().putLong("library-read-${kind.name}", at).apply()

    /** By service, failed tries in a row and when the last was. */
    private val tries = mutableMapOf<ProviderKind, Pair<Int, Long>>()

    /**
     * The network is back: the services it kept out are tried again, each
     * less often as it keeps failing (a server at home, away from it), from
     * a minute up to half an hour.
     */
    suspend fun retryFailed() {
        val now = android.os.SystemClock.elapsedRealtime()
        kotlinx.coroutines.coroutineScope {
            ProviderKind.entries.filter { signedIn(it) && enabled(it) && connections.getValue(it).value is Connection.Failed }.forEach { kind ->
                val (failed, at) = synchronized(tries) { tries[kind] } ?: (0 to 0L)
                val wait = (60_000L shl (failed - 1).coerceIn(0, 5)).coerceAtMost(30 * 60_000L)
                if (failed > 0 && now - at < wait) return@forEach
                launch {
                    val got = connect(kind)
                    synchronized(tries) {
                        if (got is Connection.Connected) tries.remove(kind) else tries[kind] = (failed + 1) to now
                    }
                }
            }
        }
    }

    suspend fun setEnabled(kind: ProviderKind, on: Boolean) {
        prefs.edit().putBoolean("enabled-${kind.name}", on).apply()
        if (on) {
            if (signedIn(kind)) connect(kind)
        } else {
            session.disconnect(kind)
            connections.getValue(kind).value = Connection.Off
        }
    }

    /** What was opened from searches, for Recently Searched. */
    val searched: String = File(context.filesDir, "recently-searched.json").path

    /** The library's lists as last read, by service: see [Library]. */
    val kept = File(context.cacheDir, "library").apply { mkdirs() }

    /** Forget the account: its sign-in goes from the phone, and its library as kept. */
    fun signOut(kind: ProviderKind) {
        session.disconnect(kind)
        vault.delete(kind.vaultName)
        kept.listFiles { f -> f.name.endsWith("-${kind.name}.json") }?.forEach { it.delete() }
        connections.getValue(kind).value = Connection.Off
    }

    /** The token Apple's sign-in page left, kept and used. */
    suspend fun signedInToApple(token: String): Connection {
        vault.write(APPLE, token)
        return connect(ProviderKind.APPLE_MUSIC)
    }

    /** The tokens SoundCloud's web player left, kept and used. */
    suspend fun signedInToSoundCloud(oauth: String, refresh: String?): Connection {
        keepSoundCloud(oauth, refresh)
        return connect(ProviderKind.SOUND_CLOUD)
    }

    /**
     * Sign in to a Navidrome server. The password goes into a salted token
     * at once and is never kept; what signs in again is kept once it has.
     */
    suspend fun signInToNavidrome(url: String, user: String, password: String): Connection {
        val kind = ProviderKind.NAVIDROME
        val credentials = try {
            session.navidromeCredentials(url, user, password)
        } catch (e: MusicException) {
            return Connection.Failed(e.message ?: e.toString()).also { connections.getValue(kind).value = it }
        }
        val got = attempt(kind) { session.connectNavidrome(credentials) }
        if (got is Connection.Connected) {
            vault.write(kind.vaultName, credentials)
            prefs.edit().putBoolean("enabled-${kind.name}", true).apply()
        }
        return got
    }

    private fun keepSoundCloud(oauth: String, refresh: String?) {
        vault.write(SOUNDCLOUD, JSONObject().put("oauth", oauth).put("refresh", refresh ?: JSONObject.NULL).toString())
    }

    /** SoundCloud renews its tokens as they expire: the new ones are kept. */
    private val keeper = object : TokenKeeper {
        override fun keep(oauth: String, refresh: String?) = keepSoundCloud(oauth, refresh)
    }

    private suspend fun connect(kind: ProviderKind): Connection {
        val kept = vault.read(kind.vaultName)
        if (kept == null) {
            // Kept but unreadable (its key gone, as after a restore): signed out, and told so.
            if (!vault.has(kind.vaultName)) return Connection.Off.also { connections.getValue(kind).value = it }
            vault.delete(kind.vaultName)
            return Connection.Failed("Sign in again").also { connections.getValue(kind).value = it }
        }
        return attempt(kind) {
            when (kind) {
                ProviderKind.APPLE_MUSIC -> session.connectApple(kept)
                ProviderKind.NAVIDROME -> session.connectNavidrome(kept)
                ProviderKind.SOUND_CLOUD -> {
                    val json = JSONObject(kept)
                    session.connectSoundcloud(json.getString("oauth"), json.optString("refresh").takeIf { !json.isNull("refresh") && it.isNotEmpty() }, keeper, soundCloudWeb)
                }
            }
        }
    }

    private suspend fun attempt(kind: ProviderKind, sign: suspend () -> String): Connection {
        val state = connections.getValue(kind)
        state.value = Connection.Connecting
        state.value = try {
            Connection.Connected(sign())
        } catch (e: MusicException) {
            Connection.Failed(e.message ?: e.toString())
        } catch (e: kotlinx.coroutines.CancellationException) {
            // Called off midway: not left Connecting for good.
            state.value = Connection.Failed("cancelled")
            throw e
        }
        if (kind == ProviderKind.APPLE_MUSIC) {
            _appleProfile.value = if (state.value is Connection.Connected) runCatching { session.appleProfile() }.getOrNull() else null
        }
        return state.value
    }

    private val ProviderKind.vaultName
        get() = when (this) {
            ProviderKind.APPLE_MUSIC -> APPLE
            ProviderKind.SOUND_CLOUD -> SOUNDCLOUD
            ProviderKind.NAVIDROME -> "navidrome"
        }

    private companion object {
        const val APPLE = "apple-music"
        const val SOUNDCLOUD = "soundcloud"
    }
}
