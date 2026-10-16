package me.kavishdevar.librepods.database.apple

import androidx.room3.Entity
import androidx.room3.PrimaryKey
import me.kavishdevar.librepods.core.data.apple.AppleCache
import me.kavishdevar.librepods.core.devices.AppleMetadata
import me.kavishdevar.librepods.core.devices.AppleSettings
import me.kavishdevar.librepods.core.bluetooth.MacAddress

@Entity
data class AppleEntity(
    @PrimaryKey
    val macAddress: MacAddress,

    val settings: AppleSettings,
    val metadata: AppleMetadata,
    val cache: AppleCache,
)
