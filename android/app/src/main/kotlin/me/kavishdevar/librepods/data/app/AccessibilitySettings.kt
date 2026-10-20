package me.kavishdevar.librepods.data.app

import kotlinx.serialization.Serializable

@Serializable
data class AccessibilitySettings(
    val differentiateWithoutColor: Boolean = false,

    val reduceTransparency: Boolean = false,
    val increaseContrast: Boolean = false,
    val showBorders: Boolean = false,
)
