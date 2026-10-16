/*
    LibrePods - AirPods liberated from Apple’s ecosystem
    Copyright (C) 2025 LibrePods contributors

    This program is free software: you can redistribute it and/or modify
    it under the terms of the GNU General Public License as published by
    the Free Software Foundation, either version 3 of the License, or
    any later version.

    This program is distributed in the hope that it will be useful,
    but WITHOUT ANY WARRANTY; without even the implied warranty of
    MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
    GNU General Public License for more details.

    You should have received a copy of the GNU General Public License
    along with this program.  If not, see <https://www.gnu.org/licenses/>.
*/

package me.kavishdevar.librepods.core.data

import kotlinx.serialization.Serializable

// TODO: make DeviceComponent, BatteryComponent, and StemPressBud the same with helpers to parse from byte for specific messages
enum class StemPressBud(val value: Byte) {
    LEFT(0x01), RIGHT(0x02);

    companion object {
        fun fromByte(byte: Byte): StemPressBud? = entries.find { it.value == byte }
    }
}

enum class StemPressType(val value: Byte) {
    SINGLE_PRESS(0x05), DOUBLE_PRESS(0x06), TRIPLE_PRESS(0x07), LONG_PRESS(0x08);

    companion object {
        fun fromByte(byte: Byte): StemPressType? = entries.find { it.value == byte }
    }
}

@Serializable
enum class StemAction {
    PLAY_PAUSE,
    PREVIOUS_TRACK,
    NEXT_TRACK,
    DIGITAL_ASSISTANT,
    CYCLE_NOISE_CONTROL_MODES;
    companion object {
        fun fromString(action: String): StemAction? {
            return entries.find { it.name == action }
        }
        val defaultActions: Map<StemPressType, StemAction> = mapOf(
            StemPressType.SINGLE_PRESS to PLAY_PAUSE,
            StemPressType.DOUBLE_PRESS to NEXT_TRACK,
            StemPressType.TRIPLE_PRESS to PREVIOUS_TRACK,
            StemPressType.LONG_PRESS to CYCLE_NOISE_CONTROL_MODES,
        )
    }
}
