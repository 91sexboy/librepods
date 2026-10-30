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
}
