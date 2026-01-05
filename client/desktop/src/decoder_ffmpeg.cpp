#include "decoder.hpp"
#include <iostream>
#include <cstring>

extern "C" {
#include <libavcodec/avcodec.h>
#include <libavutil/imgutils.h>
#include <libswscale/swscale.h>
}

class FFmpegDecoder : public IDecoder {
public:
    FFmpegDecoder()
        : codec_(nullptr)
        , context_(nullptr)
        , packet_(nullptr)
        , frame_(nullptr)
        , frame_rgb_(nullptr)
        , sws_context_(nullptr)
        , buffer_(nullptr)
        , width_(0)
        , height_(0)
    {}

    ~FFmpegDecoder() override {
        shutdown();
    }

    bool initialize() override {
        std::cout << "Initializing FFmpeg H.264 decoder..." << std::endl;

        // Find H.264 decoder
        codec_ = avcodec_find_decoder(AV_CODEC_ID_H264);
        if (!codec_) {
            std::cerr << "H.264 codec not found\n";
            return false;
        }

        // Allocate codec context
        context_ = avcodec_alloc_context3(codec_);
        if (!context_) {
            std::cerr << "Failed to allocate codec context\n";
            return false;
        }

        // Set decoder options for low latency
        context_->thread_count = 4;
        context_->thread_type = FF_THREAD_FRAME;
        context_->flags |= AV_CODEC_FLAG_LOW_DELAY;
        context_->flags2 |= AV_CODEC_FLAG2_FAST;

        // Open codec
        if (avcodec_open2(context_, codec_, nullptr) < 0) {
            std::cerr << "Failed to open codec\n";
            return false;
        }

        // Allocate packet and frames
        packet_ = av_packet_alloc();
        frame_ = av_frame_alloc();
        frame_rgb_ = av_frame_alloc();

        if (!packet_ || !frame_ || !frame_rgb_) {
            std::cerr << "Failed to allocate packet/frames\n";
            return false;
        }

        std::cout << "✓ FFmpeg H.264 decoder initialized\n";
        return true;
    }

    std::unique_ptr<DecodedFrame> decode(const std::vector<uint8_t>& packet_data) override {
        if (packet_data.empty()) {
            return nullptr;
        }

        // Fill packet with data
        packet_->data = const_cast<uint8_t*>(packet_data.data());
        packet_->size = static_cast<int>(packet_data.size());
        packet_->pts = AV_NOPTS_VALUE;
        packet_->dts = AV_NOPTS_VALUE;

        // Send packet to decoder
        int ret = avcodec_send_packet(context_, packet_);
        if (ret < 0) {
            av_packet_unref(packet_);
            return nullptr;
        }

        av_packet_unref(packet_);

        // Receive decoded frame
        ret = avcodec_receive_frame(context_, frame_);
        if (ret == AVERROR(EAGAIN) || ret == AVERROR_EOF) {
            return nullptr;
        } else if (ret < 0) {
            return nullptr;
        }

        // Initialize scaler if needed
        if (!sws_context_ || width_ != frame_->width || height_ != frame_->height) {
            width_ = frame_->width;
            height_ = frame_->height;

            if (sws_context_) {
                sws_freeContext(sws_context_);
            }

            sws_context_ = sws_getContext(
                width_, height_, static_cast<AVPixelFormat>(frame_->format),
                width_, height_, AV_PIX_FMT_RGBA,
                SWS_BILINEAR, nullptr, nullptr, nullptr
            );

            if (!sws_context_) {
                return nullptr;
            }

            int num_bytes = av_image_get_buffer_size(AV_PIX_FMT_RGBA, width_, height_, 1);
            if (buffer_) {
                av_free(buffer_);
            }
            buffer_ = static_cast<uint8_t*>(av_malloc(num_bytes));

            av_image_fill_arrays(
                frame_rgb_->data, frame_rgb_->linesize,
                buffer_, AV_PIX_FMT_RGBA,
                width_, height_, 1
            );
        }

        // Convert YUV to RGB
        sws_scale(
            sws_context_,
            frame_->data, frame_->linesize,
            0, height_,
            frame_rgb_->data, frame_rgb_->linesize
        );

        // Create decoded frame
        auto decoded = std::make_unique<DecodedFrame>();
        decoded->width = width_;
        decoded->height = height_;
        decoded->stride = frame_rgb_->linesize[0];
        decoded->pts = frame_->pts;

        size_t data_size = decoded->stride * decoded->height;
        decoded->data.resize(data_size);
        std::memcpy(decoded->data.data(), frame_rgb_->data[0], data_size);

        return decoded;
    }

    void shutdown() override {
        if (buffer_) {
            av_free(buffer_);
            buffer_ = nullptr;
        }
        if (frame_rgb_) {
            av_frame_free(&frame_rgb_);
        }
        if (frame_) {
            av_frame_free(&frame_);
        }
        if (packet_) {
            av_packet_free(&packet_);
        }
        if (sws_context_) {
            sws_freeContext(sws_context_);
            sws_context_ = nullptr;
        }
        if (context_) {
            avcodec_free_context(&context_);
        }
    }

private:
    const AVCodec* codec_;
    AVCodecContext* context_;
    AVPacket* packet_;
    AVFrame* frame_;
    AVFrame* frame_rgb_;
    SwsContext* sws_context_;
    uint8_t* buffer_;
    int width_;
    int height_;
};

// Factory function - Desktop implementation
std::unique_ptr<IDecoder> create_platform_decoder() {
    return std::make_unique<FFmpegDecoder>();
}