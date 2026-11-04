package io.github.honeyslatt.opium.data

import android.Manifest
import android.content.ContentUris
import android.content.Context
import android.content.pm.PackageManager
import android.database.ContentObserver
import android.net.Uri
import android.os.Handler
import android.os.Looper
import android.provider.MediaStore
import coil3.asImage
import io.github.honeyslatt.opium.core.LocalFile
import io.github.honeyslatt.opium.core.model.ProviderKind
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.Job
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext

/**
 * The phone's own music files, in the folders chosen: read from Android's
 * media index (which keeps up with the files itself), handed to the core as
 * the library's files, and read again as the index changes.
 */
class LocalFiles(private val context: Context, private val core: Core, private val library: Library, private val scope: CoroutineScope) {
    private val prefs = context.getSharedPreferences("opium", Context.MODE_PRIVATE)

    private val _folders = MutableStateFlow(prefs.getStringSet(FOLDERS, emptySet()).orEmpty())

    /** The folders chosen, as the index names them: `Music/`, `Download/`. */
    val folders: StateFlow<Set<String>> = _folders.asStateFlow()

    private val _found = MutableStateFlow<List<Pair<String, Int>>>(emptyList())

    /** The folders with music in them, and how many songs each: to choose from. */
    val found: StateFlow<List<Pair<String, Int>>> = _found.asStateFlow()

    /** Whether Android lets the app read the music files. */
    fun allowed() = context.checkSelfPermission(Manifest.permission.READ_MEDIA_AUDIO) == PackageManager.PERMISSION_GRANTED

    private var reading: Job? = null
    private var watching = false

    /** Read the files now, and again as the index changes. */
    fun start() {
        if (allowed() && !watching) {
            watching = true
            context.contentResolver.registerContentObserver(
                MediaStore.Audio.Media.EXTERNAL_CONTENT_URI,
                true,
                object : ContentObserver(Handler(Looper.getMainLooper())) {
                    override fun onChange(selfChange: Boolean) = read(settle = true)
                },
            )
        }
        read(settle = false)
    }

    /** Use the files of `folder`, or no longer. */
    fun choose(folder: String, on: Boolean) {
        val chosen = if (on) _folders.value + folder else _folders.value - folder
        _folders.value = chosen
        prefs.edit().putStringSet(FOLDERS, chosen).apply()
        read(settle = false)
    }

    /**
     * Read the index; `settle`: once changes stop coming (a copy of an
     * album changes it at each file).
     */
    private fun read(settle: Boolean) {
        reading?.cancel()
        reading = scope.launch {
            if (settle) delay(1500)
            if (!allowed()) {
                _found.value = emptyList()
                return@launch handOver(emptyList())
            }
            // The index out of reach (MediaProvider gone): what was read stays.
            val (files, found) = withContext(Dispatchers.IO) { runCatching { query() } }.getOrNull() ?: return@launch
            _found.value = found
            handOver(files)
        }
    }

    private suspend fun handOver(files: List<LocalFile>) {
        val was = core.connection(ProviderKind.LOCAL).value
        // Every record over to the core: too much to marshal on the screen's thread.
        withContext(Dispatchers.IO) { core.session.setLocalFiles(files) }
        if (files.isEmpty()) {
            core.filesRead(Connection.Off)
            return
        }
        val count = "${files.size} song${if (files.size == 1) "" else "s"}"
        core.filesRead(Connection.Connected(count))
        // Read before: the library's lists of them anew.
        if (was is Connection.Connected) library.reload(ProviderKind.LOCAL)
    }

    /** The files of the folders chosen, and every folder with music in it. */
    private fun query(): Pair<List<LocalFile>, List<Pair<String, Int>>> {
        val chosen = _folders.value
        val files = mutableListOf<LocalFile>()
        val counts = sortedMapOf<String, Int>()
        val columns = arrayOf(
            MediaStore.Audio.Media._ID,
            MediaStore.Audio.Media.RELATIVE_PATH,
            MediaStore.Audio.Media.DISPLAY_NAME,
            MediaStore.Audio.Media.TITLE,
            MediaStore.Audio.Media.ARTIST,
            MediaStore.Audio.Media.ALBUM_ARTIST,
            MediaStore.Audio.Media.ALBUM,
            MediaStore.Audio.Media.CD_TRACK_NUMBER,
            MediaStore.Audio.Media.DISC_NUMBER,
            MediaStore.Audio.Media.YEAR,
            MediaStore.Audio.Media.GENRE,
            MediaStore.Audio.Media.DURATION,
            MediaStore.Audio.Media.SIZE,
            MediaStore.Audio.Media.DATE_ADDED,
            MediaStore.Audio.Media.DATE_MODIFIED,
            MediaStore.Audio.Media.MIME_TYPE,
            MediaStore.Audio.Media.BITRATE,
            SAMPLE_RATE,
            BITS,
        )
        context.contentResolver.query(
            MediaStore.Audio.Media.EXTERNAL_CONTENT_URI,
            columns,
            "${MediaStore.Audio.Media.IS_MUSIC} != 0",
            null,
            null,
        )?.use { c ->
            val col = { name: String -> c.getColumnIndexOrThrow(name) }
            val id = col(MediaStore.Audio.Media._ID)
            val path = col(MediaStore.Audio.Media.RELATIVE_PATH)
            val name = col(MediaStore.Audio.Media.DISPLAY_NAME)
            val title = col(MediaStore.Audio.Media.TITLE)
            val artist = col(MediaStore.Audio.Media.ARTIST)
            val albumArtist = col(MediaStore.Audio.Media.ALBUM_ARTIST)
            val album = col(MediaStore.Audio.Media.ALBUM)
            val track = col(MediaStore.Audio.Media.CD_TRACK_NUMBER)
            val disc = col(MediaStore.Audio.Media.DISC_NUMBER)
            val year = col(MediaStore.Audio.Media.YEAR)
            val genre = col(MediaStore.Audio.Media.GENRE)
            val duration = col(MediaStore.Audio.Media.DURATION)
            val size = col(MediaStore.Audio.Media.SIZE)
            val added = col(MediaStore.Audio.Media.DATE_ADDED)
            val modified = col(MediaStore.Audio.Media.DATE_MODIFIED)
            val mime = col(MediaStore.Audio.Media.MIME_TYPE)
            val bitrate = col(MediaStore.Audio.Media.BITRATE)
            // Not in every index: read when there.
            val rate = c.getColumnIndex(SAMPLE_RATE)
            val bits = c.getColumnIndex(BITS)
            val text = { i: Int -> c.getString(i)?.trim()?.takeIf { it.isNotEmpty() && it != MediaStore.UNKNOWN_STRING } }
            while (c.moveToNext()) {
                val relative = c.getString(path) ?: continue
                // Counted by the folder at the top: `Music/`, `Download/`.
                val top = relative.substringBefore('/') + "/"
                counts[top] = (counts[top] ?: 0) + 1
                if (chosen.none { relative.startsWith(it) }) continue
                val uri = ContentUris.withAppendedId(MediaStore.Audio.Media.EXTERNAL_CONTENT_URI, c.getLong(id))
                files += LocalFile(
                    uri = uri.toString(),
                    size = c.getLong(size).toULong(),
                    modified = c.getLong(modified).toULong(),
                    added = c.getLong(added).toULong(),
                    title = text(title) ?: c.getString(name)?.substringBeforeLast('.').orEmpty(),
                    artists = listOfNotNull(text(artist)),
                    albumArtist = text(albumArtist),
                    album = text(album),
                    // `3/12`, or `3`.
                    track = text(track)?.substringBefore('/')?.toUIntOrNull(),
                    disc = text(disc)?.substringBefore('/')?.toUIntOrNull(),
                    year = c.getInt(year).takeIf { it > 0 }?.toUShort(),
                    genre = text(genre),
                    durationMs = c.getLong(duration).toULong(),
                    mime = c.getString(mime).orEmpty(),
                    bitrateKbps = c.getLong(bitrate).takeIf { it > 0 }?.let { (it / 1000).toUInt() },
                    sampleRate = if (rate >= 0) c.getInt(rate).takeIf { it > 0 }?.toUInt() else null,
                    bitDepth = if (bits >= 0) c.getInt(bits).takeIf { it in 1..64 }?.toUByte() else null,
                    cover = "$COVER$uri",
                )
            }
        }
        return files to counts.map { it.key to it.value }
    }

    /** The tags written in a file of the phone's, as the Info sheet shows them. */
    suspend fun tags(track: io.github.honeyslatt.opium.core.model.Track): io.github.honeyslatt.opium.core.FileTags? = withContext(Dispatchers.IO) {
        runCatching {
            context.contentResolver.openFileDescriptor(Uri.parse(track.id.id), "r")?.let { io.github.honeyslatt.opium.core.readFileTags(it.detachFd()) }
        }.getOrNull()
    }

    /** The lyrics in a file of the phone's tags, if any. */
    suspend fun lyrics(track: io.github.honeyslatt.opium.core.model.Track): io.github.honeyslatt.opium.core.model.Lyrics? = withContext(Dispatchers.IO) {
        if (!track.id.id.startsWith("content://")) return@withContext null
        runCatching {
            context.contentResolver.openFileDescriptor(Uri.parse(track.id.id), "r")?.let { io.github.honeyslatt.opium.core.fileLyrics(it.detachFd(), track.title) }
        }.getOrNull()
    }

    /**
     * Android's leave to change the files of `tracks`, asked of the user:
     * their window; none for a file the index hasn't, which it wouldn't be
     * asked for.
     */
    fun writeRequest(tracks: List<io.github.honeyslatt.opium.core.model.Track>): android.content.IntentSender? {
        if (tracks.any { !inIndex(it.id.id) }) return null
        return MediaStore.createWriteRequest(context.contentResolver, tracks.map { Uri.parse(it.id.id) }).intentSender
    }

    /** Write `edit` into the files of `tracks`, leave given; why some couldn't be, if any. */
    suspend fun write(tracks: List<io.github.honeyslatt.opium.core.model.Track>, edit: io.github.honeyslatt.opium.core.FileTagEdit): String? = withContext(Dispatchers.IO) {
        val failed = tracks.mapNotNull { track ->
            runCatching {
                if (!inIndex(track.id.id)) error("the file isn't in Android's music index")
                val fd = context.contentResolver.openFileDescriptor(Uri.parse(track.id.id), "rw") ?: error("the file couldn't be opened")
                io.github.honeyslatt.opium.core.writeFileTags(fd.detachFd(), edit)
            }.exceptionOrNull()?.let { "${track.title}: ${it.message}" }
        }
        // Android reads a file written again as it is closed: the library follows.
        failed.takeIf { it.isNotEmpty() }?.joinToString("; ")
    }

    /** A file opened from another app, as a song: its tags as Android reads them. */
    suspend fun opened(uri: Uri): io.github.honeyslatt.opium.core.model.Track = withContext(Dispatchers.IO) {
        val name = runCatching {
            context.contentResolver.query(uri, arrayOf(android.provider.OpenableColumns.DISPLAY_NAME), null, null, null)?.use {
                if (it.moveToFirst()) it.getString(0) else null
            }
        }.getOrNull()
        val tags = android.media.MediaMetadataRetriever()
        val read = { key: Int -> runCatching { tags.extractMetadata(key) }.getOrNull()?.trim()?.takeIf { it.isNotEmpty() } }
        runCatching { tags.setDataSource(context, uri) }
        try {
            io.github.honeyslatt.opium.core.model.Track(
                id = io.github.honeyslatt.opium.core.model.ItemId(ProviderKind.LOCAL, uri.toString()),
                title = read(android.media.MediaMetadataRetriever.METADATA_KEY_TITLE) ?: name?.substringBeforeLast('.') ?: "Untitled",
                artists = listOfNotNull(read(android.media.MediaMetadataRetriever.METADATA_KEY_ARTIST)),
                album = read(android.media.MediaMetadataRetriever.METADATA_KEY_ALBUM),
                albumId = null,
                duration = read(android.media.MediaMetadataRetriever.METADATA_KEY_DURATION)?.toLongOrNull()?.let { java.time.Duration.ofMillis(it) },
                trackNumber = null,
                discNumber = null,
                artwork = io.github.honeyslatt.opium.core.model.Artwork("$COVER$uri", null),
                explicit = false,
                quality = null,
                inLibrary = false,
                libraryId = null,
            )
        } finally {
            tags.release()
        }
    }

    companion object {
        private const val FOLDERS = "local-folders"

        /** The index's sample rate and bit depth, as Android 14 names them. */
        private const val SAMPLE_RATE = "samplerate"
        private const val BITS = "bits_per_sample"

        /** A file's cover, as the covers fetch it: this, then its address. */
        const val COVER = "opium-cover:"

        /**
         * Whether `id` names a file of Android's media index. A file opened
         * from another app comes by another provider's address, which the
         * index doesn't know and won't give leave to change.
         */
        fun inIndex(id: String) = id.startsWith("content://${MediaStore.AUTHORITY}/")
    }

    /** A file's own cover, as Android makes it: none if it has none. */
    class CoverFetcher(private val context: Context, private val uri: Uri) : coil3.fetch.Fetcher {
        override suspend fun fetch(): coil3.fetch.FetchResult? = withContext(Dispatchers.IO) {
            val bitmap = runCatching { context.contentResolver.loadThumbnail(uri, android.util.Size(640, 640), null) }.getOrNull()
                ?: return@withContext null
            coil3.fetch.ImageFetchResult(bitmap.asImage(), true, coil3.decode.DataSource.DISK)
        }

        class Factory(private val context: Context) : coil3.fetch.Fetcher.Factory<coil3.Uri> {
            override fun create(data: coil3.Uri, options: coil3.request.Options, imageLoader: coil3.ImageLoader): coil3.fetch.Fetcher? {
                val text = data.toString()
                if (!text.startsWith(COVER)) return null
                return CoverFetcher(context, Uri.parse(text.removePrefix(COVER)))
            }
        }
    }
}
