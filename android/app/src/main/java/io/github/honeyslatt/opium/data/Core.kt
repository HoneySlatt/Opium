package io.github.honeyslatt.opium.data

import android.content.Context
import io.github.honeyslatt.opium.core.OpiumException
import io.github.honeyslatt.opium.core.Session
import io.github.honeyslatt.opium.core.TokenKeeper
import io.github.honeyslatt.opium.core.model.ProviderKind
import io.github.honeyslatt.opium.ui.theme.Appearance
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

/** The services one signs in to: Opium's own playlists are none, nor the phone's files. */
val SERVICES = ProviderKind.entries - ProviderKind.OPIUM - ProviderKind.LOCAL

/** Where the library's songs come from: the services and the phone's files. */
val SOURCES = SERVICES + ProviderKind.LOCAL

/**
 * The Rust core's session, and what the app keeps between launches: the
 * sign-ins (in the vault) and the settings.
 */
class Core(context: Context) {
    private val connectivity = context.getSystemService(android.net.ConnectivityManager::class.java)

    /** On mobile data (or another network that counts what it carries) now. */
    val metered: Boolean get() = connectivity.isActiveNetworkMetered
    val session = Session().apply { keepIn(context.filesDir.path) }
    private val prefs = context.getSharedPreferences("opium", Context.MODE_PRIVATE)
    /** The sign-ins, and the downloads' key. */
    val vault = Vault(context)

    /** Sends SoundCloud's changes, as only a browser's are taken. */
    val soundCloudWeb = SoundCloudWeb(context.applicationContext)

    private val connections = ProviderKind.entries.associateWith { MutableStateFlow<Connection>(Connection.Off) }

    private val shown = connections.mapValues { it.value.asStateFlow() }

    /** The same flow each time: collected anew at each recomposition otherwise. */
    fun connection(kind: ProviderKind): StateFlow<Connection> = shown.getValue(kind)

    /** The phone's files, as [LocalFiles] reads them: their count, or Off. */
    fun filesRead(state: Connection) {
        connections.getValue(ProviderKind.LOCAL).value = state
    }

    val apple: StateFlow<Connection> = connection(ProviderKind.APPLE_MUSIC)

    private val _appleProfile = MutableStateFlow<io.github.honeyslatt.opium.core.AppleProfile?>(null)

    /** The account's profile on Apple Music, once connected and read. */
    val appleProfile: StateFlow<io.github.honeyslatt.opium.core.AppleProfile?> = _appleProfile.asStateFlow()

    init {
        // The account token from before the vault: moved in. Left in place when it
        // can't be, to move at the next start.
        val old = File(context.filesDir, "apple-token")
        if (old.exists()) {
            val token = runCatching { old.readText() }.getOrNull()
            if (token != null && vault.write(APPLE, token).isSuccess) old.delete()
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
        get() = prefs.getBoolean("welcomed", false) || SERVICES.any { signedIn(it) }
        set(v) = prefs.edit().putBoolean("welcomed", v).apply()

    /**
     * Sign in again with what was kept, every service at once; those that
     * are already, or on their way, are left be (the screen made anew calls
     * this again).
     */
    suspend fun start() {
        kotlinx.coroutines.coroutineScope {
            SERVICES.filter { signedIn(it) && enabled(it) }.forEach { kind ->
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
            SERVICES.filter { signedIn(it) && enabled(it) && connections.getValue(it).value is Connection.Failed }.forEach { kind ->
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

    /**
     * The library's lists as last read, by service: see [Library]. In the
     * app's files, as the songs downloaded need them offline: a cache may be
     * cleared. The lists from before, in the cache, are moved in.
     *
     * The folder's name must match `KEPT_LISTS` in `crates/opium-ffi`, whose
     * `keepIn` sweeps the partial files a crash left here. Those among the
     * lists moved in from the cache, after `keepIn` ran, go at the next start.
     */
    val kept = File(context.filesDir, "library").apply {
        val old = File(context.cacheDir, "library")
        if (!exists() && old.exists()) old.renameTo(this)
        mkdirs()
    }

    /** Signed out of a service: its downloads are told, its protected songs then no longer playing. */
    var signedOut: (ProviderKind) -> Unit = {}

    /** Forget the account: its sign-in goes from the phone, and its library as kept; its downloads stay. */
    fun signOut(kind: ProviderKind) {
        session.disconnect(kind)
        vault.delete(kind.vaultName)
        kept.listFiles { f -> f.name.endsWith("-${kind.name}.json") }?.forEach { it.delete() }
        signedOut(kind)
        connections.getValue(kind).value = Connection.Off
    }

    /** The token Apple's sign-in page left, kept and used; failed if it can't be kept. */
    suspend fun signedInToApple(token: String): Connection {
        vault.write(APPLE, token).onFailure { return notKept(ProviderKind.APPLE_MUSIC, it) }
        return connect(ProviderKind.APPLE_MUSIC)
    }

    /** The tokens SoundCloud's web player left, kept and used; failed if they can't be kept. */
    suspend fun signedInToSoundCloud(oauth: String, refresh: String?): Connection {
        keepSoundCloud(oauth, refresh).onFailure { return notKept(ProviderKind.SOUND_CLOUD, it) }
        return connect(ProviderKind.SOUND_CLOUD)
    }

    /**
     * A sign-in the vault couldn't keep (the Keystore failing, the disk full): not used, and shown
     * failed, to try again. The vault still holds what it held: the next start finds no sign-in,
     * or the service's older one, kept before, and uses that.
     */
    private fun notKept(kind: ProviderKind, error: Throwable): Connection {
        session.disconnect(kind)
        return Connection.Failed("Couldn’t keep the sign-in on this phone (${error.javaClass.simpleName}); try again")
            .also { connections.getValue(kind).value = it }
    }

    /**
     * Sign in to a Navidrome server. The password goes into a salted token
     * at once and is never kept; what signs in again is kept once it has.
     */
    suspend fun signInToNavidrome(url: String, user: String, password: String): Connection {
        val kind = ProviderKind.NAVIDROME
        val credentials = try {
            session.navidromeCredentials(url, user, password)
        } catch (e: OpiumException) {
            return Connection.Failed(e.message ?: e.toString()).also { connections.getValue(kind).value = it }
        }
        val got = attempt(kind) { session.connectNavidrome(credentials) }
        if (got is Connection.Connected) {
            vault.write(kind.vaultName, credentials).onFailure { return notKept(kind, it) }
            prefs.edit().putBoolean("enabled-${kind.name}", true).apply()
            // A session on its own API too, for the changes that need one
            // (a playlist's cover); kept with the rest, never the password.
            runCatching { openNavidromeSession(password) }
        }
        return got
    }

    /**
     * A session on the Navidrome server's own API, for `password`, kept with
     * the sign-in: whether the server gave it (a wrong password: no).
     */
    suspend fun openNavidromeSession(password: String): Boolean {
        val opened = session.navidromeOpenSession(password)
        if (opened) keepNavidrome()
        return opened
    }

    /**
     * The Navidrome sign-in as it stands, its session renewed, kept in place of the last. Not
     * kept (the Keystore failing), the last one stays, and is used at the next start.
     */
    fun keepNavidrome() {
        session.navidromeKept()?.let { kept ->
            vault.write(ProviderKind.NAVIDROME.vaultName, kept).onFailure { notRenewed("Navidrome", it) }
        }
    }

    private fun keepSoundCloud(oauth: String, refresh: String?): Result<Unit> =
        vault.write(SOUNDCLOUD, JSONObject().put("oauth", oauth).put("refresh", refresh ?: JSONObject.NULL).toString())

    /**
     * SoundCloud renews its tokens as they expire: the new ones are kept. Not kept (the
     * Keystore failing), the last ones stay; if SoundCloud no longer takes them at the next
     * start, it asks to sign in again.
     */
    private val keeper = object : TokenKeeper {
        override fun keep(oauth: String, refresh: String?) {
            keepSoundCloud(oauth, refresh).onFailure { notRenewed("SoundCloud", it) }
        }
    }

    /** A sign-in renewed but not kept, written to the system's log: nothing else can tell. */
    private fun notRenewed(service: String, error: Throwable) {
        android.util.Log.w("Opium", "$service: couldn't keep the renewed sign-in (${error.javaClass.simpleName}: ${error.message})")
    }

    private suspend fun connect(kind: ProviderKind): Connection {
        val state = connections.getValue(kind)
        val kept = when (val step = signInStep(vault.read(kind.vaultName))) {
            SignInStep.NotSignedIn -> return Connection.Off.also { state.value = it }
            // Gone for good (its key lost, as after a restore): signed out, and told so.
            SignInStep.SignInAgain -> {
                vault.delete(kind.vaultName)
                return Connection.Failed("Sign in again").also { state.value = it }
            }
            // Not readable now: kept, and tried again with the network or at the next start.
            is SignInStep.TryLater ->
                return Connection.Failed("Couldn’t read the sign-in kept on this phone (${step.why})").also { state.value = it }
            is SignInStep.Use -> step.kept
        }
        return attempt(kind) {
            when (kind) {
                ProviderKind.APPLE_MUSIC -> session.connectApple(kept)
                ProviderKind.NAVIDROME -> session.connectNavidrome(kept).also { keepNavidrome() }
                ProviderKind.SOUND_CLOUD -> {
                    val json = JSONObject(kept)
                    session.connectSoundcloud(json.getString("oauth"), json.optString("refresh").takeIf { !json.isNull("refresh") && it.isNotEmpty() }, keeper, soundCloudWeb)
                }
                ProviderKind.OPIUM -> error("Opium's playlists need no sign-in")
                ProviderKind.LOCAL -> error("the files need no sign-in")
            }
        }
    }

    private suspend fun attempt(kind: ProviderKind, sign: suspend () -> String): Connection {
        val state = connections.getValue(kind)
        state.value = Connection.Connecting
        state.value = try {
            Connection.Connected(sign())
        } catch (e: OpiumException) {
            Connection.Failed(e.message ?: e.toString())
        } catch (e: kotlinx.coroutines.CancellationException) {
            // Called off midway: not left Connecting for good.
            state.value = Connection.Failed("cancelled")
            throw e
        } catch (e: Exception) {
            // Unforeseen (a kept sign-in that doesn't parse, a fault in the core): failed, never
            // a crash. Its message is left out: it may quote the sign-in.
            Connection.Failed("Couldn’t sign in (${e.javaClass.simpleName})")
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
            ProviderKind.OPIUM -> "opium"
            ProviderKind.LOCAL -> "local"
        }

    private companion object {
        const val APPLE = "apple-music"
        const val SOUNDCLOUD = "soundcloud"
    }
}
