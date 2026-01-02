cmake_minimum_required(VERSION 3.20)
project(StreamingClient VERSION 0.1.0)

set(CMAKE_CXX_STANDARD 23)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_EXPORT_COMPILE_COMMANDS ON)

# Platform detection
if(WIN32)
    set(PLATFORM_NAME "windows")
elseif(APPLE)
    set(PLATFORM_NAME "macos")
else()
    set(PLATFORM_NAME "linux")
endif()

message(STATUS "Building for platform: ${PLATFORM_NAME}")
message(STATUS "Build type: ${CMAKE_BUILD_TYPE}")

# Find SDL3
find_package(SDL3 CONFIG REQUIRED)
include_directories(${SDL3_INCLUDE_DIRS})

# Find FFmpeg using pkg-config
find_package(PkgConfig REQUIRED)
pkg_check_modules(FFMPEG REQUIRED
    libavcodec>=58
    libavformat>=58
    libavutil>=56
    libswscale>=5
)

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

# Include directories
include_directories(
    ${CMAKE_SOURCE_DIR}/include
    ${CMAKE_SOURCE_DIR}/src
)

# Rust library path
set(RUST_TARGET_DIR ${CMAKE_SOURCE_DIR}/../../target)

if(WIN32)
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
    src/main.cpp
    src/client.cpp
    src/decoder.cpp
    src/renderer.cpp
    src/input_handler.cpp
    src/network_client.cpp
)

# Header files
set(HEADERS
    src/client.hpp
    src/decoder.hpp
    src/renderer.hpp
    src/input_handler.hpp
    src/network_client.hpp
    include/kodomo.h
)

# Create executable
add_executable(streaming-client ${SOURCES} ${HEADERS})

# Compiler flags
if(MSVC)
    target_compile_options(streaming-client PRIVATE /W4)
else()
    target_compile_options(streaming-client PRIVATE -Wall -Wextra -pedantic)
endif()

# Link libraries
target_link_libraries(streaming-client
    SDL3::SDL3
    ${RUST_LIB}
    ${FFMPEG_LIBRARIES}
    ${PLATFORM_LIBS}
)

# Copy SDL3 DLL on Windows
if(WIN32)
    add_custom_command(TARGET streaming-client POST_BUILD
        COMMAND ${CMAKE_COMMAND} -E copy_if_different
        $<TARGET_FILE:SDL3::SDL3>
        $<TARGET_FILE_DIR:streaming-client>
    )
endif()

# Install targets
install(TARGETS streaming-client DESTINATION bin)

# Print summary
message(STATUS "")
message(STATUS "Configuration Summary:")
message(STATUS "  Platform: ${PLATFORM_NAME}")
message(STATUS "  C++ Standard: ${CMAKE_CXX_STANDARD}")
message(STATUS "  Build Type: ${CMAKE_BUILD_TYPE}")
message(STATUS "  SDL3: ${SDL3_VERSION}")
message(STATUS "  FFmpeg: ${FFMPEG_VERSION}")
message(STATUS "  Rust Library: ${RUST_LIB}")
message(STATUS "")