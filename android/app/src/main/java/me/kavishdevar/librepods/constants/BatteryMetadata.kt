/*
 * LibrePods - AirPods liberated from Apple’s ecosystem
 *
 * Copyright (C) 2025 LibrePods contributors
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as published
 * by the Free Software Foundation, either version 3 of the License.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU Affero General Public License for more details.
 *
 * You should have received a copy of the GNU Affero General Public License
 * along with this program. If not, see <https://www.gnu.org/licenses/>.
 */

package me.kavishdevar.librepods.constants

internal enum class BatteryMetadataField {
    CASE_LEVEL,
    CASE_CHARGING,
    LEFT_LEVEL,
    LEFT_CHARGING,
    RIGHT_LEVEL,
    RIGHT_CHARGING,
}

internal data class BatteryMetadataPayload(
    val field: BatteryMetadataField,
    val value: String,
)

internal fun planBatteryMetadata(batteries: List<Battery>): List<BatteryMetadataPayload> {
    val byComponent = batteries.associateBy(Battery::component)
    return listOf(
        BatteryMetadataPayload(
            BatteryMetadataField.CASE_LEVEL,
            byComponent[BatteryComponent.CASE]?.level.toString(),
        ),
        BatteryMetadataPayload(
            BatteryMetadataField.CASE_CHARGING,
            if (byComponent[BatteryComponent.CASE]?.status == BatteryStatus.CHARGING) "1" else "0",
        ),
        BatteryMetadataPayload(
            BatteryMetadataField.LEFT_LEVEL,
            byComponent[BatteryComponent.LEFT]?.level.toString(),
        ),
        BatteryMetadataPayload(
            BatteryMetadataField.LEFT_CHARGING,
            if (byComponent[BatteryComponent.LEFT]?.status == BatteryStatus.CHARGING) "1" else "0",
        ),
        BatteryMetadataPayload(
            BatteryMetadataField.RIGHT_LEVEL,
            byComponent[BatteryComponent.RIGHT]?.level.toString(),
        ),
        BatteryMetadataPayload(
            BatteryMetadataField.RIGHT_CHARGING,
            if (byComponent[BatteryComponent.RIGHT]?.status == BatteryStatus.CHARGING) "1" else "0",
        ),
    )
}
