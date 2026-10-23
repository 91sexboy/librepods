package me.kavishdevar.librepods.core.data.apple

enum class DeviceColor {
    White,
    Black,
    Red,
    Blue,
    Pink,
    Gray,
    Silver,
    Gold,
    RoseGold,
    SpaceGray,
    DarkBlue,
    LightBlue,
    Yellow,
    Unknown;

    companion object {
        fun fromProximity(byte: Byte): DeviceColor {
            return when (byte.toInt()) {
                0x00 -> White
                0x01 -> Black
                0x02 -> Red
                0x03 -> Blue
                0x04 -> Pink
                0x05 -> Gray
                0x06 -> Silver
                0x07 -> Gold
                0x08 -> RoseGold
                0x09 -> SpaceGray
                0x0A -> DarkBlue
                0x0B -> LightBlue
                0x0C -> Yellow
                else -> Unknown
            }
        }
    }
}
