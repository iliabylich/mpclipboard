## patched FlorisBoard

This directory contains a set of scripts to clone, patch, and build [FlorisBoard](https://github.com/florisboard/florisboard).

The latest APK file is always attached [to the latest release](https://github.com/iliabylich/mpclipboard/releases/tag/latest).

## building manually

Alternatively if you want to do a realease build yourself first you'll need a self-signed key:

```sh
keytool -genkey -v -keystore release-key.jks -keyalg RSA -keysize 2048 -validity 10000 -alias release
```

Once it's generated **make sure to save it**.

### in Docker

Then to build in Docker you can run this (it will prompt you for the password that you entered previously)

```sh
just florisboard::build-in-docker /path/to/generated/keyfile ./florisboard.apk
```

### locally

Or alternatively you can build it fully locally. For that first create a `.env` file in the root directory of the project:

```
# .env
export ANDROID_KEYSTORE_PATH=/path/to/generated/keyfile
export ANDROID_KEYSTORE_PASSWORD=PasswordThatYouEntered
export ANDROID_KEY_ALIAS=release
export ANDROID_KEY_PASSWORD=PasswordThatYouEntered
```

and then run

```sh
just florisboard::build-locally ./florisboard.apk
```
