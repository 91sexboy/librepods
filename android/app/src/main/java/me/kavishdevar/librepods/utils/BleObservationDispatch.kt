package me.kavishdevar.librepods.utils

internal data class BleObservationDispatch(
    val deliverStatusObservation: Boolean,
    val notifyNewAddress: Boolean,
    val lidStateChange: Boolean?,
)

internal fun planBleObservationDispatch(
    previousLidOpen: Boolean?,
    currentGlobalLidState: Boolean?,
    observedLidOpen: Boolean,
    statusChanged: Boolean,
): BleObservationDispatch {
    val isNewAddress = previousLidOpen == null
    val lidStateChange = when {
        isNewAddress && currentGlobalLidState != observedLidOpen -> observedLidOpen
        !isNewAddress && previousLidOpen != observedLidOpen &&
            currentGlobalLidState != observedLidOpen -> observedLidOpen
        else -> null
    }

    return BleObservationDispatch(
        deliverStatusObservation = !isNewAddress && statusChanged,
        notifyNewAddress = isNewAddress,
        lidStateChange = lidStateChange,
    )
}
