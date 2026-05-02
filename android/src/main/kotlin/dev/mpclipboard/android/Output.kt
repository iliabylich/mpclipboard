package dev.mpclipboard.android

sealed interface Output {
    data class ConnectivityChanged(
        val connectivity: Connectivity,
    ) : Output

    data class NewText(
        val text: String,
    ) : Output

    companion object {
        internal fun from(nativeOutput: NativeOutput): Output? {
            return when (nativeOutput.tag) {
                Ffi.MPCLIPBOARD_OUTPUT_CONNECTIVITY_CHANGED -> {
                    ConnectivityChanged(Connectivity.from(nativeOutput.connectivity))
                }
                Ffi.MPCLIPBOARD_OUTPUT_NEW_TEXT -> {
                    val text = nativeOutput.text?.toString(Charsets.UTF_8) ?: return null
                    NewText(text)
                }
                Ffi.MPCLIPBOARD_OUTPUT_IGNORE -> null
                Ffi.MPCLIPBOARD_OUTPUT_ERROR -> throw RuntimeException("mpclipboard_read failed")
                else -> throw RuntimeException("unknown native output tag: ${nativeOutput.tag}")
            }
        }
    }
}
