cmake_minimum_required(VERSION 3.22.1)
project(KodomoDesktop VERSION 0.1.0)

set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_EXPORT_COMPILE_COMMANDS ON)

find_package(SDL3 CONFIG REQUIRED)
find_package(FFMPEG REQUIRED)
add_subdirectory(../shared ${CMAKE_BINARY_DIR}/shared)

set(DESKTOP_SOURCES
    src/main.cpp
    src/desktop_window.cpp
    src/decoder_ffmpeg.cpp
)

set(DESKTOP_HEADERS
    include/desktop_window.hpp
)

add_executable(kodomo-client ${DESKTOP_SOURCES} ${DESKTOP_HEADERS})

target_include_directories(kodomo-client PRIVATE
    ${CMAKE_CURRENT_SOURCE_DIR}/include
    ${KODOMO_SHARED_INCLUDE_DIR}
    ${FFMPEG_INCLUDE_DIRS}
)

target_link_libraries(kodomo-client PRIVATE
    kodomo-shared
    ${FFMPEG_LIBRARIES}
)

install(TARGETS kodomo-client DESTINATION bin)
