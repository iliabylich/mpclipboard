## Android client

This is the Android client of MPClipboard Android app that is:

1. a wrapper around native `generic-client`
2. provides a settings screen with URL+token+ID text fields
3. provides a home screen widget with connectivity status
4. exposes a service at `dev.ibylich.mpclipboard/dev.ibylich.mpclipboard.MPClipboardService` that communicates with the IME app through AIDL

Clients must request the signature-only permission and be signed with the same
certificate as this app:

```xml
<uses-permission android:name="dev.ibylich.mpclipboard.permission.BIND_SERVICE" />
```

Both debug and release builds require these environment variables and are signed
with the configured key (alternatively you can dump them into `.env`):

```text
ANDROID_KEYSTORE_PATH
ANDROID_KEYSTORE_PASSWORD
```

Use `mise build:debug` or `mise build:release` from the `android/` directory.

Additionally, there are:

1. `mise install:{debug,release}` to install APK on the connected device using adb
2. `muse logs` to view logs of the running app
3. `mise fdroid:update` to update F-Droid index (server via GitHub Pages)
