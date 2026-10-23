package me.kavishdevar.librepods.core.data.apple

enum class AccessoryState {
    Disconnected,
    Idle,
    Music,
    Call,
    Ringing,
    HangingUp,
    Unknown;

    companion object {
        fun fromProximity(byte: Byte): AccessoryState {
            return when (byte.toInt()) {
                0x00 -> Disconnected
                0x04 -> Idle
                0x05 -> Music
                0x06 -> Call
                0x07 -> Ringing
                0x09 -> HangingUp
                else -> Unknown
            }
        }
    }
}
