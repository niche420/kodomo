#include "decoder.hpp"
#include <media/NdkMediaCodec.h>
#include <media/NdkMediaFormat.h>
#include <android/log.h>
#include <cstring>
#include <android/api-level.h>

#define LOG_TAG "AndroidDecoder"
#define LOGI(...) __android_log_print(ANDROID_LOG_INFO, LOG_TAG, __VA_ARGS__)
#define LOGE(...) __android_log_print(ANDROID_LOG_ERROR, LOG_TAG, __VA_ARGS__)

class AndroidDecoder : public IDecoder {
public:
    AndroidDecoder()
            : codec_(nullptr)
            , format_(nullptr)
            , width_(1920)
            , height_(1080)
            , frame_count_(0)
    {}

    ~AndroidDecoder() override {
        shutdown();
    }

    bool initialize() override {
        LOGI("Initializing Android MediaCodec decoder: %dx%d", width_, height_);

        // Create H.264 decoder
        codec_ = AMediaCodec_createDecoderByType("video/avc");
        if (!codec_) {
            LOGE("Failed to create H.264 decoder");
            return false;
        }

        // Create format
        format_ = AMediaFormat_new();
        AMediaFormat_setString(format_, AMEDIAFORMAT_KEY_MIME, "video/avc");
        AMediaFormat_setInt32(format_, AMEDIAFORMAT_KEY_WIDTH, width_);
        AMediaFormat_setInt32(format_, AMEDIAFORMAT_KEY_HEIGHT, height_);

#if __ANDROID_API__ >= 30
            AMediaFormat_setInt32(format_, AMEDIAFORMAT_KEY_LOW_LATENCY, 1);
#endif
#if __ANDROID_API__ >= 28
            AMediaFormat_setInt32(format_, AMEDIAFORMAT_KEY_PRIORITY, 0);
#endif

        // Configure without surface (buffer mode)
        media_status_t status = AMediaCodec_configure(codec_, format_, nullptr, nullptr, 0);
        if (status != AMEDIA_OK) {
            LOGE("Failed to configure codec: %d", status);
            return false;
        }

        // Start codec
        status = AMediaCodec_start(codec_);
        if (status != AMEDIA_OK) {
            LOGE("Failed to start codec: %d", status);
            return false;
        }

        LOGI("✓ Android MediaCodec decoder initialized");
        return true;
    }

    std::unique_ptr<DecodedFrame> decode(const std::vector<uint8_t>& packet) override {
        if (!codec_ || packet.empty()) {
            return nullptr;
        }

        // Get input buffer
        ssize_t input_index = AMediaCodec_dequeueInputBuffer(codec_, 10000);
        if (input_index < 0) {
            if (input_index == AMEDIACODEC_INFO_TRY_AGAIN_LATER) {
                return nullptr; // Not an error, just busy
            }
            LOGE("Failed to dequeue input buffer: %zd", input_index);
            return nullptr;
        }

        // Get buffer pointer
        size_t buffer_size;
        uint8_t* buffer = AMediaCodec_getInputBuffer(codec_, input_index, &buffer_size);

        if (!buffer) {
            LOGE("Failed to get input buffer");
            return nullptr;
        }

        // Copy packet data
        size_t data_size = std::min(packet.size(), buffer_size);
        std::memcpy(buffer, packet.data(), data_size);

        // Queue input buffer
        uint64_t pts = frame_count_++ * 16666; // ~60fps (16.666ms per frame)
        media_status_t status = AMediaCodec_queueInputBuffer(
                codec_, input_index, 0, data_size, pts, 0
        );

        if (status != AMEDIA_OK) {
            LOGE("Failed to queue input buffer: %d", status);
            return nullptr;
        }

        // Try to dequeue output (non-blocking)
        AMediaCodecBufferInfo info;
        ssize_t output_index = AMediaCodec_dequeueOutputBuffer(codec_, &info, 0);

        if (output_index >= 0) {
            // Get output buffer
            uint8_t* output_buffer = AMediaCodec_getOutputBuffer(codec_, output_index, &buffer_size);

            if (output_buffer) {
                // Create frame
                auto frame = std::make_unique<DecodedFrame>();
                frame->width = width_;
                frame->height = height_;
                frame->stride = width_ * 4; // RGBA
                frame->pts = info.presentationTimeUs;

                // Copy data
                frame->data.resize(info.size);
                std::memcpy(frame->data.data(), output_buffer + info.offset, info.size);

                // Release buffer
                AMediaCodec_releaseOutputBuffer(codec_, output_index, false);

                return frame;
            }

            AMediaCodec_releaseOutputBuffer(codec_, output_index, false);
        } else if (output_index == AMEDIACODEC_INFO_OUTPUT_FORMAT_CHANGED) {
            // Update format if needed
            AMediaFormat* new_format = AMediaCodec_getOutputFormat(codec_);
            AMediaFormat_getInt32(new_format, AMEDIAFORMAT_KEY_WIDTH, &width_);
            AMediaFormat_getInt32(new_format, AMEDIAFORMAT_KEY_HEIGHT, &height_);
            LOGI("Output format changed: %dx%d", width_, height_);
            AMediaFormat_delete(new_format);
        }

        return nullptr; // No frame available yet
    }

    void shutdown() override {
        if (codec_) {
            AMediaCodec_stop(codec_);
            AMediaCodec_delete(codec_);
            codec_ = nullptr;
        }

        if (format_) {
            AMediaFormat_delete(format_);
            format_ = nullptr;
        }

        LOGI("Android decoder shutdown");
    }

private:
    AMediaCodec* codec_;
    AMediaFormat* format_;
    int width_;
    int height_;
    uint64_t frame_count_;
};

// Factory function - Android implementation
std::unique_ptr<IDecoder> create_platform_decoder() {
    return std::make_unique<AndroidDecoder>();
}
