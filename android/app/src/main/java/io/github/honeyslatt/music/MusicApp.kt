package io.github.honeyslatt.music

import android.app.Application
import io.github.honeyslatt.music.core.Tls
import io.github.honeyslatt.music.data.Core
import io.github.honeyslatt.music.data.Library
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.flow.distinctUntilChanged
import kotlinx.coroutines.flow.map
import kotlinx.coroutines.launch
import io.github.honeyslatt.music.core.model.ProviderKind
import io.github.honeyslatt.music.data.Connection

class MusicApp : Application() {
    /** For work that outlives a screen: sign-ins, the library's loading. */
    val scope = CoroutineScope(SupervisorJob() + Dispatchers.Main)
    lateinit var core: Core
        private set
    lateinit var library: Library
        private set
    lateinit var settings: io.github.honeyslatt.music.data.Settings
        private set

    /** The user's other devices, through Music Connect. */
    lateinit var devices: io.github.honeyslatt.music.data.Devices
        private set

    @androidx.annotation.OptIn(androidx.media3.common.util.UnstableApi::class)
    lateinit var playback: io.github.honeyslatt.music.playback.Playback
        private set

    override fun onCreate() {
        super.onCreate()
        Tls.init(this)
        core = Core(this)
        settings = io.github.honeyslatt.music.data.Settings(this)
        library = Library(core, scope)
        devices = io.github.honeyslatt.music.data.Devices(this)
        playback = io.github.honeyslatt.music.playback.Playback(this, core, scope, devices)
        // Back on a network (out of the metro, onto the Wi-Fi): the services
        // it kept out connect, and a song it stopped goes on.
        getSystemService(android.net.ConnectivityManager::class.java).registerDefaultNetworkCallback(
            object : android.net.ConnectivityManager.NetworkCallback() {
                override fun onAvailable(network: android.net.Network) {
                    scope.launch { retry() }
                }

                // Asleep (Doze), the app is kept off the network it has: woken, it is let back on.
                override fun onBlockedStatusChanged(network: android.net.Network, blocked: Boolean) {
                    if (!blocked) scope.launch { retry() }
                }
            },
        )
    }

    /**
     * What the network kept out, tried again: a song it stopped, the
     * services, the library's lists. When it comes back, and when the app
     * shows again.
     */
    suspend fun retry() {
        if (!started) return
        playback.online()
        library.retryFailed()
        core.retryFailed()
    }

    private var started = false

    /**
     * Connect the services and keep the library in step with them: each
     * service's lists show as kept while it connects (or can't), load once
     * it answers, and go when it is off. Once, whether the screen or the
     * car (which opens the app's music without it) comes first.
     */
    fun start() {
        if (started) return
        started = true
        scope.launch { core.start() }
        ProviderKind.entries.forEach { kind ->
            scope.launch {
                core.connection(kind).map { (it is Connection.Connected) to (it == Connection.Off) }.distinctUntilChanged().collect {
                    when (core.connection(kind).value) {
                        is Connection.Connected -> library.load(kind)
                        Connection.Off -> library.forget(kind)
                        else -> library.showKept(kind)
                    }
                }
            }
        }
    }
}
