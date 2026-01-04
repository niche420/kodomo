#!/bin/bash
set -e

# Build Rust library for Android

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
PROJECT_ROOT="$SCRIPT_DIR/../../.."

echo "Building Rust library for Android..."

# Ensure cargo-ndk is installed
if ! command -v cargo-ndk &> /dev/null; then
    echo "Installing cargo-ndk..."
    cargo install cargo-ndk
fi

# Build for all supported ABIs
cd "$PROJECT_ROOT"

ABIs=("arm64-v8a" "armeabi-v7a")
OUTPUT_DIR="$SCRIPT_DIR/app/src/main/jniLibs"

for abi in "${ABIs[@]}"; do
    echo "Building for $abi..."
    cargo ndk -t "$abi" -o "$OUTPUT_DIR" build --release -p kd-c
done

echo "✅ Rust libraries built for Android"
echo "Output: $OUTPUT_DIR"
ls -lh "$OUTPUT_DIR"/*/*.so