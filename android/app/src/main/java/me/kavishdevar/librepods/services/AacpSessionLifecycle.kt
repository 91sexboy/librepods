/*
 * LibrePods - AirPods liberated from Apple’s ecosystem
 *
 * Copyright (C) 2025 LibrePods Contributors
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

package me.kavishdevar.librepods.services

internal data class AacpConnectionAttempt(val generation: Long)

internal data class DisconnectedAacpLifecycle<Socket : Any>(
    val pendingSocket: Socket?,
    val activeSocket: Socket?,
    val cleanupToken: Long?,
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
    private var cleanupGeneration: Long? = null
    private var pending: Pending<Socket>? = null
    private var active: Active<Socket>? = null

    @Synchronized
    fun beginAttempt(): AacpConnectionAttempt? {
        if (pending != null || active != null || cleanupGeneration != null) return null

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
        cleanupGeneration = attempt.generation
        return true
    }

    @Synchronized
    fun disconnect(): DisconnectedAacpLifecycle<Socket> {
        if (cleanupGeneration != null) {
            return DisconnectedAacpLifecycle(
                pendingSocket = null,
                activeSocket = null,
                cleanupToken = null,
            )
        }

        generation++
        cleanupGeneration = generation
        val disconnected = DisconnectedAacpLifecycle(
            pendingSocket = pending?.socket,
            activeSocket = active?.socket,
            cleanupToken = generation,
        )
        pending = null
        active = null
        return disconnected
    }

    @Synchronized
    fun completeCleanup(cleanupToken: Long?): Boolean {
        if (cleanupToken == null || cleanupGeneration != cleanupToken) return false

        cleanupGeneration = null
        return true
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
