cmake_minimum_required(VERSION 3.20)
project(KodomoDesktop VERSION 0.1.0)

set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_EXPORT_COMPILE_COMMANDS ON)

# Build shared library first
add_subdirectory(../shared ${CMAKE_BINARY_DIR}/shared)
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

# Desktop-specific source files
set(DESKTOP_SOURCES
    src/main.cpp
    src/desktop_window.cpp
)

set(DESKTOP_HEADERS
    include/desktop_window.hpp
)

# Create desktop executable
add_executable(kodomo-client ${DESKTOP_SOURCES} ${DESKTOP_HEADERS})

# Include directories
target_include_directories(kodomo-client PRIVATE
    ${CMAKE_CURRENT_SOURCE_DIR}/include
    ${KODOMO_SHARED_INCLUDE_DIR}
)

# Link against shared library
target_link_libraries(kodomo-client PRIVATE
    kodomo-shared
)

# Install
install(TARGETS kodomo-client DESTINATION bin)

# Print summary
message(STATUS "Desktop Client Configuration:")
message(STATUS "  Build Type: ${CMAKE_BUILD_TYPE}")
message(STATUS "  Shared Library: kodomo-shared")
message(STATUS "")