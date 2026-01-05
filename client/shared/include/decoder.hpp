#pragma once

#include <vector>
#include <cstdint>
#include <memory>

// Decoded frame data structure
struct DecodedFrame {
    std::vector<uint8_t> data;  // RGBA pixel data
    int width;
    int height;
    int stride;                  // Bytes per row
    uint64_t pts;                // Presentation timestamp
};

// Abstract decoder interface
class IDecoder {
public:
    virtual ~IDecoder() = default;

    // Initialize the decoder
    virtual bool initialize() = 0;

    // Decode H.264 packet, returns frame if ready (may return nullptr if need more data)
    virtual std::unique_ptr<DecodedFrame> decode(const std::vector<uint8_t>& packet) = 0;

    // Cleanup decoder resources
    virtual void shutdown() = 0;
};

// Factory function to create platform-specific decoder
// Implemented by:
//   - client/desktop/src/decoder_ffmpeg.cpp (Desktop)
//   - client/mobile/android/.../decoder_android.cpp (Android)
std::unique_ptr<IDecoder> create_platform_decoder();