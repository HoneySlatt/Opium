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
import io.github.honeyslatt.music.ui.App
import io.github.honeyslatt.music.ui.connect.ConnectHost
import io.github.honeyslatt.music.ui.connect.Connecting
import io.github.honeyslatt.music.ui.connect.Welcome
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.ui.Modifier
import io.github.honeyslatt.music.ui.theme.MusicTheme
import kotlinx.coroutines.launch

class MainActivity : ComponentActivity() {
    private var controller: ListenableFuture<MediaController>? = null

    // The playing notification needs the permission since Android 13.
    private val notifications = registerForActivityResult(ActivityResultContracts.RequestPermission()) {}

    override fun onStart() {
        super.onStart()
        // Being connected to it is what keeps the session service running.
        controller = MediaController.Builder(this, SessionToken(this, ComponentName(this, PlaybackService::class.java))).buildAsync()
        (application as MusicApp).devices.looking(true)
        val app = application as MusicApp
        app.scope.launch { app.retry() }
    }

    @androidx.annotation.OptIn(androidx.media3.common.util.UnstableApi::class)
    override fun onStop() {
        (application as MusicApp).devices.looking(false)
        // Closed or not, what plays is kept, to be found as it was.
        (application as MusicApp).playback.keepSession()
        controller?.let(MediaController::releaseFuture)
        controller = null
        super.onStop()
    }

    /** In the app, the volume keys set the volume of the device played on, if any. */
    @androidx.annotation.OptIn(androidx.media3.common.util.UnstableApi::class)
    override fun onKeyDown(keyCode: Int, event: android.view.KeyEvent): Boolean {
        val up = keyCode == android.view.KeyEvent.KEYCODE_VOLUME_UP
        if ((up || keyCode == android.view.KeyEvent.KEYCODE_VOLUME_DOWN) && (application as MusicApp).playback.stepRemoteVolume(up)) return true
        return super.onKeyDown(keyCode, event)
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
            val themes by app.themes.all.collectAsStateWithLifecycle()
            val chosen by app.themes.chosen.collectAsStateWithLifecycle()
            val connecting = remember { Connecting() }
            var welcomed by remember { mutableStateOf(app.core.welcomed) }
            LaunchedEffect(Unit) { app.start() }
            MusicTheme(appearance, app.themes.current(themes, chosen)) {
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
