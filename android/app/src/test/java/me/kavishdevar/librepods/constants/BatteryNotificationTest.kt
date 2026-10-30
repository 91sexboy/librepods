package me.kavishdevar.librepods.constants

import org.junit.Assert.assertEquals
import org.junit.Test

class BatteryNotificationTest {
    private fun packet(
        leftLevel: Int,
        leftStatus: Int,
        rightLevel: Int,
        rightStatus: Int,
        caseLevel: Int,
        caseStatus: Int,
    ): ByteArray = byteArrayOf(
        0x04, 0x00, 0x04, 0x00, 0x04, 0x00, 0x03,
        BatteryComponent.LEFT.toByte(), 0x01, leftLevel.toByte(), leftStatus.toByte(), 0x01,
        BatteryComponent.RIGHT.toByte(), 0x01, rightLevel.toByte(), rightStatus.toByte(), 0x01,
        BatteryComponent.CASE.toByte(), 0x01, caseLevel.toByte(), caseStatus.toByte(), 0x01,
    )

    @Test
    fun unknownLevelsDoNotReplaceLastKnownBatteryReadings() {
        val notification = AirPodsNotifications.BatteryNotification()
        notification.setBattery(
            packet(
                leftLevel = 58,
                leftStatus = BatteryStatus.NOT_CHARGING,
                rightLevel = 77,
                rightStatus = BatteryStatus.NOT_CHARGING,
                caseLevel = 86,
                caseStatus = BatteryStatus.NOT_CHARGING,
            )
        )

        notification.setBattery(
            packet(
                leftLevel = 57,
                leftStatus = BatteryStatus.NOT_CHARGING,
                rightLevel = 127,
                rightStatus = BatteryStatus.NOT_CHARGING,
                caseLevel = 0,
                caseStatus = BatteryStatus.DISCONNECTED,
            )
        )

        val readings = notification.getBattery().associateBy { it.component }
        assertEquals(57, readings.getValue(BatteryComponent.LEFT).level)
        assertEquals(77, readings.getValue(BatteryComponent.RIGHT).level)
        assertEquals(86, readings.getValue(BatteryComponent.CASE).level)
    }
}
