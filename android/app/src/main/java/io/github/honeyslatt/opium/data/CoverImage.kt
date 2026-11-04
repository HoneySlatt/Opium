package io.github.honeyslatt.opium.data

import android.content.Context
import android.graphics.Bitmap
import android.graphics.ImageDecoder
import android.net.Uri
import java.io.ByteArrayOutputStream
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext

/** A picture chosen as a playlist's cover, made as the services take one. */
object CoverImage {
    /** The side of a cover made: as large as the pages show one. */
    private const val SIDE = 600

    /**
     * The picture at `uri` (a photo, HEIC as well), its middle cut square
     * and brought to 600 px, as a JPEG; none if it can't be read.
     */
    suspend fun square(context: Context, uri: Uri): ByteArray? = withContext(Dispatchers.IO) {
        runCatching {
            val source = ImageDecoder.createSource(context.contentResolver, uri)
            val decoded = ImageDecoder.decodeBitmap(source) { decoder, info, _ ->
                // Read no larger than needed: its shorter side about SIDE.
                val shorter = minOf(info.size.width, info.size.height)
                if (shorter > SIDE * 2) decoder.setTargetSampleSize(shorter / SIDE / 2 * 2)
                decoder.allocator = ImageDecoder.ALLOCATOR_SOFTWARE
            }
            val side = minOf(decoded.width, decoded.height)
            val cut = Bitmap.createBitmap(decoded, (decoded.width - side) / 2, (decoded.height - side) / 2, side, side)
            val sized = if (side > SIDE) Bitmap.createScaledBitmap(cut, SIDE, SIDE, true) else cut
            ByteArrayOutputStream().use {
                sized.compress(Bitmap.CompressFormat.JPEG, 88, it)
                it.toByteArray()
            }
        }.getOrNull()
    }
}
