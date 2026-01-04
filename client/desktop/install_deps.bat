@echo off
echo Installing Dependencies for Streaming Client (Windows)
echo.

REM Check if vcpkg is available
where vcpkg >nul 2>nul
if %errorlevel% neq 0 (
    echo vcpkg not found in PATH
    echo.
    echo Please install vcpkg:
    echo   1. git clone https://github.com/microsoft/vcpkg.git
    echo   2. cd vcpkg
    echo   3. bootstrap-vcpkg.bat
    echo   4. Add vcpkg to PATH
    echo.
    echo Then install packages:
    echo   vcpkg install sdl2:x64-windows
    echo   vcpkg install ffmpeg:x64-windows
    echo.
    pause
    exit /b 1
)

echo Installing SDL2 and FFmpeg via vcpkg...
vcpkg install sdl2:x64-windows ffmpeg:x64-windows

if %errorlevel% neq 0 (
    echo Failed to install dependencies
    pause
    exit /b 1
)

echo.
echo All dependencies installed!
echo.
echo Next steps:
echo   1. Build Rust library: cargo build --release -p streaming-ffi
echo   2. Build client: build.bat
echo.
pause