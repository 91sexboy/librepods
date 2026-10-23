package me.kavishdevar.librepods.data.app

object LibrePodsIntents {
    const val ACTION_CONNECT = "me.kavishdevar.librepods.action.CONNECT"
    const val ACTION_SET_ANC_MODE = "me.kavishdevar.librepods.action.SET_ANC_MODE"

    object Extras {
        const val MAC_ADDRESS = "me.kavishdevar.librepods.extra.MAC_ADDRESS"
        const val ANC_MODE = "me.kavishdevar.librepods.extra.ANC_MODE"
    }
}
