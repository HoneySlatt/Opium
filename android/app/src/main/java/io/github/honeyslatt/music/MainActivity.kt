package io.github.honeyslatt.music

import android.Manifest
import android.content.ComponentName
import android.content.pm.PackageManager
import android.os.Bundle
import androidx.activity.result.contract.ActivityResultContracts
import androidx.media3.session.MediaController
import androidx.media3.session.SessionToken
import com.google.common.util.concurrent.ListenableFuture
import io.github.honeyslatt.music.playback.PlaybackService
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import io.github.honeyslatt.music.core.model.ProviderKind
import io.github.honeyslatt.music.data.Connection
import io.github.honeyslatt.music.ui.App
import io.github.honeyslatt.music.ui.connect.ConnectHost
import io.github.honeyslatt.music.ui.connect.Connecting
import io.github.honeyslatt.music.ui.connect.Welcome
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.runtime.key
import androidx.compose.ui.Modifier
import io.github.honeyslatt.music.ui.theme.MusicTheme

class MainActivity : ComponentActivity() {
    private var controller: ListenableFuture<MediaController>? = null

    // The playing notification needs the permission since Android 13.
    private val notifications = registerForActivityResult(ActivityResultContracts.RequestPermission()) {}

    override fun onStart() {
        super.onStart()
        // Being connected to it is what keeps the session service running.
        controller = MediaController.Builder(this, SessionToken(this, ComponentName(this, PlaybackService::class.java))).buildAsync()
    }

    override fun onStop() {
        controller?.let(MediaController::releaseFuture)
        controller = null
        super.onStop()
    }

    override fun onCreate(savedInstanceState: Bundle?) {
        enableEdgeToEdge()
        super.onCreate(savedInstanceState)
        if (checkSelfPermission(Manifest.permission.POST_NOTIFICATIONS) != PackageManager.PERMISSION_GRANTED) {
            notifications.launch(Manifest.permission.POST_NOTIFICATIONS)
        }
        val app = application as MusicApp
        setContent {
            val appearance by app.core.appearance.collectAsStateWithLifecycle()
            val connecting = remember { Connecting() }
            var welcomed by remember { mutableStateOf(app.core.welcomed) }
            LaunchedEffect(Unit) { app.core.start() }
            // Each service's library shows as kept while it connects (or can't),
            // loads once it answers, and goes when it is off.
            ProviderKind.entries.forEach { kind ->
                key(kind) {
                    val state by app.core.connection(kind).collectAsStateWithLifecycle()
                    LaunchedEffect(state is Connection.Connected, state is Connection.Off) {
                        when (state) {
                            is Connection.Connected -> app.library.load(kind)
                            Connection.Off -> app.library.forget(kind)
                            else -> app.library.showKept(kind)
                        }
                    }
                }
            }
            MusicTheme(appearance) {
                Box(Modifier.fillMaxSize()) {
                    if (welcomed) {
                        App(app.core, app.library, app.playback, app.settings, connecting)
                    } else {
                        Welcome(app.core, connecting) {
                            app.core.welcomed = true
                            welcomed = true
                        }
                    }
                    ConnectHost(connecting, app.core)
                }
            }
        }
    }
}
