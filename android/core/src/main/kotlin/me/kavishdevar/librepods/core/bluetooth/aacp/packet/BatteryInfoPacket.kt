package me.kavishdevar.librepods.core.bluetooth.aacp.packet

import android.util.Log
import me.kavishdevar.librepods.core.bluetooth.aacp.types.MessageOpcode
import me.kavishdevar.librepods.core.bluetooth.aacp.types.Opcode
import me.kavishdevar.librepods.core.devices.Battery
import me.kavishdevar.librepods.core.devices.BatteryComponent
import me.kavishdevar.librepods.core.devices.BatterySource
import me.kavishdevar.librepods.core.devices.BatteryStatus
import me.kavishdevar.librepods.core.devices.PacketDestination

private const val TAG = "BatteryInfoPacket"

data class BatteryInfoPacket(
    val batteries: Set<Battery>,
    override val payload: ByteArray,
): AACPPacket {
    override val destination: PacketDestination = PacketDestination.HOST

    override val type: AACPPacketType = AACPPacketType.MESSAGE
    override val service: Byte = 0x04

    override val opcode: Opcode = MessageOpcode.BATTERY_INFO

    companion object {
        fun parse(
            packet: ByteArray,
        ): BatteryInfoPacket {
            val payload = packet.sliceArray(6 until packet.size)

            var offset = 0
            val batteryCount = payload[offset].toInt()
            val batteries = mutableSetOf<Battery>()
            offset += 1
            for (i in 0 until batteryCount) {
                val componentByte = payload[offset]
                val component = BatteryComponent.fromAirPodsByte(componentByte)
                val levelByte = payload[offset + 2]
                val level = levelByte.toInt()
                val statusByte = payload[offset + 3]
                val status = BatteryStatus.fromAirPodsByte(statusByte)

                Log.i(TAG, "parsed battery#$i: component=${component.name}, level=$level, status=${status.name}")

                batteries.add(Battery(component, level, status, BatterySource.AACP))
                offset += 5
            }
            Log.i(TAG, "parsed Battery Info: $batteries")

            return BatteryInfoPacket(batteries, payload)
        }
    }
}
