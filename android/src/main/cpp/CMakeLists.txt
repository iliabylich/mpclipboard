cmake_minimum_required(VERSION 3.22)

project(mpclipboard_android)

set(ROOT_DIR "${CMAKE_CURRENT_SOURCE_DIR}/../../..")
set(GENERIC_CLIENT_LIB "${ROOT_DIR}/libmpclipboard_generic_client.a")

if(NOT EXISTS "${GENERIC_CLIENT_LIB}")
    message(FATAL_ERROR "Prebuilt generic-client library not found: ${GENERIC_CLIENT_LIB}")
endif()

add_library(mpclipboard_generic_client STATIC IMPORTED GLOBAL)
set_target_properties(
    mpclipboard_generic_client
    PROPERTIES IMPORTED_LOCATION "${GENERIC_CLIENT_LIB}"
)

add_library(mpclipboard_android SHARED mpclipboard_android_jni.c)
target_include_directories(mpclipboard_android PRIVATE "${ROOT_DIR}")

target_link_options(
    mpclipboard_android
    PRIVATE
    -Wl,--allow-multiple-definition
    -Wl,--build-id=none
)

target_link_libraries(
    mpclipboard_android
    -Wl,--whole-archive
    mpclipboard_generic_client
    -Wl,--no-whole-archive
    android
    log
)
