#include "renderer.hpp"
#include <iostream>

Renderer::Renderer(SDL_Window * window)
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
    // If window_ is null there's nothing we can do
    if (!window_) {
        std::cerr << "Renderer initialization failed: window is null\n";
        return false;
    }

    renderer_ = SDL_CreateRenderer(
        window_,
        NULL
    );

    if (!renderer_) {
        std::cerr << "Renderer creation failed: " << SDL_GetError() << std::endl;
        return false;
    }

    std::cout << "✓ Renderer initialized\n";
    return true;
}

void Renderer::render(const DecodedFrame& frame) {
    // Defensive: ensure renderer exists
    if (!renderer_) {
        std::cerr << "Render skipped: SDL_Renderer is null\n";
        return;
    }

    // Create or recreate texture if size changed
    if (!texture_ || texture_width_ != frame.width || texture_height_ != frame.height) {
        if (texture_) {
            SDL_DestroyTexture(texture_);
            texture_ = nullptr;
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
    if (!SDL_UpdateTexture(texture_, nullptr, frame.data.data(), frame.stride)) {
        std::cerr << "SDL_UpdateTexture failed: " << SDL_GetError() << std::endl;
        return;
    }

    // Defensive: check renderer again before rendering
    if (!renderer_) {
        std::cerr << "Render skipped: SDL_Renderer became null\n";
        return;
    }

    if (!SDL_RenderClear(renderer_)) {
        std::cerr << "SDL_RenderClear failed: " << SDL_GetError() << std::endl;
        // still attempt to present texture (optional), but bail out to avoid crash
        return;
    }

    if (!SDL_RenderTexture(renderer_, texture_, nullptr, nullptr)) {
        std::cerr << "SDL_RenderCopy failed: " << SDL_GetError() << std::endl;
    }

    SDL_RenderPresent(renderer_);
}

void Renderer::present() {
    // Defensive: ensure renderer exists
    if (!renderer_) {
        std::cerr << "Present skipped: SDL_Renderer is null\n";
        return;
    }

    if (texture_) {
        if (!SDL_RenderClear(renderer_)) {
            std::cerr << "SDL_RenderClear failed: " << SDL_GetError() << std::endl;
            return;
        }
        if (!SDL_RenderTexture(renderer_, texture_, nullptr, nullptr)) {
            std::cerr << "SDL_RenderCopy failed: " << SDL_GetError() << std::endl;
            return;
        }
        SDL_RenderPresent(renderer_);
    }
    else {
        // No texture: clear to black (optional) and present
        if (!SDL_RenderClear(renderer_)) {
            std::cerr << "SDL_RenderClear failed: " << SDL_GetError() << std::endl;
            return;
        }
        SDL_RenderPresent(renderer_);
    }
}

void Renderer::resize(int width, int height) {
    std::cout << "Window resized to " << width << "x" << height << "\n";

    // If window was recreated by the system, SDL may have invalidated the renderer.
    // Best practice: re-create the renderer and texture when resize/window events indicate a change.
    // For now we just drop the texture so it can be recreated on next render.
    if (texture_) {
        SDL_DestroyTexture(texture_);
        texture_ = nullptr;
        texture_width_ = 0;
        texture_height_ = 0;
    }
}