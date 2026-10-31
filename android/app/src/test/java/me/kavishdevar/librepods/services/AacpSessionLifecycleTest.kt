package me.kavishdevar.librepods.services

import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNull
import org.junit.Assert.assertSame
import org.junit.Assert.assertTrue
import org.junit.Test

class AacpSessionLifecycleTest {
    private class FakeSocket

    @Test
    fun duplicateStartIsRejectedWithoutCreatingConnectedState() {
        val lifecycle = AacpSessionLifecycle<FakeSocket>()

        assertNotNull(lifecycle.beginAttempt())

        assertNull(lifecycle.beginAttempt())
        assertTrue(lifecycle.hasPendingAttempt())
        assertFalse(lifecycle.isConnected)
    }

    @Test
    fun pendingSocketDoesNotBecomeConnectedBeforePromotion() {
        val lifecycle = AacpSessionLifecycle<FakeSocket>()
        val attempt = lifecycle.beginAttempt()!!
        val socket = FakeSocket()

        assertTrue(lifecycle.attachSocket(attempt, socket))

        assertFalse(lifecycle.isConnected)
        assertNull(lifecycle.activeSocket())
    }

    @Test
    fun explicitDisconnectCancelsPendingAttemptWithoutEndingSession() {
        val lifecycle = AacpSessionLifecycle<FakeSocket>()
        val attempt = lifecycle.beginAttempt()!!
        val socket = FakeSocket()
        assertTrue(lifecycle.attachSocket(attempt, socket))

        val disconnected = lifecycle.disconnect()

        assertSame(socket, disconnected.pendingSocket)
        assertNull(disconnected.activeSocket)
        assertFalse(disconnected.endedActiveSession)
        assertFalse(lifecycle.promote(attempt, socket))
        assertFalse(lifecycle.isConnected)
        assertNotNull(lifecycle.beginAttempt())
    }

    @Test
    fun oldCreateFailureCannotCancelNewAttempt() {
        val lifecycle = AacpSessionLifecycle<FakeSocket>()
        val oldAttempt = lifecycle.beginAttempt()!!
        lifecycle.disconnect()
        val newAttempt = lifecycle.beginAttempt()!!
        val newSocket = FakeSocket()

        assertFalse(lifecycle.failAttempt(oldAttempt))
        assertTrue(lifecycle.attachSocket(newAttempt, newSocket))
        assertTrue(lifecycle.promote(newAttempt, newSocket))
        assertTrue(lifecycle.isActive(newAttempt, newSocket))
    }

    @Test
    fun oldConnectFailureCannotResetNewSession() {
        val lifecycle = AacpSessionLifecycle<FakeSocket>()
        val oldAttempt = lifecycle.beginAttempt()!!
        val oldSocket = FakeSocket()
        assertTrue(lifecycle.attachSocket(oldAttempt, oldSocket))
        lifecycle.disconnect()

        val newAttempt = lifecycle.beginAttempt()!!
        val newSocket = FakeSocket()
        assertTrue(lifecycle.attachSocket(newAttempt, newSocket))
        assertTrue(lifecycle.promote(newAttempt, newSocket))

        assertFalse(lifecycle.failAttempt(oldAttempt, oldSocket))
        assertTrue(lifecycle.isActive(newAttempt, newSocket))
        assertSame(newSocket, lifecycle.activeSocket())
        assertTrue(lifecycle.isConnected)
    }

    @Test
    fun oldReaderFinallyAndDelayedCallbackCannotReachReplacementSession() {
        val lifecycle = AacpSessionLifecycle<FakeSocket>()
        val oldAttempt = lifecycle.beginAttempt()!!
        val oldSocket = FakeSocket()
        assertTrue(lifecycle.attachSocket(oldAttempt, oldSocket))
        assertTrue(lifecycle.promote(oldAttempt, oldSocket))
        assertSame(oldSocket, lifecycle.disconnect().activeSocket)

        val newAttempt = lifecycle.beginAttempt()!!
        val newSocket = FakeSocket()
        assertTrue(lifecycle.attachSocket(newAttempt, newSocket))
        assertTrue(lifecycle.promote(newAttempt, newSocket))

        assertFalse(lifecycle.isActive(oldAttempt, oldSocket))
        assertFalse(lifecycle.endSession(oldAttempt, oldSocket))
        assertTrue(lifecycle.isActive(newAttempt, newSocket))
        assertSame(newSocket, lifecycle.activeSocket())
    }

    @Test
    fun promotionAndCleanupRequireMatchingTokenAndSocket() {
        val lifecycle = AacpSessionLifecycle<FakeSocket>()
        val attempt = lifecycle.beginAttempt()!!
        val socket = FakeSocket()
        val otherSocket = FakeSocket()
        assertTrue(lifecycle.attachSocket(attempt, socket))

        assertFalse(lifecycle.promote(attempt, otherSocket))
        assertFalse(lifecycle.isConnected)
        assertTrue(lifecycle.promote(attempt, socket))
        assertFalse(lifecycle.endSession(attempt, otherSocket))
        assertTrue(lifecycle.isActive(attempt, socket))
        assertTrue(lifecycle.endSession(attempt, socket))
        assertFalse(lifecycle.isConnected)
    }

    @Test
    fun disconnectReturnsOnlyTheOwnedActiveSocket() {
        val lifecycle = AacpSessionLifecycle<FakeSocket>()
        val attempt = lifecycle.beginAttempt()!!
        val socket = FakeSocket()
        assertTrue(lifecycle.attachSocket(attempt, socket))
        assertTrue(lifecycle.promote(attempt, socket))

        val disconnected = lifecycle.disconnect()

        assertNull(disconnected.pendingSocket)
        assertSame(socket, disconnected.activeSocket)
        assertTrue(disconnected.endedActiveSession)
        assertFalse(lifecycle.isConnected)
    }
}
