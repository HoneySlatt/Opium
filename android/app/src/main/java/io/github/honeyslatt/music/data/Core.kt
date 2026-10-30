package io.github.honeyslatt.music.data

import android.content.Context
import io.github.honeyslatt.music.core.MusicException
import io.github.honeyslatt.music.core.Session
import io.github.honeyslatt.music.ui.theme.Appearance
import java.io.File
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow

/** How a service stands. */
sealed interface Connection {
    data object Off : Connection
    data object Connecting : Connection
    data class Connected(val name: String) : Connection
    data class Failed(val reason: String) : Connection
}

/**
 * The Rust core's session, and what the app keeps between launches: the
 * sign-ins and the settings.
 */
class Core(context: Context) {
    val session = Session()
    private val prefs = context.getSharedPreferences("music", Context.MODE_PRIVATE)

    // The account token, readable by the app only. Android Keystore comes
    // with the sign-in screens.
    private val appleToken = File(context.filesDir, "apple-token")

    private val _apple = MutableStateFlow<Connection>(Connection.Off)
    val apple: StateFlow<Connection> = _apple.asStateFlow()

    private val _appearance = MutableStateFlow(
        runCatching { Appearance.valueOf(prefs.getString("appearance", null) ?: "") }
            .getOrDefault(Appearance.Automatic),
    )
    val appearance: StateFlow<Appearance> = _appearance.asStateFlow()

    fun setAppearance(appearance: Appearance) {
        _appearance.value = appearance
        prefs.edit().putString("appearance", appearance.name).apply()
    }

    val hasAppleToken get() = appleToken.exists()

    /** A choice the app remembers: a sort order, a switch. */
    fun setting(key: String): String? = prefs.getString(key, null)

    fun setSetting(key: String, value: String) {
        prefs.edit().putString(key, value).apply()
    }

    /** Settings → Apple Music → Use Apple Music: off keeps the sign-in. */
    val appleEnabled get() = prefs.getBoolean("apple-enabled", true)

    /** Sign in again with what was kept. */
    suspend fun start() {
        if (hasAppleToken && appleEnabled) connectApple(appleToken.readText())
    }

    suspend fun setAppleEnabled(on: Boolean) {
        prefs.edit().putBoolean("apple-enabled", on).apply()
        if (on) {
            start()
        } else {
            session.disconnect(io.github.honeyslatt.music.core.model.ProviderKind.APPLE_MUSIC)
            _apple.value = Connection.Off
        }
    }

    /** Forget the account: its token goes from the phone. */
    fun signOutOfApple() {
        session.disconnect(io.github.honeyslatt.music.core.model.ProviderKind.APPLE_MUSIC)
        appleToken.delete()
        _apple.value = Connection.Off
    }

    /** The token Apple's sign-in page left, kept and used. */
    suspend fun signedInToApple(token: String) {
        appleToken.writeText(token)
        connectApple(token)
    }

    private suspend fun connectApple(token: String) {
        _apple.value = Connection.Connecting
        _apple.value = try {
            Connection.Connected(session.connectApple(token))
        } catch (e: MusicException) {
            Connection.Failed(e.message ?: e.toString())
        }
    }
}
