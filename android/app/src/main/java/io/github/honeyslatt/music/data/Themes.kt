package io.github.honeyslatt.music.data

import android.content.Context
import android.net.Uri
import android.provider.OpenableColumns
import io.github.honeyslatt.music.core.builtInThemes
import io.github.honeyslatt.music.core.model.ThemeFile
import io.github.honeyslatt.music.core.parseTheme
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import java.io.File

/**
 * The themes to choose from: Stock and Spotify, then the files imported
 * from Settings or sent by the desktop, kept in the app's files under
 * `themes/`. The same files as the desktop's, read by the core.
 */
class Themes(context: Context) {
    /** A theme to choose: its id (a file's stem), its name, and the theme or
     *  what is wrong with its file. */
    data class Entry(val id: String, val name: String, val file: ThemeFile?, val problem: String?, val builtIn: Boolean)

    private val resolver = context.contentResolver
    private val dir = File(context.filesDir, "themes")
    private val prefs = context.getSharedPreferences("music", Context.MODE_PRIVATE)
    private val builtIn = builtInThemes().map { Entry(it.id, it.file.name, it.file, null, true) }

    private val _all = MutableStateFlow(read())
    val all: StateFlow<List<Entry>> = _all.asStateFlow()

    private val _chosen = MutableStateFlow(prefs.getString("theme", null) ?: "stock")
    val chosen: StateFlow<String> = _chosen.asStateFlow()

    /** The theme in use: the one chosen, else Stock if it has gone or broken. */
    fun current(all: List<Entry>, chosen: String): ThemeFile =
        all.firstOrNull { it.id == chosen }?.file ?: builtIn.first().file!!

    fun choose(id: String) {
        _chosen.value = id
        prefs.edit().putString("theme", id).apply()
    }

    private fun read(): List<Entry> {
        val own = dir.listFiles { f -> f.extension == "toml" }.orEmpty()
            .filter { f -> builtIn.none { it.id == f.nameWithoutExtension } }
            .map { f ->
                val id = f.nameWithoutExtension
                runCatching { parseTheme(f.readText()) }.fold(
                    { Entry(id, it.name.ifEmpty { id }, it, null, false) },
                    { Entry(id, id, null, it.message ?: "Couldn’t read it", false) },
                )
            }
            .sortedBy { it.name.lowercase() }
        return builtIn + own
    }

    /** Keep a theme's file under `id`, replacing one of that id; what is
     *  wrong with it, if it isn't a theme. */
    fun keep(id: String, source: String, refresh: Boolean = true): String? {
        val safe = idOf(id)
        if (builtIn.any { it.id == safe }) return "“$safe” is a built-in theme’s name."
        val file = File(dir, "$safe.toml")
        // The same as kept: nothing to write, nor to read again.
        if (file.exists() && runCatching { file.readText() }.getOrNull() == source) return null
        runCatching { parseTheme(source) }.onFailure { return it.message ?: "It isn’t a theme." }
        dir.mkdirs()
        file.writeText(source)
        if (refresh) _all.value = read()
        return null
    }

    /** Import the theme file the user picked; what is wrong with it, if any.
     *  Chosen once kept. */
    fun import(uri: Uri): String? {
        val source = runCatching { resolver.openInputStream(uri)?.use { it.readBytes().decodeToString() } }.getOrNull()
            ?: return "Couldn’t read the file."
        val name = resolver.query(uri, arrayOf(OpenableColumns.DISPLAY_NAME), null, null, null)?.use { c ->
            if (c.moveToFirst()) c.getString(0) else null
        } ?: "theme.toml"
        val id = idOf(name.substringBeforeLast('.'))
        return keep(id, source) ?: run {
            choose(id)
            null
        }
    }

    /** A file's stem as an id: lower case, without what a file name can't hold. */
    private fun idOf(name: String) =
        name.lowercase().replace(Regex("[^a-z0-9._-]+"), "-").trim('-').ifEmpty { "theme" }

    /** The desktop's themes, all of them: kept, replacing those of the same
     *  id; those it sent before and no longer has, taken away. */
    fun shared(themes: List<io.github.honeyslatt.music.core.connect.SharedTheme>) {
        val before = prefs.getStringSet("desktop-themes", emptySet()).orEmpty()
        // Read again once at the end, not once for each theme.
        val now = themes.filter { keep(it.id, it.source, refresh = false) == null }.map { idOf(it.id) }.toSet()
        (before - now).forEach { remove(it) }
        prefs.edit().putStringSet("desktop-themes", now).apply()
        val read = read()
        if (read != _all.value) _all.value = read
    }

    /** Take a theme away; Stock again if it was in use. */
    fun remove(id: String) {
        File(dir, "$id.toml").delete()
        _all.value = read()
        if (_chosen.value == id) choose("stock")
    }
}
