package dev.mpclipboard.android

enum class Connectivity {
    Connecting,
    Connected,
    Disconnected,
    ;

    internal companion object {
        fun from(tag: Int): Connectivity {
            return when (tag) {
                Ffi.MPCLIPBOARD_CONNECTIVITY_CONNECTING -> Connecting
                Ffi.MPCLIPBOARD_CONNECTIVITY_CONNECTED -> Connected
                else -> Disconnected
            }
        }

        fun valueOfOrNull(value: String): Connectivity? {
            return entries.firstOrNull { it.name == value }
        }
    }
}
