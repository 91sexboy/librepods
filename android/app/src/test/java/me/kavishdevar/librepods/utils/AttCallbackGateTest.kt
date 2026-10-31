package me.kavishdevar.librepods.utils

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test
import java.util.concurrent.CountDownLatch
import java.util.concurrent.TimeUnit
import java.util.concurrent.atomic.AtomicBoolean
import java.util.concurrent.atomic.AtomicInteger

class AttCallbackGateTest {
    @Test
    fun closeWaitsForInFlightCallbackAndRejectsLaterCallbacks() {
        val gate = AttCallbackGate()
        val callbackStarted = CountDownLatch(1)
        val releaseCallback = CountDownLatch(1)
        val closeStarted = CountDownLatch(1)
        val closeReturned = CountDownLatch(1)
        val callbackAccepted = AtomicBoolean(false)
        val callbackCount = AtomicInteger(0)

        val callbackThread = Thread {
            callbackAccepted.set(gate.runIfOpen {
                callbackStarted.countDown()
                releaseCallback.await()
                callbackCount.incrementAndGet()
            })
        }
        val closeThread = Thread {
            closeStarted.countDown()
            gate.closeAndDrain()
            closeReturned.countDown()
        }

        try {
            callbackThread.start()
            assertTrue(callbackStarted.await(1, TimeUnit.SECONDS))
            closeThread.start()
            assertTrue(closeStarted.await(1, TimeUnit.SECONDS))

            assertFalse(closeReturned.await(50, TimeUnit.MILLISECONDS))
            releaseCallback.countDown()
            assertTrue(closeReturned.await(1, TimeUnit.SECONDS))

            callbackThread.join(1_000)
            closeThread.join(1_000)
            assertFalse(callbackThread.isAlive)
            assertFalse(closeThread.isAlive)
            assertTrue(callbackAccepted.get())
            assertFalse(gate.runIfOpen { callbackCount.incrementAndGet() })
            assertEquals(1, callbackCount.get())
        } finally {
            releaseCallback.countDown()
            callbackThread.join(1_000)
            closeThread.join(1_000)
        }
    }
}
