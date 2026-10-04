## Android client

This is the Android client of MPClipboard Android app that is:

1. a wrapper around native `generic-client`
2. provides a settings screen with URL+token+ID text fields, stored in a system account of type `dev.ibylich.mpclipboard`
3. has an API that lets external app (i.e. IME) to embed this app and run as a part of its own process.

Both debug and release builds require these environment variables and are signed
with the configured key (alternatively you can dump them into `.env`):

```text
ANDROID_KEYSTORE_PATH
ANDROID_KEYSTORE_PASSWORD
```

Use `mise build:debug` or `mise build:release` from the `android/` directory.

Additionally, there are:

1. `mise install:{debug,release}` to install APK on the connected device using adb
2. `mise logs` to view logs of the running app
3. `mise fdroid:update` to update F-Droid index (server via GitHub Pages)

### Integration

A client app (i.e. an IME) must load this app's code into its own process and run the connection there, so the connection lives exactly as long as the IME process. This app only manages the config.

#### Requirements

1. The client must be signed with the same certificate.
2. The client's manifest must have:

   ```xml
   <uses-permission android:name="android.permission.INTERNET" />

   <queries>
       <package android:name="dev.ibylich.mpclipboard" />
   </queries>
   ```

#### Loading

```kotlin
val signatures = context.packageManager.checkSignatures(
    context.packageName,
    "dev.ibylich.mpclipboard",
)
check(signatures == PackageManager.SIGNATURE_MATCH)

val packageContext = context.createPackageContext(
    "dev.ibylich.mpclipboard",
    Context.CONTEXT_INCLUDE_CODE or Context.CONTEXT_IGNORE_SECURITY,
)
val entryClass = packageContext.classLoader.loadClass("dev.ibylich.mpclipboard.Embedded")
val embedded = entryClass
    .getConstructor(Context::class.java, Consumer::class.java, Consumer::class.java)
    .newInstance(
        context.applicationContext,
        Consumer<String> { text -> /* text received from other devices */ },
        Consumer<String> { connectivity -> /* "Connecting", "Connected" or "Disconnected" */ },
    )
entryClass.getMethod("start").invoke(embedded)

// and once you have text that you want to share
entryClass.getMethod("sendText", String::class.java).invoke(embedded, "copied text")
```

#### API of `dev.ibylich.mpclipboard.Embedded`

```kotlin
class Embedded(
    context: Context,
    onReceive: Consumer<String>,
    onConnectivityChanged: Consumer<String>,
) {
    fun start()
    fun sendText(text: String)
}
```

All members must be called on the main thread, and both callbacks are invoked on it.
