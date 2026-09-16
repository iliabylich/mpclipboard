package dev.ibylich.mpclipboard

enum class PushResult {
    Pushed,
    Dropped,
    ;

    companion object {
        internal fun from(pushResult: Int): PushResult {
            return when (pushResult) {
                Ffi.MPCLIPBOARD_PUSH_RESULT_PUSHED -> Pushed
                Ffi.MPCLIPBOARD_PUSH_RESULT_DROPPED -> Dropped
                Ffi.MPCLIPBOARD_PUSH_RESULT_ERROR -> throw RuntimeException("mpclipboard_push_text failed")
                else -> throw RuntimeException("unknown native push result: $pushResult")
            }
        }
    }
}
