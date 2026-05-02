package dev.mpclipboard.android

import android.content.Context
import dalvik.system.BaseDexClassLoader

internal object Ffi {
    const val MPCLIPBOARD_CONNECTIVITY_CONNECTING = 0
    const val MPCLIPBOARD_CONNECTIVITY_CONNECTED = 1
    const val MPCLIPBOARD_CONNECTIVITY_DISCONNECTED = 2

    const val MPCLIPBOARD_OUTPUT_CONNECTIVITY_CHANGED = 0
    const val MPCLIPBOARD_OUTPUT_NEW_TEXT = 1
    const val MPCLIPBOARD_OUTPUT_IGNORE = 2
    const val MPCLIPBOARD_OUTPUT_ERROR = 3

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
    external fun mpclipboard_init(): Boolean

    @JvmStatic
    external fun mpclipboard_setup_rustls_on_jvm(context: Context)

    @JvmStatic
    external fun mpclipboard_config_new(uri: ByteArray, token: ByteArray, name: ByteArray): Long

    @JvmStatic
    external fun mpclipboard_context_new(configPtr: Long): Long

    @JvmStatic
    external fun mpclipboard_new(contextPtr: Long): Long

    @JvmStatic
    external fun mpclipboard_get_fd(clientPtr: Long): Int

    @JvmStatic
    external fun mpclipboard_read(clientPtr: Long): NativeOutput?

    @JvmStatic
    external fun mpclipboard_push_text2(clientPtr: Long, text: ByteArray): Boolean
}
