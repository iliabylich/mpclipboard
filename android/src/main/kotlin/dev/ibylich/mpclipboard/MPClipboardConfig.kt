package dev.ibylich.mpclipboard

data class MPClipboardConfig(
    val host: String = "",
    val token: String = "",
    val name: String = "",
) {
    val isComplete: Boolean
        get() = host.isNotBlank() && token.isNotBlank() && name.isNotBlank()
}
