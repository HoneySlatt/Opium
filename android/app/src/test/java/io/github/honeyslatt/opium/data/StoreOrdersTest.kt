package io.github.honeyslatt.opium.data

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNull
import kotlin.test.assertSame

/** What the downloads' store is told of the services, and in which order, while it opens and after. */
class StoreOrdersTest {
    /** A store that writes down what it is told. */
    private class Store {
        val told = mutableListOf<String>()
    }

    private fun newOrders() = StoreOrders<Store, String> { store, service -> store.told += "forget $service" }

    private val orders = newOrders()
    private val store = Store()

    private fun confirm(service: String) = orders.confirming(service).answered { it.told += "confirm $service" }

    private fun signOut(service: String) = orders.signedOut(service)

    @Test
    fun `a sign-out said while the store opens is told once it opens, after a confirmation said before it`() {
        confirm("soundcloud")
        signOut("soundcloud")
        assertEquals(emptyList(), store.told)
        orders.opened(store, notSignedIn = emptyList())
        assertEquals(listOf("confirm soundcloud", "forget soundcloud"), store.told)
    }

    @Test
    fun `a confirmation asked before a sign-out and answered after it is dropped, while the store opens`() {
        val asked = orders.confirming("apple")
        signOut("apple")
        asked.answered { it.told += "confirm apple" }
        orders.opened(store, notSignedIn = emptyList())
        assertEquals(listOf("forget apple"), store.told)
    }

    @Test
    fun `a confirmation asked before a sign-out and answered after it is dropped, once the store is open`() {
        orders.opened(store, notSignedIn = emptyList())
        val asked = orders.confirming("apple")
        signOut("apple")
        asked.answered { it.told += "confirm apple" }
        assertEquals(listOf("forget apple"), store.told)
    }

    @Test
    fun `a confirmation asked after a sign-out counts`() {
        signOut("apple")
        confirm("apple")
        orders.opened(store, notSignedIn = emptyList())
        assertEquals(listOf("forget apple", "confirm apple"), store.told)
    }

    @Test
    fun `a sign-out leaves another service's confirmation be`() {
        val asked = orders.confirming("apple")
        signOut("soundcloud")
        asked.answered { it.told += "confirm apple" }
        orders.opened(store, notSignedIn = emptyList())
        assertEquals(listOf("forget soundcloud", "confirm apple"), store.told)
    }

    @Test
    fun `once the store is open, what is said is told at once`() {
        assertNull(orders.open)
        orders.opened(store, notSignedIn = emptyList())
        assertSame(store, orders.open)
        signOut("navidrome")
        assertEquals(listOf("forget navidrome"), store.told)
        confirm("navidrome")
        assertEquals(listOf("forget navidrome", "confirm navidrome"), store.told)
    }

    @Test
    fun `a store that couldn't open is told nothing`() {
        confirm("soundcloud")
        orders.opened(null, notSignedIn = listOf("apple"))
        signOut("soundcloud")
        assertNull(orders.open)
        assertEquals(emptyList(), store.told)
    }

    @Test
    fun `a sign-out said while the store couldn't open is told it at the next start`() {
        // A start whose store couldn't open: the sign-out said then reaches no store.
        val closed = newOrders()
        closed.opened(null, notSignedIn = emptyList())
        closed.signedOut("apple")
        // The next start: Apple isn't signed in any more, and the store hears it.
        orders.opened(store, notSignedIn = listOf("apple"))
        assertEquals(listOf("forget apple"), store.told)
    }

    @Test
    fun `the services not signed in as the store opens are told so, after what was said meanwhile`() {
        confirm("soundcloud")
        orders.opened(store, notSignedIn = listOf("apple", "navidrome"))
        assertEquals(listOf("confirm soundcloud", "forget apple", "forget navidrome"), store.told)
    }

    @Test
    fun `a confirmation of a service not signed in as the store opens, answered after, is dropped`() {
        val asked = orders.confirming("apple")
        orders.opened(store, notSignedIn = listOf("apple"))
        asked.answered { it.told += "confirm apple" }
        assertEquals(listOf("forget apple"), store.told)
    }
}
