package me.kavishdevar.librepods.services

internal data class AacpConnectionAttempt(val generation: Long)

internal data class DisconnectedAacpLifecycle<Socket : Any>(
    val pendingSocket: Socket?,
    val activeSocket: Socket?,
) {
    val endedActiveSession: Boolean
        get() = activeSocket != null
}

internal class AacpSessionLifecycle<Socket : Any> {
    private data class Pending<Socket : Any>(
        val attempt: AacpConnectionAttempt,
        var socket: Socket? = null,
    )

    private data class Active<Socket : Any>(
        val attempt: AacpConnectionAttempt,
        val socket: Socket,
    )

    private var generation = 0L
    private var pending: Pending<Socket>? = null
    private var active: Active<Socket>? = null

    @Synchronized
    fun beginAttempt(): AacpConnectionAttempt? {
        if (pending != null || active != null) return null

        val attempt = AacpConnectionAttempt(++generation)
        pending = Pending(attempt)
        return attempt
    }

    @Synchronized
    fun attachSocket(attempt: AacpConnectionAttempt, socket: Socket): Boolean {
        val current = pending ?: return false
        if (current.attempt != attempt || current.socket != null) return false

        current.socket = socket
        return true
    }

    @Synchronized
    fun promote(attempt: AacpConnectionAttempt, socket: Socket): Boolean {
        val current = pending ?: return false
        if (current.attempt != attempt || current.socket !== socket || active != null) return false

        active = Active(attempt, socket)
        pending = null
        return true
    }

    @Synchronized
    fun failAttempt(attempt: AacpConnectionAttempt, socket: Socket? = null): Boolean {
        val current = pending ?: return false
        if (current.attempt != attempt || current.socket !== socket) return false

        pending = null
        return true
    }

    @Synchronized
    fun endSession(attempt: AacpConnectionAttempt, socket: Socket): Boolean {
        val current = active ?: return false
        if (current.attempt != attempt || current.socket !== socket) return false

        active = null
        return true
    }

    @Synchronized
    fun disconnect(): DisconnectedAacpLifecycle<Socket> {
        generation++
        val disconnected = DisconnectedAacpLifecycle(
            pendingSocket = pending?.socket,
            activeSocket = active?.socket,
        )
        pending = null
        active = null
        return disconnected
    }

    @Synchronized
    fun isActive(attempt: AacpConnectionAttempt, socket: Socket): Boolean =
        active?.let { it.attempt == attempt && it.socket === socket } == true

    @Synchronized
    fun activeSocket(): Socket? = active?.socket

    @Synchronized
    fun hasPendingAttempt(): Boolean = pending != null

    @get:Synchronized
    val isConnected: Boolean
        get() = active != null
}
