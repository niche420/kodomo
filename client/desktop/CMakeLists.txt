cmake_minimum_required(VERSION 3.22.1)
project(KodomoDesktop VERSION 0.1.0)

set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_EXPORT_COMPILE_COMMANDS ON)

# ============================================================================
# Add SDL3 from submodule BEFORE adding shared library
# ============================================================================
message(STATUS "Adding SDL3 from submodule...")

set(SDL_SHARED ON CACHE BOOL "Build SDL3 as shared library" FORCE)
set(SDL_STATIC OFF CACHE BOOL "Don't build static SDL3" FORCE)
set(SDL_TEST OFF CACHE BOOL "Don't build SDL3 tests" FORCE)
set(SDL_EXAMPLES OFF CACHE BOOL "Don't build SDL3 examples" FORCE)

# Add SDL3 from submodule
add_subdirectory(${CMAKE_CURRENT_SOURCE_DIR}/../external/SDL ${CMAKE_BINARY_DIR}/SDL)

message(STATUS "✓ SDL3 added from submodule")

# ============================================================================
# Now add shared library (which expects SDL3::SDL3 to exist)
# ============================================================================
add_subdirectory(../shared ${CMAKE_BINARY_DIR}/shared)

# ============================================================================
# Find FFmpeg for decoder
# ============================================================================
find_package(PkgConfig REQUIRED)
pkg_check_modules(FFMPEG REQUIRED
    libavcodec>=58
    libavformat>=58
    libavutil>=56
    libswscale>=5
)

if(FFMPEG_FOUND)
    message(STATUS "✓ FFmpeg found:")
    message(STATUS "  libavcodec: ${FFMPEG_libavcodec_VERSION}")
    message(STATUS "  libavformat: ${FFMPEG_libavformat_VERSION}")
    message(STATUS "  libavutil: ${FFMPEG_libavutil_VERSION}")
    message(STATUS "  libswscale: ${FFMPEG_libswscale_VERSION}")
else()
    message(FATAL_ERROR "FFmpeg not found. Install with:\n"
                        "  Linux: sudo apt install libavcodec-dev libavformat-dev libavutil-dev libswscale-dev\n"
                        "  macOS: brew install ffmpeg")
endif()

# ============================================================================
# Desktop-specific source files
# ============================================================================
set(DESKTOP_SOURCES
    src/main.cpp
    src/desktop_window.cpp
    src/decoder_ffmpeg.cpp  # FFmpeg decoder implementation
)

set(DESKTOP_HEADERS
    include/desktop_window.hpp
)

# ============================================================================
# Create desktop executable
# ============================================================================
add_executable(kodomo-client ${DESKTOP_SOURCES} ${DESKTOP_HEADERS})

# Include directories
target_include_directories(kodomo-client PRIVATE
    ${CMAKE_CURRENT_SOURCE_DIR}/include
    ${KODOMO_SHARED_INCLUDE_DIR}
    ${FFMPEG_INCLUDE_DIRS}
)

# Link libraries
target_link_libraries(kodomo-client PRIVATE
    kodomo-shared
    ${FFMPEG_LIBRARIES}
)

# Install
install(TARGETS kodomo-client DESTINATION bin)

# ============================================================================
# Print summary
# ============================================================================
message(STATUS "================================================")
message(STATUS "Desktop Client Configuration")
message(STATUS "================================================")
message(STATUS "  Build Type: ${CMAKE_BUILD_TYPE}")
message(STATUS "  SDL3: Built from submodule (client/external/SDL)")
message(STATUS "  FFmpeg: ${FFMPEG_LIBRARIES}")
message(STATUS "  Decoder: FFmpeg (software)")
message(STATUS "================================================")