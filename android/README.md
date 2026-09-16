## MPClipboard for Android

This directory contains the standalone MPClipboard Android app. The app owns the
network connection, exposes it to a separately installed clipboard app through
AIDL, provides a settings screen for host/token/name, and includes a home-screen
connectivity widget.

The service component is:

```text
dev.ibylich.mpclipboard/dev.ibylich.mpclipboard.MPClipboardService
```

Clients must request the signature-only permission and be signed with the same
certificate as this app:

```xml
<uses-permission android:name="dev.ibylich.mpclipboard.permission.BIND_SERVICE" />
```

Both debug and release builds require these environment variables and are signed
with the configured key:

```text
ANDROID_KEYSTORE_PATH
ANDROID_KEYSTORE_PASSWORD
ANDROID_KEY_ALIAS
ANDROID_KEY_PASSWORD
```

The native `generic-client` library must be built before Gradle assembles the app.
Use `just android::build-debug` or `just android::build-release` from the repository
root after exporting the signing variables.
