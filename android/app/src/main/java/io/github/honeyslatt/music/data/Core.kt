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

    private val connections = ProviderKind.entries.associateWith { MutableStateFlow<Connection>(Connection.Off) }

    fun connection(kind: ProviderKind): StateFlow<Connection> = connections.getValue(kind).asStateFlow()

    val apple: StateFlow<Connection> = connection(ProviderKind.APPLE_MUSIC)

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

    /** Sign in again with what was kept, every service at once. */
    suspend fun start() {
        kotlinx.coroutines.coroutineScope {
            ProviderKind.entries.filter { signedIn(it) && enabled(it) }.forEach { kind ->
                launch { connect(kind) }
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
        val kept = vault.read(kind.vaultName) ?: return Connection.Off.also { connections.getValue(kind).value = it }
        return attempt(kind) {
            when (kind) {
                ProviderKind.APPLE_MUSIC -> session.connectApple(kept)
                ProviderKind.NAVIDROME -> session.connectNavidrome(kept)
                ProviderKind.SOUND_CLOUD -> {
                    val json = JSONObject(kept)
                    session.connectSoundcloud(json.getString("oauth"), json.optString("refresh").takeIf { !json.isNull("refresh") && it.isNotEmpty() }, keeper)
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
