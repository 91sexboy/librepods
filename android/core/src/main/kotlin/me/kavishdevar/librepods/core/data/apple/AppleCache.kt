package me.kavishdevar.librepods.core.data.apple

import kotlinx.serialization.Serializable
import me.kavishdevar.librepods.core.bluetooth.aacp.types.CapabilityEntry
import me.kavishdevar.librepods.core.bluetooth.aacp.types.ControlCommandIdentifier
import me.kavishdevar.librepods.core.bluetooth.aacp.types.MagicKeyType

@Serializable
data class AppleCache(
    val capabilities: Set<CapabilityEntry> = emptySet(),
    val magicKeys: Map<MagicKeyType, ByteArray> = emptyMap(),
    val controlStates: Map<ControlCommandIdentifier, ByteArray> = emptyMap(),
)
