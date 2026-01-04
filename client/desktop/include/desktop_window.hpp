#pragma once

#include <SDL3/SDL.h>

class Renderer;

/**
 * Desktop-specific window management
 * Handles SDL window creation, fullscreen toggling, etc.
 */
class DesktopWindow {
public:
    DesktopWindow();
    ~DesktopWindow();

    bool create(int width, int height, bool fullscreen);
    void destroy();

    void toggle_fullscreen();
    void set_title(const char* title);

    SDL_Window* get_window() { return window_; }

    bool should_quit() const { return should_quit_; }
    void request_quit() { should_quit_ = true; }

    // Attach renderer for automatic window resizing
    void set_renderer(Renderer* renderer);

private:
    SDL_Window* window_;
    Renderer* renderer_;
    bool fullscreen_;
    bool should_quit_;
    int window_width_;
    int window_height_;
};