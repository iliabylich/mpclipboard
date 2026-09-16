package dev.ibylich.mpclipboard

internal object Ffi {
    const val MPCLIPBOARD_CONNECTIVITY_CONNECTING = 0
    const val MPCLIPBOARD_CONNECTIVITY_CONNECTED = 1
    const val MPCLIPBOARD_CONNECTIVITY_DISCONNECTED = 2

    const val MPCLIPBOARD_OUTPUT_CONNECTIVITY_CHANGED = 0
    const val MPCLIPBOARD_OUTPUT_NEW_TEXT = 1
    const val MPCLIPBOARD_OUTPUT_BOTH = 2
    const val MPCLIPBOARD_OUTPUT_IGNORE = 3
    const val MPCLIPBOARD_OUTPUT_ERROR = 4

    const val MPCLIPBOARD_PUSH_RESULT_PUSHED = 0
    const val MPCLIPBOARD_PUSH_RESULT_DROPPED = 1
    const val MPCLIPBOARD_PUSH_RESULT_ERROR = 2

    fun loadLibrary() {
        System.loadLibrary("mpclipboard_android")
    }

    @JvmStatic
    external fun mpclipboard_new_inline(uri: ByteArray, token: ByteArray, name: ByteArray): Long

    @JvmStatic
    external fun mpclipboard_drop(clientPtr: Long)

    @JvmStatic
    external fun mpclipboard_get_fd(clientPtr: Long): Int

    @JvmStatic
    external fun mpclipboard_read(clientPtr: Long): NativeOutput?

    @JvmStatic
    external fun mpclipboard_push_text(clientPtr: Long, text: ByteArray): Int
}
