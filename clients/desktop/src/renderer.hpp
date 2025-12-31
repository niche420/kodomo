#pragma once

#include <SDL2/SDL.h>
#include "decoder.hpp"

class Renderer {
public:
    explicit Renderer(SDL_Window* window);
    ~Renderer();

    bool initialize();
    void render(const DecodedFrame& frame);
    void present();
    void resize(int width, int height);

private:
    SDL_Window* window_;
    SDL_Renderer* renderer_;
    SDL_Texture* texture_;
    int texture_width_;
    int texture_height_;
};