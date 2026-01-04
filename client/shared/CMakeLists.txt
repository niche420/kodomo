cmake_minimum_required(VERSION 3.20)
project(KodomoShared VERSION 0.1.0)

set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

# Find SDL3
find_package(SDL3 CONFIG REQUIRED)

# Find FFmpeg
find_package(PkgConfig REQUIRED)
pkg_check_modules(FFMPEG REQUIRED
    libavcodec>=58
    libavformat>=58
    libavutil>=56
    libswscale>=5
)

# Rust library path
set(RUST_TARGET_DIR ${CMAKE_SOURCE_DIR}/../../server/target)

if(IOS)
    set(RUST_LIB ${RUST_TARGET_DIR}/aarch64-apple-ios/release/libkd_c.a)
elseif(ANDROID)
    if(ANDROID_ABI STREQUAL "arm64-v8a")
        set(RUST_LIB ${RUST_TARGET_DIR}/aarch64-linux-android/release/libkd_c.a)
    elseif(ANDROID_ABI STREQUAL "armeabi-v7a")
        set(RUST_LIB ${RUST_TARGET_DIR}/armv7-linux-androideabi/release/libkd_c.a)
    endif()
elseif(WIN32)
    set(RUST_LIB $<IF:$<CONFIG:Debug>,
        ${RUST_TARGET_DIR}/debug/kd_c.lib,
        ${RUST_TARGET_DIR}/release/kd_c.lib>)
else()
    set(RUST_LIB $<IF:$<CONFIG:Debug>,
        ${RUST_TARGET_DIR}/debug/libkd_c.a,
        ${RUST_TARGET_DIR}/release/libkd_c.a>)
endif()

# Platform-specific libraries
if(WIN32)
    set(PLATFORM_LIBS ws2_32 userenv bcrypt ntdll)
elseif(IOS)
    set(PLATFORM_LIBS
        "-framework Foundation"
        "-framework UIKit"
        "-framework Security"
        "-framework SystemConfiguration"
    )
elseif(ANDROID)
    set(PLATFORM_LIBS log android)
elseif(APPLE)
    set(PLATFORM_LIBS
        "-framework CoreFoundation"
        "-framework Security"
        "-framework SystemConfiguration"
    )
else()
    set(PLATFORM_LIBS pthread dl m)
endif()

# Source files
set(SOURCES
    src/decoder.cpp
    src/renderer.cpp
    src/input_handler.cpp
    src/network_client.cpp
    src/client_core.cpp
    src/connection_manager.cpp
)

set(HEADERS
    include/kodomo.h
    include/decoder.hpp
    include/renderer.hpp
    include/input_handler.hpp
    include/network_client.hpp
    include/client_core.hpp
    include/connection_manager.hpp
)

# Create static library
add_library(kodomo-shared STATIC ${SOURCES} ${HEADERS})

target_include_directories(kodomo-shared PUBLIC
    ${CMAKE_CURRENT_SOURCE_DIR}/include
    ${SDL3_INCLUDE_DIRS}
    ${FFMPEG_INCLUDE_DIRS}
)

target_link_libraries(kodomo-shared PUBLIC
    SDL3::SDL3
    ${FFMPEG_LIBRARIES}
    ${RUST_LIB}
    ${PLATFORM_LIBS}
)

# Export for use by platform projects
set(KODOMO_SHARED_INCLUDE_DIR ${CMAKE_CURRENT_SOURCE_DIR}/include PARENT_SCOPE)
set(KODOMO_SHARED_LIBRARY kodomo-shared PARENT_SCOPE)