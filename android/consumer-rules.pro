# Constructed from JNI by exact class name and constructor signature.
-keep class dev.mpclipboard.android.NativeOutput {
    <init>(int, int, byte[]);
    <fields>;
    <methods>;
}

# Exported JNI entry points use name-based registration.
-keepclasseswithmembernames class dev.mpclipboard.android.Ffi {
    native <methods>;
}
