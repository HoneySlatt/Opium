package io.github.honeyslatt.music.playback

import android.content.ContentProvider
import android.content.ContentValues
import android.content.Context
import android.database.Cursor
import android.net.Uri
import android.os.ParcelFileDescriptor
import java.io.File
import java.io.FileNotFoundException
import java.util.concurrent.ConcurrentHashMap
import okhttp3.Request

/**
 * The covers Android Auto shows while browsing: it reads them from a
 * content URI rather than the web. Only covers the app has offered are
 * served, downloaded once and kept in the cache.
 */
class ArtProvider : ContentProvider() {
    companion object {
        /** By key, the covers offered since the app started. */
        private val offered = ConcurrentHashMap<String, String>()

        /** The URI Android Auto reads the cover at `url` from. */
        fun uri(context: Context, url: String): Uri {
            val key = "%016x".format(url.fold(-0x340d631b7bdddcdbL) { h, c -> (h xor c.code.toLong()) * 0x100000001b3L })
            offered[key] = url
            return Uri.parse("content://${context.packageName}.art/$key")
        }

        /** How many covers are kept: the oldest go past it. */
        private const val KEPT = 500
    }

    private val dir get() = File(context!!.cacheDir, "car-art")

    override fun onCreate(): Boolean {
        Thread {
            val files = dir.listFiles()?.sortedByDescending { it.lastModified() } ?: return@Thread
            files.drop(KEPT).forEach { it.delete() }
        }.start()
        return true
    }

    override fun openFile(uri: Uri, mode: String): ParcelFileDescriptor {
        val key = uri.lastPathSegment?.takeIf { it.matches(Regex("[0-9a-f]{16}")) } ?: throw FileNotFoundException()
        val file = File(dir, key)
        if (!file.exists()) {
            val url = offered[key] ?: throw FileNotFoundException()
            dir.mkdirs()
            val part = File(dir, "$key.part")
            // Out of reach: said as the one failure a provider may give.
            try {
                Http.client.newCall(Request.Builder().url(url).build()).execute().use { response ->
                    if (!response.isSuccessful) throw FileNotFoundException("HTTP ${response.code}")
                    part.outputStream().use { out -> (response.body ?: throw FileNotFoundException()).byteStream().copyTo(out) }
                }
            } catch (e: java.io.IOException) {
                part.delete()
                throw e as? FileNotFoundException ?: FileNotFoundException(e.message)
            }
            part.renameTo(file)
        }
        return ParcelFileDescriptor.open(file, ParcelFileDescriptor.MODE_READ_ONLY)
    }

    override fun getType(uri: Uri) = "image/jpeg"

    override fun query(uri: Uri, projection: Array<out String>?, selection: String?, selectionArgs: Array<out String>?, sortOrder: String?): Cursor? = null

    override fun insert(uri: Uri, values: ContentValues?): Uri? = null

    override fun delete(uri: Uri, selection: String?, selectionArgs: Array<out String>?) = 0

    override fun update(uri: Uri, values: ContentValues?, selection: String?, selectionArgs: Array<out String>?) = 0
}
