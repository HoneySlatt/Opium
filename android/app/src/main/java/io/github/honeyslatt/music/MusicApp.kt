package io.github.honeyslatt.music

import android.app.Application
import io.github.honeyslatt.music.core.Tls
import io.github.honeyslatt.music.data.Core
import io.github.honeyslatt.music.data.Library
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob

class MusicApp : Application() {
    /** For work that outlives a screen: sign-ins, the library's loading. */
    val scope = CoroutineScope(SupervisorJob() + Dispatchers.Main)
    lateinit var core: Core
        private set
    lateinit var library: Library
        private set

    override fun onCreate() {
        super.onCreate()
        Tls.init(this)
        core = Core(this)
        library = Library(core, scope)
    }
}
