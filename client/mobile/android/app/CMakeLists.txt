cmake_minimum_required(VERSION 3.20)
project(KodomoAndroid)

# Set C++ standard
set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

# Android-specific settings
set(BUILD_MOBILE ON)
set(ANDROID TRUE)

add_subdirectory(../../../shared ${CMAKE_BINARY_DIR}/shared)
if(FFMPEG_FOUND)
    message(STATUS "FFmpeg found:")
    message(STATUS "  libavcodec: ${FFMPEG_libavcodec_VERSION}")
    message(STATUS "  libavformat: ${FFMPEG_libavformat_VERSION}")
    message(STATUS "  libavutil: ${FFMPEG_libavutil_VERSION}")
    message(STATUS "  libswscale: ${FFMPEG_libswscale_VERSION}")

    include_directories(${FFMPEG_INCLUDE_DIRS})
    link_directories(${FFMPEG_LIBRARY_DIRS})
else()
    message(FATAL_ERROR "FFmpeg not found. Install with:\n"
            "  Linux: sudo apt install libavcodec-dev libavformat-dev libavutil-dev libswscale-dev\n"
            "  macOS: brew install ffmpeg")
endif()

# Rust library path
set(RUST_TARGET_DIR ${CMAKE_SOURCE_DIR}/../../server/target)

if(ANDROID_ABI STREQUAL "arm64-v8a")
    set(RUST_LIB ${RUST_TARGET_DIR}/aarch64-linux-android/release/libkd_c.a)
elseif(ANDROID_ABI STREQUAL "armeabi-v7a")
    set(RUST_LIB ${RUST_TARGET_DIR}/armv7-linux-androideabi/release/libkd_c.a)
elseif(ANDROID_ABI STREQUAL "x86_64")
    set(RUST_LIB ${RUST_TARGET_DIR}/x86_64-linux-android/release/libkd_c.a)
elseif(ANDROID_ABI STREQUAL "x86")
    set(RUST_LIB ${RUST_TARGET_DIR}/i686-linux-android/release/libkd_c.a)
endif()

# Android-specific sources
set(ANDROID_SOURCES
    src/main/cpp/android_main.cpp
    src/main/cpp/jni_bridge.cpp
)

# Include directories
include_directories(
    ${CMAKE_SOURCE_DIR}/src/main/cpp
    ${KODOMO_SHARED_INCLUDE_DIR}
)

# Create shared library for Android
add_library(kodomo-android SHARED
    ${ANDROID_SOURCES}
)

# Link libraries
target_link_libraries(kodomo-android
    kodomo-shared
    ${RUST_LIB}
    android
    log
    GLESv2
    EGL
)

# Strip symbols in release
if(CMAKE_BUILD_TYPE STREQUAL "Release")
    add_custom_command(TARGET kodomo-android POST_BUILD
        COMMAND ${CMAKE_STRIP} --strip-unneeded $<TARGET_FILE:kodomo-android>
    )
endif()