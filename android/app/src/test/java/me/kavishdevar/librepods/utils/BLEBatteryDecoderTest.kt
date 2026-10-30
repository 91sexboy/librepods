package me.kavishdevar.librepods.utils

import org.junit.Assert.assertEquals
import org.junit.Test

class BLEBatteryDecoderTest {
    @Test
    fun unknownEncryptedLevelFallsBackToPublicReading() {
        val fallback = DecodedBattery(charging = false, level = 60)

        assertEquals(
            fallback,
            decodeEncryptedBattery(byteVal = 0x7F, fallback = fallback),
        )
        assertEquals(
            fallback,
            decodeEncryptedBattery(byteVal = 0xFF, fallback = fallback),
        )
        assertEquals(
            DecodedBattery(charging = true, level = 50),
            decodeEncryptedBattery(byteVal = 0xB2, fallback = fallback),
        )
    }

    @Test
    fun firstUnknownEncryptedLevelPreservesNullFallback() {
        val fallback = DecodedBattery(charging = false, level = null)

        assertEquals(
            fallback,
            decodeEncryptedBattery(byteVal = 0x7F, fallback = fallback),
        )
    }

    @Test
    fun invalidEncryptedCaseUsesLastKnownFallback() {
        val publicFallback = DecodedBattery(charging = false, level = 80)
        val lastKnown = DecodedBattery(charging = true, level = 67)

        assertEquals(
            lastKnown,
            decodeEncryptedBattery(
                byteVal = 0x7F,
                fallback = publicFallback,
                lastKnown = lastKnown,
            ),
        )
    }
}
