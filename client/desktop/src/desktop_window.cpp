#include "desktop_window.hpp"
#include "renderer.hpp"
#include <iostream>

DesktopWindow::DesktopWindow()
    : window_(nullptr)
    , renderer_(nullptr)
    , fullscreen_(false)
    , should_quit_(false)
    , window_width_(0)
    , window_height_(0)
{
}

DesktopWindow::~DesktopWindow() {
    destroy();
}

bool DesktopWindow::create(int width, int height, bool fullscreen) {
    window_width_ = width;
    window_height_ = height;
    fullscreen_ = fullscreen;

    SDL_WindowFlags flags = SDL_WINDOW_MINIMIZED | SDL_WINDOW_RESIZABLE;
    if (fullscreen) {
        flags = SDL_WINDOW_FULLSCREEN;
    }

    window_ = SDL_CreateWindow(
        "Kodomo Client",
        width,
        height,
        flags
    );

    if (!window_) {
        std::cerr << "Failed to create window: " << SDL_GetError() << std::endl;
        return false;
    }

    std::cout << "✓ Window created: " << width << "x" << height << std::endl;
    return true;
}

void DesktopWindow::destroy() {
    if (window_) {
        SDL_DestroyWindow(window_);
        window_ = nullptr;
    }
}

void DesktopWindow::toggle_fullscreen() {
    fullscreen_ = !fullscreen_;

    if (fullscreen_) {
        SDL_SetWindowFullscreen(window_, true);
        std::cout << "✓ Fullscreen: ON\n";
    } else {
        SDL_SetWindowFullscreen(window_, false);
        SDL_SetWindowSize(window_, window_width_, window_height_);
        std::cout << "✓ Fullscreen: OFF\n";
    }
}

void DesktopWindow::set_title(const char* title) {
    if (window_) {
        SDL_SetWindowTitle(window_, title);
    }
}

void DesktopWindow::set_renderer(Renderer* renderer) {
    renderer_ = renderer;
}