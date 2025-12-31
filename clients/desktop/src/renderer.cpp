#include "renderer.hpp"
#include <iostream>

Renderer::Renderer(SDL_Window* window)
    : window_(window)
    , renderer_(nullptr)
    , texture_(nullptr)
    , texture_width_(0)
    , texture_height_(0)
{
}

Renderer::~Renderer() {
    if (texture_) {
        SDL_DestroyTexture(texture_);
    }
    if (renderer_) {
        SDL_DestroyRenderer(renderer_);
    }
}

bool Renderer::initialize() {
    renderer_ = SDL_CreateRenderer(
        window_,
        -1,
        SDL_RENDERER_ACCELERATED | SDL_RENDERER_PRESENTVSYNC
    );

    if (!renderer_) {
        std::cerr << "Renderer creation failed: " << SDL_GetError() << std::endl;
        return false;
    }

    std::cout << "✓ Renderer initialized\n";
    return true;
}

void Renderer::render(const DecodedFrame& frame) {
    // Create or recreate texture if size changed
    if (!texture_ || texture_width_ != frame.width || texture_height_ != frame.height) {
        if (texture_) {
            SDL_DestroyTexture(texture_);
        }

        texture_ = SDL_CreateTexture(
            renderer_,
            SDL_PIXELFORMAT_RGBA32,
            SDL_TEXTUREACCESS_STREAMING,
            frame.width,
            frame.height
        );

        if (!texture_) {
            std::cerr << "Texture creation failed: " << SDL_GetError() << std::endl;
            return;
        }

        texture_width_ = frame.width;
        texture_height_ = frame.height;
    }

    // Update texture with frame data
    SDL_UpdateTexture(
        texture_,
        nullptr,
        frame.data.data(),
        frame.stride
    );

    // Render
    SDL_RenderClear(renderer_);
    SDL_RenderCopy(renderer_, texture_, nullptr, nullptr);
    SDL_RenderPresent(renderer_);
}

void Renderer::present() {
    if (texture_) {
        SDL_RenderClear(renderer_);
        SDL_RenderCopy(renderer_, texture_, nullptr, nullptr);
        SDL_RenderPresent(renderer_);
    }
}

void Renderer::resize(int width, int height) {
    std::cout << "Window resized to " << width << "x" << height << "\n";
}