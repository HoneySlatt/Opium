package io.github.honeyslatt.opium.data

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertTrue

/** The desktop's themes beside the user's own (audit finding L12). */
class DesktopThemesTest {
    /** The phone's theme folder: each file's text, by id. */
    private val folder = mutableMapOf<String, String>()

    /** What the phone's settings keep of the desktop's themes, as last saved. */
    private var settings = FromDesktop(ids = emptySet(), places = emptyMap())

    /** Whether the settings save. */
    private var saving = true

    /** What a share did, in order: the settings saved, the files written and taken away. */
    private val done = mutableListOf<String>()

    /** The desktop shares `themes`, all of them. */
    private fun share(vararg themes: Pair<String, String>): FromDesktop =
        shareDesktopThemes(
            themes.toList(),
            held = folder.keys.toSet(),
            before = settings,
            keep = { id, source ->
                done += "keep $id"
                (source != "not a theme").also { if (it) folder[id] = source }
            },
            remove = {
                done += "remove $it"
                folder.remove(it)
            },
            save = {
                done += "save ${it.ids.sorted()}"
                saving.also { saved -> if (saved) settings = it }
            },
        )

    @Test
    fun `a theme of the user's stays through a desktop theme of its id, and through that theme's removal`() {
        folder["dusk"] = "the user's"
        share("dusk" to "the desktop's")
        assertEquals(mapOf("dusk" to "the user's", "dusk-desktop" to "the desktop's"), folder)
        assertEquals(setOf("dusk-desktop"), settings.ids)

        // Shared again: the same id, nothing more.
        share("dusk" to "the desktop's")
        assertEquals(mapOf("dusk" to "the user's", "dusk-desktop" to "the desktop's"), folder)
        assertEquals(setOf("dusk-desktop"), settings.ids)

        share()
        assertEquals(mapOf("dusk" to "the user's"), folder)
        assertEquals(emptySet(), settings.ids)
    }

    @Test
    fun `a desktop theme is replaced and removed as before`() {
        share("dusk" to "first")
        assertEquals(mapOf("dusk" to "first"), folder)
        share("dusk" to "second")
        assertEquals(mapOf("dusk" to "second"), folder)
        assertEquals(setOf("dusk"), settings.ids)
        share()
        assertEquals(emptyMap(), folder)
    }

    @Test
    fun `a desktop theme that no longer keeps is taken away, never a theme of the user's`() {
        folder["mine"] = "the user's"
        share("dusk" to "first")
        share("dusk" to "not a theme", "mine" to "not a theme")
        assertEquals(emptySet(), settings.ids)
        assertEquals(mapOf("mine" to "the user's"), folder)
    }

    @Test
    fun `the desktop's other id skips every id the user holds`() {
        folder["dusk"] = "the user's"
        folder["dusk-desktop"] = "the user's too"
        assertEquals(listOf("dusk-desktop-2"), desktopThemeIds(listOf("dusk"), usersOwn = setOf("dusk", "dusk-desktop")))
        share("dusk" to "the desktop's")
        assertEquals("the desktop's", folder["dusk-desktop-2"])
        assertEquals("the user's too", folder["dusk-desktop"])
    }

    @Test
    fun `a desktop theme whose own id is free keeps it, before another's other id may take it`() {
        assertEquals(
            listOf("dusk-desktop-2", "dusk-desktop"),
            desktopThemeIds(listOf("dusk", "dusk-desktop"), usersOwn = setOf("dusk")),
        )
    }

    @Test
    fun `two desktop themes of the same id each keep one`() {
        assertEquals(listOf("dusk", "dusk-desktop"), desktopThemeIds(listOf("dusk", "dusk"), usersOwn = emptySet()))
    }

    @Test
    fun `two desktop themes of the same id keep their ids from one share to the next`() {
        share("dusk" to "first", "dusk" to "second")
        val folderBefore = folder.toMap()
        share("dusk" to "first", "dusk" to "second")
        assertEquals(mapOf("dusk" to "first", "dusk-desktop" to "second"), folderBefore)
        assertEquals(folderBefore, folder)
    }

    @Test
    fun `with no theme of the user's in the way, every desktop theme keeps its own id`() {
        assertEquals(listOf("a", "b"), desktopThemeIds(listOf("a", "b"), usersOwn = setOf("c")))
        assertEquals(emptyList(), desktopThemeIds(emptyList(), usersOwn = setOf("c")))
    }

    @Test
    fun `a desktop theme keeps its id on the phone while the desktop shares it`() {
        folder["dusk"] = "the user's"
        share("dusk" to "the desktop's")
        assertEquals(mapOf("dusk" to "dusk-desktop"), settings.places)

        // The user deletes their own: the desktop's, perhaps chosen, stays where it is.
        folder.remove("dusk")
        done.clear()
        share("dusk" to "the desktop's")
        assertEquals(mapOf("dusk-desktop" to "the desktop's"), folder)
        assertEquals(setOf("dusk-desktop"), settings.ids)
        assertEquals(mapOf("dusk" to "dusk-desktop"), settings.places)
        assertTrue(done.none { it.startsWith("remove") }, "$done")

        // No longer shared, it goes, and so does its place.
        share()
        assertEquals(emptyMap(), folder)
        assertEquals(emptyMap(), settings.places)
    }

    @Test
    fun `a place the user's own theme took since is never used again`() {
        // The desktop's dusk was at dusk-desktop, then the user imported a theme of that id.
        folder["dusk"] = "the user's"
        folder["dusk-desktop"] = "the user's import"
        settings = FromDesktop(ids = emptySet(), places = mapOf("dusk" to "dusk-desktop"))
        share("dusk" to "the desktop's")
        assertEquals("the user's", folder["dusk"])
        assertEquals("the user's import", folder["dusk-desktop"])
        assertEquals("the desktop's", folder["dusk-desktop-2"])
        assertEquals(mapOf("dusk" to "dusk-desktop-2"), settings.places)
    }

    @Test
    fun `the ids that will hold the desktop's themes are saved before any file is written`() {
        folder["dusk"] = "the user's"
        share("dusk" to "the desktop's", "dawn" to "the desktop's")
        assertEquals("save [dawn, dusk-desktop]", done.first(), "$done")
        assertEquals(listOf("save [dawn, dusk-desktop]", "keep dusk-desktop", "keep dawn", "save [dawn, dusk-desktop]"), done)
    }

    @Test
    fun `if the settings can't be saved first, no theme is written nor taken away`() {
        share("dusk" to "first")
        val before = settings
        saving = false
        done.clear()
        assertEquals(before, share("dawn" to "second"))
        assertEquals(mapOf("dusk" to "first"), folder)
        assertEquals(listOf("save [dawn, dusk]"), done)
        assertEquals(before, settings)
    }
}
