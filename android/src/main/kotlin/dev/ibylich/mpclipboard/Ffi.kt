package dev.ibylich.mpclipboard

import java.io.FileDescriptor

object Ffi {
    enum class Connectivity {
        Connecting,
        Connected,
        Disconnected,
        ;

        internal companion object {
            private var CONNECTING = 0
            private var CONNECTED = 0
            private var DISCONNECTED = 0

            fun from(tag: Int): Connectivity {
                return when (tag) {
                    CONNECTING -> Connecting
                    CONNECTED -> Connected
                    DISCONNECTED -> Disconnected
                    else -> fatal("unknown native connectivity tag: $tag")
                }
            }

            fun valueOfOrNull(value: String): Connectivity? {
                return entries.firstOrNull { it.name == value }
            }
        }
    }

    data class Output(
        val connectivity: Connectivity?,
        val text: String?,
    ) {
        companion object {
            internal fun from(output: Pair<Int?, String?>): Output {
                val connectivity = output.first?.let(Connectivity::from)
                val text = output.second
                if (connectivity == null && text == null) {
                    fatal("native output contains neither connectivity nor text")
                }
                return Output(connectivity, text)
            }
        }
    }

    enum class PushResult {
        Pushed,
        Dropped,

        ;

        companion object {
            private var PUSHED = 0
            private var DROPPED = 0

            internal fun from(tag: Int): PushResult {
                return when (tag) {
                    PUSHED -> Pushed
                    DROPPED -> Dropped
                    else -> fatal("unknown native push result: $tag")
                }
            }
        }
    }

    init {
        System.loadLibrary("mpclipboard_android")
        mpclipboard_define_enums()
    }

    private fun fatal(message: String): Nothing {
        mpclipboard_fatal(message)
        error("mpclipboard_fatal unexpectedly returned")
    }

    class Client private constructor(
        private val handle: Long,
        val fd: FileDescriptor,
    ) {
        companion object {
            @JvmStatic
            fun new(host: String, token: String, name: String): Client? {
                val handle = mpclipboard_new_inline(
                    host,
                    token,
                    name,
                )
                if (handle == 0L) return null
                val fileDescriptor = FileDescriptor()
                val setInt =
                    FileDescriptor::class.java.getDeclaredMethod(
                        "setInt$",
                        Int::class.javaPrimitiveType,
                    )
                setInt.isAccessible = true
                setInt.invoke(fileDescriptor, mpclipboard_get_fd(handle))
                return Client(handle, fileDescriptor)
            }
        }

        fun read(): Output? {
            return mpclipboard_read(handle)?.let(Output::from)
        }

        fun pushText(text: String): PushResult {
            return PushResult.from(mpclipboard_push_text(handle, text))
        }

        fun close() {
            mpclipboard_drop(handle)
        }
    }

    @JvmStatic
    private external fun mpclipboard_fatal(message: String)

    @JvmStatic
    private external fun mpclipboard_define_enums()

    @JvmStatic
    private external fun mpclipboard_new_inline(uri: String, token: String, name: String): Long

    @JvmStatic
    private external fun mpclipboard_drop(clientPtr: Long)

    @JvmStatic
    private external fun mpclipboard_get_fd(clientPtr: Long): Int

    @JvmStatic
    private external fun mpclipboard_read(clientPtr: Long): Pair<Int?, String?>?

    @JvmStatic
    private external fun mpclipboard_push_text(clientPtr: Long, text: String): Int
}
