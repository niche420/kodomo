#pragma once

#include <vector>
#include <cstdint>
#include <memory>

struct DecodedFrame {
    std::vector<uint8_t> data;
    int width;
    int height;
    int stride;
    uint64_t pts;
};

class Decoder {
public:
    Decoder();
    ~Decoder();

    bool initialize();
    std::unique_ptr<DecodedFrame> decode(const std::vector<uint8_t>& packet);

private:
    void* decoder_context_; // FFmpeg AVCodecContext
    bool initialized_;
};