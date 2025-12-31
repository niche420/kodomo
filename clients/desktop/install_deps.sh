#!/bin/bash
set -e

echo "🔧 Installing Dependencies for Streaming Client"
echo ""

# Detect OS
if [[ "$OSTYPE" == "linux-gnu"* ]]; then
    echo "Detected: Linux"
    echo "Installing via apt..."
    
    sudo apt update
    sudo apt install -y \
        cmake \
        build-essential \
        pkg-config \
        libsdl2-dev \
        libavcodec-dev \
        libavformat-dev \
        libavutil-dev \
        libswscale-dev \
        libswresample-dev
    
    echo "✓ Linux dependencies installed"
    
elif [[ "$OSTYPE" == "darwin"* ]]; then
    echo "Detected: macOS"
    echo "Installing via Homebrew..."
    
    if ! command -v brew &> /dev/null; then
        echo "Homebrew not found. Installing..."
        /bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"
    fi
    
    brew install cmake sdl2 ffmpeg pkg-config
    
    echo "✓ macOS dependencies installed"
    
elif [[ "$OSTYPE" == "msys" || "$OSTYPE" == "cygwin" ]]; then
    echo "Detected: Windows"
    echo ""
    echo "Please install manually:"
    echo "1. Visual Studio 2022 with C++ tools"
    echo "2. CMake from https://cmake.org/"
    echo "3. SDL2 from https://libsdl.org/"
    echo "4. FFmpeg from https://ffmpeg.org/"
    echo ""
    echo "Or use vcpkg:"
    echo "  vcpkg install sdl2:x64-windows ffmpeg:x64-windows"
    exit 1
    
else
    echo "Unsupported OS: $OSTYPE"
    exit 1
fi

echo ""
echo "✅ All dependencies installed successfully!"
echo ""
echo "Next steps:"
echo "  1. Build Rust library: cargo build --release -p streaming-ffi"
echo "  2. Build client: ./build.sh"