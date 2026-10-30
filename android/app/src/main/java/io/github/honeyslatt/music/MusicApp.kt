package io.github.honeyslatt.music

import android.app.Application
import io.github.honeyslatt.music.core.Tls
import io.github.honeyslatt.music.data.Core
import io.github.honeyslatt.music.data.Library
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.launch

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
                    scope.launch {
                        playback.online()
                        core.retryFailed()
                    }
                }
            },
        )
    }
}
