#!/usr/bin/env bash

set -euo pipefail

cd "$(dirname "$0")"

repo="https://github.com/rustls/rustls-platform-verifier.git"
ref="v/0.7.0"
tmp_dir="$PWD/tmp"
checkout_dir="$tmp_dir/rustls-platform-verifier"
aar_name="rustls-platform-verifier-release.aar"
aar_path="$checkout_dir/android/rustls-platform-verifier/build/outputs/aar/$aar_name"
out_dir="$PWD/build"
out_path="$out_dir/rustls-platform-verifier.aar"

rm -rf "$tmp_dir"
mkdir -p "$tmp_dir" "$out_dir"

git clone --depth 1 --branch "$ref" "$repo" "$checkout_dir"

git -C "$checkout_dir" apply "$PWD/patches/prefer-crls-on-android.patch"

./gradlew \
    -p "$checkout_dir/android" \
    :rustls-platform-verifier:assembleRelease

cp "$aar_path" "$out_path"

echo "$out_path"
