package me.kavishdevar.librepods.core.bluetooth.aacp.packet

import me.kavishdevar.librepods.core.bluetooth.aacp.types.MessageOpcode
import me.kavishdevar.librepods.core.bluetooth.aacp.types.Opcode
import me.kavishdevar.librepods.core.devices.DeviceComponent
import me.kavishdevar.librepods.core.devices.PacketDestination

data class BudRolePacket(
    val primary: DeviceComponent,
    val extra: Int,
    override val payload: ByteArray,
): AACPPacket {
    override val destination: PacketDestination = PacketDestination.HOST

    override val type: AACPPacketType = AACPPacketType.MESSAGE
    override val service: Byte = 0x04

    override val opcode: Opcode = MessageOpcode.BUD_ROLE

    companion object {
        fun parse(
            packet: ByteArray,
        ): BudRolePacket {
            val payload = packet.sliceArray(6 until packet.size)

            return BudRolePacket(
                primary = when (payload[0].toInt()) {
                    0x01 -> DeviceComponent.LEFT
                    0x02 -> DeviceComponent.RIGHT
                    else -> DeviceComponent.HEADSET
                },
                extra = payload[2].toInt(),
                payload = payload
            )
        }
    }
}
