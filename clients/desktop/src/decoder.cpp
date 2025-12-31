#include "decoder.hpp"
#include <iostream>
#include <cstring>

// FFmpeg C headers
extern "C" {
#include <libavcodec/avcodec.h>
#include <libavutil/imgutils.h>
#include <libswscale/swscale.h>
}

struct DecoderImpl {
    const AVCodec* codec;
    AVCodecContext* context;
    AVPacket* packet;
    AVFrame* frame;
    AVFrame* frame_rgb;
    SwsContext* sws_context;
    uint8_t* buffer;
    int width;
    int height;
};

Decoder::Decoder()
    : decoder_context_(nullptr)
    , initialized_(false)
{
}

Decoder::~Decoder() {
    if (!decoder_context_) return;

    auto* impl = static_cast<DecoderImpl*>(decoder_context_);

    if (impl->buffer) {
        av_free(impl->buffer);
    }
    if (impl->frame_rgb) {
        av_frame_free(&impl->frame_rgb);
    }
    if (impl->frame) {
        av_frame_free(&impl->frame);
    }
    if (impl->packet) {
        av_packet_free(&impl->packet);
    }
    if (impl->sws_context) {
        sws_freeContext(impl->sws_context);
    }
    if (impl->context) {
        avcodec_free_context(&impl->context);
    }

    delete impl;
}

bool Decoder::initialize() {
    auto* impl = new DecoderImpl{};
    decoder_context_ = impl;

    // Find H.264 decoder
    impl->codec = avcodec_find_decoder(AV_CODEC_ID_H264);
    if (!impl->codec) {
        std::cerr << "H.264 codec not found\n";
        return false;
    }

    // Allocate codec context
    impl->context = avcodec_alloc_context3(impl->codec);
    if (!impl->context) {
        std::cerr << "Failed to allocate codec context\n";
        return false;
    }

    // Set decoder options for low latency
    impl->context->thread_count = 4;
    impl->context->thread_type = FF_THREAD_FRAME;
    impl->context->flags |= AV_CODEC_FLAG_LOW_DELAY;
    impl->context->flags2 |= AV_CODEC_FLAG2_FAST;

    // Open codec
    if (avcodec_open2(impl->context, impl->codec, nullptr) < 0) {
        std::cerr << "Failed to open codec\n";
        return false;
    }

    // Allocate packet and frames
    impl->packet = av_packet_alloc();
    impl->frame = av_frame_alloc();
    impl->frame_rgb = av_frame_alloc();

    if (!impl->packet || !impl->frame || !impl->frame_rgb) {
        std::cerr << "Failed to allocate packet/frames\n";
        return false;
    }

    impl->sws_context = nullptr;
    impl->buffer = nullptr;
    impl->width = 0;
    impl->height = 0;

    initialized_ = true;
    std::cout << "✓ H.264 decoder initialized\n";
    return true;
}

std::unique_ptr<DecodedFrame> Decoder::decode(const std::vector<uint8_t>& packet) {
    if (!initialized_ || packet.empty()) {
        return nullptr;
    }

    auto* impl = static_cast<DecoderImpl*>(decoder_context_);

    // Fill packet with data
    impl->packet->data = const_cast<uint8_t*>(packet.data());
    impl->packet->size = static_cast<int>(packet.size());

    // Send packet to decoder
    int ret = avcodec_send_packet(impl->context, impl->packet);
    if (ret < 0) {
        std::cerr << "Error sending packet to decoder\n";
        return nullptr;
    }

    // Receive decoded frame
    ret = avcodec_receive_frame(impl->context, impl->frame);
    if (ret == AVERROR(EAGAIN) || ret == AVERROR_EOF) {
        // Need more data
        return nullptr;
    } else if (ret < 0) {
        std::cerr << "Error receiving frame from decoder\n";
        return nullptr;
    }

    // Initialize scaler if needed
    if (!impl->sws_context ||
        impl->width != impl->frame->width ||
        impl->height != impl->frame->height) {

        impl->width = impl->frame->width;
        impl->height = impl->frame->height;

        if (impl->sws_context) {
            sws_freeContext(impl->sws_context);
        }

        impl->sws_context = sws_getContext(
            impl->width, impl->height, static_cast<AVPixelFormat>(impl->frame->format),
            impl->width, impl->height, AV_PIX_FMT_RGBA,
            SWS_BILINEAR, nullptr, nullptr, nullptr
        );

        if (!impl->sws_context) {
            std::cerr << "Failed to initialize scaler\n";
            return nullptr;
        }

        // Allocate buffer for RGB data
        int num_bytes = av_image_get_buffer_size(AV_PIX_FMT_RGBA, impl->width, impl->height, 1);
        if (impl->buffer) {
            av_free(impl->buffer);
        }
        impl->buffer = static_cast<uint8_t*>(av_malloc(num_bytes));

        // Setup RGB frame
        av_image_fill_arrays(
            impl->frame_rgb->data,
            impl->frame_rgb->linesize,
            impl->buffer,
            AV_PIX_FMT_RGBA,
            impl->width,
            impl->height,
            1
        );
    }

    // Convert YUV to RGB
    sws_scale(
        impl->sws_context,
        impl->frame->data,
        impl->frame->linesize,
        0,
        impl->height,
        impl->frame_rgb->data,
        impl->frame_rgb->linesize
    );

    // Create decoded frame
    auto decoded = std::make_unique<DecodedFrame>();
    decoded->width = impl->width;
    decoded->height = impl->height;
    decoded->stride = impl->frame_rgb->linesize[0];
    decoded->pts = impl->frame->pts;

    // Copy RGB data
    size_t data_size = decoded->stride * decoded->height;
    decoded->data.resize(data_size);
    std::memcpy(decoded->data.data(), impl->frame_rgb->data[0], data_size);

    return decoded;
}