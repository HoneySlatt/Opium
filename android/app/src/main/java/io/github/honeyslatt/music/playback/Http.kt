package io.github.honeyslatt.music.playback

import androidx.media3.common.util.UnstableApi
import androidx.media3.datasource.okhttp.OkHttpDataSource
import io.github.honeyslatt.music.core.appleAssetHost
import okhttp3.OkHttpClient
import okhttp3.Request

/**
 * One HTTP client for what the player downloads, so its connections are
 * kept between songs; the one to Apple's files is opened ahead, for the
 * first song to start as fast as the next.
 */
@UnstableApi
object Http {
    val client = OkHttpClient()
    val sources = OkHttpDataSource.Factory(client)

    fun warm() {
        Thread {
            runCatching { client.newCall(Request.Builder().url(appleAssetHost()).head().build()).execute().close() }
        }.start()
    }
}
