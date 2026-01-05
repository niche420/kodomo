cmake_minimum_required(VERSION 3.22.1)
project(KodomoShared VERSION 0.1.0)

set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

# SDL3 is added by the parent project (desktop or android)
# We just use the SDL3::SDL3 target that's already defined
if(NOT TARGET SDL3::SDL3)
    message(FATAL_ERROR "SDL3::SDL3 target not found. Parent project must add SDL3 first.")
endif()

message(STATUS "Shared library using SDL3::SDL3 target")

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

# ONLY platform-agnostic source files
set(SOURCES
    src/renderer.cpp
    src/input_handler.cpp
    src/network_client.cpp
    src/client_core.cpp
    src/connection_manager.cpp
)

# ONLY platform-agnostic headers
set(HEADERS
    include/decoder.hpp              # Interface only, no implementation
    include/renderer.hpp
    include/input_handler.hpp
    include/network_client.hpp
    include/client_core.hpp
    include/connection_manager.hpp
)

# Create static library (NO decoder implementation)
add_library(kodomo-shared STATIC ${SOURCES} ${HEADERS})

target_include_directories(kodomo-shared PUBLIC
    ${CMAKE_CURRENT_SOURCE_DIR}/include
)

target_link_libraries(kodomo-shared PUBLIC
    SDL3::SDL3
    ${PLATFORM_LIBS}
)

# Export for use by platform projects
set(KODOMO_SHARED_INCLUDE_DIR ${CMAKE_CURRENT_SOURCE_DIR}/include PARENT_SCOPE)

# Print configuration
message(STATUS "================================================")
message(STATUS "Kodomo Shared Library")
message(STATUS "================================================")
message(STATUS "  Platform: ${CMAKE_SYSTEM_NAME}")
if(ANDROID)
    message(STATUS "  Android ABI: ${ANDROID_ABI}")
endif()
message(STATUS "  SDL3: Using SDL3::SDL3 target")
message(STATUS "  Decoder: Provided by platform project")
message(STATUS "================================================")