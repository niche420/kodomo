cmake_minimum_required(VERSION 4.1.2)

set(CMAKE_ANDROID_STL_TYPE c++_shared)

project(KodomoAndroid)

set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_POSITION_INDEPENDENT_CODE ON)

# ============================================================================
# Add SDL3 from submodule BEFORE adding shared library
# ============================================================================
message(STATUS "Adding SDL3 from submodule...")

# Enable 16KB page size support (required for Android 15+)
set(CMAKE_EXE_LINKER_FLAGS "${CMAKE_EXE_LINKER_FLAGS} -Wl,-z,max-page-size=16384")
set(CMAKE_SHARED_LINKER_FLAGS "${CMAKE_SHARED_LINKER_FLAGS} -Wl,-z,max-page-size=16384")

set(SDL_SHARED ON CACHE BOOL "Build SDL3 as shared library" FORCE)
set(SDL_STATIC OFF CACHE BOOL "Don't build static SDL3" FORCE)
set(SDL_TEST OFF CACHE BOOL "Don't build SDL3 tests" FORCE)
set(SDL_EXAMPLES OFF CACHE BOOL "Don't build SDL3 examples" FORCE)

# Add SDL3 from submodule (adjust path to go up from app/ to client/)
add_subdirectory(${CMAKE_CURRENT_SOURCE_DIR}/../../../external/SDL ${CMAKE_BINARY_DIR}/SDL)

message(STATUS "✓ SDL3 added from submodule")

# ============================================================================
# Now add shared library (which expects SDL3::SDL3 to exist)
# ============================================================================
add_subdirectory(../../../shared ${CMAKE_BINARY_DIR}/shared)

# ============================================================================
# Android-specific sources
# ============================================================================
set(ANDROID_SOURCES
        src/main/cpp/android_main.cpp
        src/main/cpp/jni_bridge.cpp
        src/main/cpp/decoder_android.cpp  # MediaCodec decoder implementation
)

# Include directories
include_directories(
        ${CMAKE_SOURCE_DIR}/src/main/cpp
        ${KODOMO_SHARED_INCLUDE_DIR}
)

# ============================================================================
# Create shared library for Android
# ============================================================================
add_library(kodomo-android SHARED
        ${ANDROID_SOURCES}
)

target_compile_definitions(kodomo-android PRIVATE __ANDROID__)

# Link libraries
target_link_libraries(kodomo-android
        kodomo-shared
        mediandk  # MediaCodec for hardware video decoding
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
