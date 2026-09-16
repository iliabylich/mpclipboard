package dev.ibylich.mpclipboard

class MPClipboard private constructor(
    private var ptr: Long,
) {
    companion object {
        private val lock = Any()

        @Volatile
        private var didInit = false

        @JvmStatic
        fun init(): Boolean {
            synchronized(lock) {
                if (didInit) {
                    return true
                }

                Ffi.loadLibrary()
                didInit = true
                return true
            }
        }

        @JvmStatic
        fun initialize(host: String, token: String, name: String): MPClipboard? {
            check(didInit) { "MPClipboard.init() must be called first" }

            val mpclipboard = Ffi.mpclipboard_new_inline(
                host.toByteArray(Charsets.UTF_8),
                token.toByteArray(Charsets.UTF_8),
                name.toByteArray(Charsets.UTF_8),
            )
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

    fun pushText(text: String): PushResult {
        return PushResult.from(Ffi.mpclipboard_push_text(ptr, text.toByteArray(Charsets.UTF_8)))
    }

    fun close() {
        if (ptr != 0L) {
            Ffi.mpclipboard_drop(ptr)
            ptr = 0L
        }
    }
}
