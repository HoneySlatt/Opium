package io.github.honeyslatt.music.ui.nav

import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateListOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.compose.ui.graphics.vector.ImageVector
import io.github.honeyslatt.music.core.model.ItemId
import io.github.honeyslatt.music.ui.Icons

enum class Tab(val label: String, val icon: ImageVector) {
    Home("Home", Icons.Home),
    New("New", Icons.Grid),
    Radio("Radio", Icons.Radio),
    Library("Library", Icons.Library),
    Search("Search", Icons.Search),
}

/** A page of a tab's stack. */
sealed interface Route {
    data object Root : Route
    data object Playlists : Route
    data object Albums : Route
    data object Artists : Route
    data object Songs : Route
    data object RecentlyAdded : Route
    data class Album(val id: ItemId) : Route
    /** `known`: the playlist itself, for one not in the library. */
    data class Playlist(val id: ItemId, val known: io.github.honeyslatt.music.core.model.Playlist? = null) : Route
    data class LibraryArtist(val artist: io.github.honeyslatt.music.core.model.Artist) : Route
    data class Artist(val id: ItemId) : Route
}

/**
 * One back stack per tab, as on iOS. Tapping the selected tab pops to its
 * root, or scrolls the root back to the top.
 */
class Navigator(start: Tab = Tab.Library) {
    var tab by mutableStateOf(start)
        private set
    private val stacks = Tab.entries.associateWith { mutableStateListOf<Route>(Route.Root) }

    /** The bottom bar folds while a page scrolls down, as on iOS 26. */
    var folded by mutableStateOf(false)

    /** Bumped when the root should scroll back to its top. */
    var toTop by mutableIntStateOf(0)
        private set

    val stack: List<Route> get() = stacks.getValue(tab)
    val page: Route get() = stack.last()
    val canPop: Boolean get() = stack.size > 1

    fun push(route: Route) {
        stacks.getValue(tab).add(route)
        folded = false
    }

    fun pop(): Boolean {
        val stack = stacks.getValue(tab)
        if (stack.size <= 1) return false
        stack.removeAt(stack.lastIndex)
        folded = false
        return true
    }

    fun select(next: Tab) {
        folded = false
        if (next != tab) {
            tab = next
            return
        }
        val stack = stacks.getValue(tab)
        if (stack.size > 1) {
            stack.removeRange(1, stack.size)
        } else {
            toTop++
        }
    }
}
