FROM docker.io/library/eclipse-temurin:17-jdk-noble AS builder

ENV ANDROID_HOME=/opt/android-sdk
ENV ANDROID_SDK_ROOT=/opt/android-sdk
ENV ANDROID_NDK_VERSION=26.1.10909125
ENV ANDROID_NDK_HOME=${ANDROID_HOME}/ndk/${ANDROID_NDK_VERSION}
ENV NDK_HOME=${ANDROID_NDK_HOME}

ENV PATH="${PATH}:${ANDROID_HOME}/cmdline-tools/latest/bin:${ANDROID_HOME}/platform-tools:${ANDROID_NDK_HOME}/toolchains/llvm/prebuilt/linux-x86_64/bin"

USER root

RUN apt update
RUN apt install -y ca-certificates curl unzip git pkg-config build-essential just libssl-dev

RUN mkdir -p ${ANDROID_HOME}/cmdline-tools
RUN curl -fsSL https://dl.google.com/android/repository/commandlinetools-linux-13114758_latest.zip -o /tmp/android-commandlinetools.zip
RUN unzip -q /tmp/android-commandlinetools.zip -d ${ANDROID_HOME}/cmdline-tools
RUN mv ${ANDROID_HOME}/cmdline-tools/cmdline-tools ${ANDROID_HOME}/cmdline-tools/latest
RUN yes | sdkmanager --licenses
RUN sdkmanager \
    "platform-tools" \
    "platforms;android-33" \
    "platforms;android-35" \
    "platforms;android-36" \
    "build-tools;30.0.3" \
    "build-tools;35.0.0" \
    "ndk;${ANDROID_NDK_VERSION}" \
    "ndk;27.0.12077973" \
    "cmake;3.22.1" \
    "cmake;4.0.2"
RUN rm -rf /tmp/android-commandlinetools.zip

ENV RUSTUP_HOME=/usr/local/rustup
ENV CARGO_HOME=/usr/local/cargo
ENV PATH=/usr/local/cargo/bin:$PATH

RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal --default-toolchain stable
RUN rustc --version
RUN cargo --version

WORKDIR /usr/bin
RUN curl -L https://github.com/cargo-bins/cargo-binstall/releases/latest/download/cargo-binstall-x86_64-unknown-linux-gnu.tgz | tar -xvzf -

RUN rustup target add aarch64-linux-android
RUN cargo binstall cargo-ndk

RUN git config --global advice.detachedHead false

###

FROM builder AS rustls-platform-verifier

WORKDIR /build
COPY rustls-platform-verifier/gradle   ./gradle
COPY rustls-platform-verifier/patches  ./patches
COPY rustls-platform-verifier/gradlew  ./
COPY rustls-platform-verifier/Justfile ./

RUN just release /rustls-platform-verifier.aar
RUN ls -l /rustls-platform-verifier.aar

###

FROM builder AS generic-client-builder

WORKDIR /build/clip
COPY clip/Cargo.toml ./
COPY clip/README.md  ./
COPY clip/src        ./src

WORKDIR /build/generic-client
COPY generic-client/src        ./src
COPY generic-client/Cargo.toml ./
COPY generic-client/README.md  ./
COPY generic-client/Justfile   ./

RUN just release-android /libmpclipboard_generic_client.a
RUN ls -l /libmpclipboard_generic_client.a

###

FROM builder AS mpclipboard-android-builder

WORKDIR /build/android
COPY android/cli                 ./cli
COPY android/gradle              ./gradle
COPY android/src                 ./src
COPY android/build.gradle.kts    ./
COPY android/consumer-rules.pro  ./
COPY android/gradle.properties   ./
COPY android/gradlew             ./
COPY android/Justfile            ./
COPY android/settings.gradle.kts ./

COPY generic-client/bindings.h ./
COPY --from=generic-client-builder   /libmpclipboard_generic_client.a ./
COPY --from=rustls-platform-verifier /rustls-platform-verifier.aar    ./

RUN just --unstable release /mpclipboard.aar
RUN ls -l /mpclipboard.aar

###

FROM builder AS florisboard-builder

WORKDIR /build/florisboard

COPY florisboard/integrate-mpclipboard.patch ./
COPY florisboard/Justfile                    ./

COPY --from=mpclipboard-android-builder /mpclipboard.aar                 ./
COPY --from=rustls-platform-verifier    /rustls-platform-verifier.aar    ./
