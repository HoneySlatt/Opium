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
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import io.github.honeyslatt.music.core.model.ProviderKind
import io.github.honeyslatt.music.data.Connection
import io.github.honeyslatt.music.ui.App
import io.github.honeyslatt.music.ui.AppleSignIn
import io.github.honeyslatt.music.ui.SignInPrompt
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
            val apple by app.core.apple.collectAsStateWithLifecycle()
            var signingIn by remember { mutableStateOf(false) }
            val scope = rememberCoroutineScope()
            LaunchedEffect(Unit) { app.core.start() }
            // The library loads once Apple Music answers.
            LaunchedEffect(apple is Connection.Connected) {
                if (apple is Connection.Connected) {
                    app.library.load(ProviderKind.APPLE_MUSIC)
                } else {
                    app.library.forget(ProviderKind.APPLE_MUSIC)
                }
            }
            MusicTheme(appearance) {
                when {
                    signingIn -> AppleSignIn { token ->
                        signingIn = false
                        scope.launch { app.core.signedInToApple(token) }
                    }
                    !app.core.hasAppleToken && apple !is Connection.Connecting -> SignInPrompt { signingIn = true }
                    else -> App(app.core, app.library, app.playback, app.settings) { signingIn = true }
                }
            }
        }
    }
}
