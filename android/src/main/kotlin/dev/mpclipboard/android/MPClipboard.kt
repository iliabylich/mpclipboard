package dev.mpclipboard.android

import android.content.Context

class MPClipboard private constructor(
    private var ptr: Long,
) {
    companion object {
        private val lock = Any()

        @Volatile
        private var didInit = false

        @JvmStatic
        fun init(context: Context): Boolean {
            synchronized(lock) {
                Ffi.loadLibrary(context.applicationContext)
                Ffi.mpclipboard_setup_rustls_on_jvm(context.applicationContext)

                if (didInit) {
                    return true
                }

                didInit = Ffi.mpclipboard_init()
                return didInit
            }
        }

        @JvmStatic
        fun initialize(host: String, token: String, name: String): MPClipboard? {
            check(didInit) { "MPClipboard.init() must be called first" }

            val config = Ffi.mpclipboard_config_new(
                host.toByteArray(Charsets.UTF_8),
                token.toByteArray(Charsets.UTF_8),
                name.toByteArray(Charsets.UTF_8),
            )
            if (config == 0L) {
                return null
            }

            val context = Ffi.mpclipboard_context_new(config)
            if (context == 0L) {
                return null
            }

            val mpclipboard = Ffi.mpclipboard_new(context)
            if (mpclipboard == 0L) {
                return null
            }

            return MPClipboard(mpclipboard)
        }
    }

    fun getFd(): Int {
        return Ffi.mpclipboard_get_fd(ptr)
    }

    fun read(): Output? = Ffi.mpclipboard_read(ptr)?.let(Output::from)

    fun pushText(text: String): Boolean {
        return Ffi.mpclipboard_push_text2(ptr, text.toByteArray(Charsets.UTF_8))
    }
}
