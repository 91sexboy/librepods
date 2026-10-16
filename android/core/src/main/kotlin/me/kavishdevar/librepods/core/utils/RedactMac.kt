package me.kavishdevar.librepods.core.utils

fun String.redactMac(): String {
    val parts = this.split(":")
    if (parts.size != 6) return this
    return "${parts[0]}:${parts[1]}:XX:XX:${parts[4]}:${parts[5]}"
}
