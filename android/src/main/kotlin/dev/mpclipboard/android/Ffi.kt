package dev.mpclipboard.android

import android.content.Context
import dalvik.system.BaseDexClassLoader

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

    fun loadLibrary(context: Context) {
        try {
            System.loadLibrary("mpclipboard_android")
            return
        } catch (_: UnsatisfiedLinkError) {
        }

        val classLoader = Ffi::class.java.classLoader
        if (classLoader is BaseDexClassLoader) {
            val path = classLoader.findLibrary("mpclipboard_android")
            if (path != null) {
                System.load(path)
                return
            }
        }

        val appInfo = context.packageManager.getApplicationInfo("dev.mpclipboard.android.cli", 0)
        System.load("${appInfo.nativeLibraryDir}/libmpclipboard_android.so")
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
