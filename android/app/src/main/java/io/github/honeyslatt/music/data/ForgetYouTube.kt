package io.github.honeyslatt.music.data

import android.content.Context
import android.webkit.CookieManager
import android.webkit.WebStorage
import java.io.File

/**
 * What the YouTube Music trial, taken out of the app on 2026-11-02, left on
 * the phone: its sign-in in the vault, its lists kept for the library, and
 * the Google and YouTube cookies and storage of its sign-in page. Those of
 * other sites are left alone: SoundCloud's likes go through the same
 * WebView. Done once all of it is gone; tried again at the next launch if not.
 */
fun forgetYouTube(context: Context) {
    val prefs = context.getSharedPreferences("music", Context.MODE_PRIVATE)
    if (prefs.getBoolean(FORGOTTEN, false)) return
    val vault = Vault(context)
    vault.delete("youtube-music")
    vault.delete("youtube-music.part")
    File(context.cacheDir, "library").listFiles { file -> file.name.contains("-YOU_TUBE_MUSIC.json") }
        ?.forEach { it.delete() }
    ORIGINS.forEach { WebStorage.getInstance().deleteOrigin(it) }
    val cookies = CookieManager.getInstance()
    for (url in ORIGINS) {
        val names = cookies.getCookie(url)?.split(';')?.map { it.substringBefore('=').trim() }.orEmpty()
        for (name in names.filter { it.isNotEmpty() }) {
            // Set again as expired, under each scope it may have been set with:
            // the host alone, or the whole site.
            for (scope in listOf("", "; Domain=.${site(url)}")) {
                for (kind in listOf("; Secure", "; Secure; SameSite=None; Partitioned", "")) {
                    cookies.setCookie(url, "$name=; Max-Age=0; Path=/$scope$kind")
                }
            }
        }
    }
    cookies.flush()
    if (ORIGINS.all { cookies.getCookie(it).isNullOrEmpty() }) prefs.edit().putBoolean(FORGOTTEN, true).apply()
}

/** `music.youtube.com` → `youtube.com`. */
private fun site(url: String) = url.removePrefix("https://").split('.').takeLast(2).joinToString(".")

private const val FORGOTTEN = "youtube-forgotten"

private val ORIGINS = listOf(
    "https://music.youtube.com",
    "https://www.youtube.com",
    "https://youtube.com",
    "https://m.youtube.com",
    "https://accounts.youtube.com",
    "https://accounts.google.com",
    "https://www.google.com",
    "https://google.com",
    "https://accounts.google.fr",
    "https://www.google.fr",
)
