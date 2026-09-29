package dev.ibylich.mpclipboard

import android.os.ParcelFileDescriptor
import android.system.Os
import android.system.OsConstants
import android.util.Log
import java.io.FileDescriptor
import java.io.IOException

object Ffi {
    enum class Connectivity {
        Connecting,
        Connected,
        Disconnected,
        ;

        internal companion object {
            private val CONNECTING = connectivityConnecting()
            private val CONNECTED = connectivityConnected()
            private val DISCONNECTED = connectivityDisconnected()

            fun from(tag: Int): Connectivity {
                return when (tag) {
                    CONNECTING -> Connecting
                    CONNECTED -> Connected
                    DISCONNECTED -> Disconnected
                    else -> fatal("unknown native connectivity tag: $tag")
                }
            }
        }
    }

    data class Output(
        val connectivity: Connectivity?,
        val text: String?,
    ) {
        companion object {
            internal fun from(output: Pair<Int?, ByteArray?>): Output {
                val (tag, bytes) = output
                val connectivity = tag?.let(Connectivity::from)
                val text = bytes?.let { String(it, Charsets.UTF_8) }
                if (connectivity == null && text == null) {
                    fatal("native output contains neither connectivity nor text")
                }
                return Output(connectivity, text)
            }
        }
    }

    init {
        System.loadLibrary("mpclipboard_android")
    }

    private fun fatal(message: String): Nothing {
        Log.e("MPClipboard", message)
        Os.kill(Os.getpid(), OsConstants.SIGABRT)
        error("unreachable")
    }

    class Client private constructor(
        private val handle: Long,
        private val pfd: ParcelFileDescriptor,
    ) {
        val fd: FileDescriptor = pfd.fileDescriptor

        companion object {
            @JvmStatic
            fun new(host: String, token: String, name: String): Client? {
                val handle = mpclipboard_new_inline(
                    host.toByteArray(),
                    token.toByteArray(),
                    name.toByteArray(),
                )
                if (handle == 0L) return null
                val pfd = try {
                    ParcelFileDescriptor.fromFd(mpclipboard_get_fd(handle))
                } catch (e: IOException) {
                    fatal("failed to dup mpclipboard fd: $e")
                }
                return Client(handle, pfd)
            }
        }

        fun read(): Output? {
            return mpclipboard_read(handle)?.let(Output::from)
        }

        fun pushText(text: String) {
            mpclipboard_push_text(handle, text.toByteArray())
        }

        fun close() {
            try {
                pfd.close()
            } catch (e: IOException) {
                fatal("failed to close mpclipboard fd: $e")
            }
            mpclipboard_drop(handle)
        }
    }

    @JvmStatic
    private external fun connectivityConnecting(): Int

    @JvmStatic
    private external fun connectivityConnected(): Int

    @JvmStatic
    private external fun connectivityDisconnected(): Int

    @JvmStatic
    private external fun mpclipboard_new_inline(uri: ByteArray, token: ByteArray, name: ByteArray): Long

    @JvmStatic
    private external fun mpclipboard_drop(clientPtr: Long)

    @JvmStatic
    private external fun mpclipboard_get_fd(clientPtr: Long): Int

    @JvmStatic
    private external fun mpclipboard_read(clientPtr: Long): Pair<Int?, ByteArray?>?

    @JvmStatic
    private external fun mpclipboard_push_text(clientPtr: Long, text: ByteArray)
}
