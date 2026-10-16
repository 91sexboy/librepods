package me.kavishdevar.librepods.core.bluetooth.aacp.packet

import me.kavishdevar.librepods.core.bluetooth.aacp.types.MessageOpcode
import me.kavishdevar.librepods.core.data.StemPressBud
import me.kavishdevar.librepods.core.data.StemPressType
import me.kavishdevar.librepods.core.devices.PacketDestination

data class StemPressPacket(
    val stemPressBud: StemPressBud,
    val stemPressType: StemPressType,
    override val payload: ByteArray,
): AACPPacket {
    override val destination: PacketDestination = PacketDestination.HOST

    override val type: AACPPacketType = AACPPacketType.MESSAGE
    override val service: Byte = 0x04

    override val opcode = MessageOpcode.STEM_PRESS

    companion object {
        fun parse(
            packet: ByteArray,
        ): StemPressPacket {
            if (packet.size != 8) {
                throw IllegalArgumentException("Data array too short to parse Stem Press Response")
            }

            val payload = packet.copyOfRange(6, packet.size)

            val bud = StemPressBud.fromByte(payload[1])?: throw IllegalArgumentException("Invalid bud value: ${payload[1]}")
            val type = StemPressType.fromByte(payload[0])?: throw IllegalArgumentException("Invalid type value: ${payload[0]}")

            return StemPressPacket(bud, type, payload)
        }
    }
}
