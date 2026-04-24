package dev.mpclipboard.android

internal data class NativeOutput(
    val tag: Int,
    val connectivity: Int,
    val text: ByteArray?,
)
