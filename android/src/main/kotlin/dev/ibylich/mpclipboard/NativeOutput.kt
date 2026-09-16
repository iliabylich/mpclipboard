package dev.ibylich.mpclipboard

internal data class NativeOutput(
    val tag: Int,
    val connectivity: Int,
    val text: ByteArray?,
)
