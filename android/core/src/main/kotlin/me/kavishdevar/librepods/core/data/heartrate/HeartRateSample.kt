package me.kavishdevar.librepods.core.data.heartrate

import kotlin.time.Instant

data class HeartRateSample (
    val bpm: Int,
    val timestamp: Instant
)
