cmake_minimum_required(VERSION 3.22)

project(mpclipboard_android)

if(CMAKE_HOST_WIN32)
    set(USER_HOME "$ENV{USERPROFILE}")
else()
    set(USER_HOME "$ENV{HOME}")
endif()

find_program(CARGO_EXECUTABLE NAMES cargo HINTS "${USER_HOME}/.cargo/bin")
if(NOT CARGO_EXECUTABLE)
    message(FATAL_ERROR "Cargo not found on this system.")
endif()

set(GENERIC_CLIENT_DIR "${CMAKE_CURRENT_SOURCE_DIR}/../../../../generic-client")

add_library(mpclipboard_android SHARED mpclipboard_android_jni.c)
target_include_directories(mpclipboard_android PRIVATE "${GENERIC_CLIENT_DIR}")

if(CMAKE_ANDROID_ARCH_ABI STREQUAL "armeabi-v7a")
    set(ANDROID_TARGET "armv7a-linux-androideabi")
    set(RUST_TARGET "armv7-linux-androideabi")
elseif(CMAKE_ANDROID_ARCH_ABI STREQUAL "arm64-v8a")
    set(ANDROID_TARGET "aarch64-linux-android")
    set(RUST_TARGET "aarch64-linux-android")
elseif(CMAKE_ANDROID_ARCH_ABI STREQUAL "x86")
    set(ANDROID_TARGET "i686-linux-android")
    set(RUST_TARGET "i686-linux-android")
elseif(CMAKE_ANDROID_ARCH_ABI STREQUAL "x86_64")
    set(ANDROID_TARGET "x86_64-linux-android")
    set(RUST_TARGET "x86_64-linux-android")
else()
    message(FATAL_ERROR "Unsupported ABI: ${CMAKE_ANDROID_ARCH_ABI}")
endif()

get_filename_component(LLVM_TOOLCHAIN ${CMAKE_C_COMPILER} DIRECTORY)
set(GENERIC_CLIENT_RUST_PATH "${GENERIC_CLIENT_DIR}/target/${RUST_TARGET}/release/libmpclipboard_generic_client.a")

add_custom_target(
    mpclipboard_generic_client_build ALL
    COMMAND ${CARGO_EXECUTABLE} ndk
        -t ${CMAKE_ANDROID_ARCH_ABI}
        build
        --release
        --locked
    BYPRODUCTS ${GENERIC_CLIENT_RUST_PATH}
    WORKING_DIRECTORY ${GENERIC_CLIENT_DIR}
)

add_dependencies(mpclipboard_android mpclipboard_generic_client_build)

target_link_libraries(
    mpclipboard_android
    android
    log
    -Wl,--whole-archive
    -Wl,--allow-multiple-definition
    -Wl,--build-id=none
    ${GENERIC_CLIENT_RUST_PATH}
)
