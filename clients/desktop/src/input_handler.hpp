#pragma once

#include <SDL3/SDL.h>
#include <vector>
#include <cstdint>

struct InputEvent {
    enum Type { KEYBOARD, MOUSE_MOVE, MOUSE_BUTTON } type;

    // Keyboard
    uint32_t keycode;
    bool pressed;

    // Mouse
    int32_t mouse_x;
    int32_t mouse_y;
    uint8_t mouse_button;

    uint64_t timestamp;
};

class InputHandler {
public:
    explicit InputHandler(SDL_Window* window);

    void handle_keyboard(const SDL_KeyboardEvent& event, bool pressed);
    void handle_mouse_motion(const SDL_MouseMotionEvent& event);
    void handle_mouse_button(const SDL_MouseButtonEvent& event, bool pressed);

    const InputEvent& get_last_event() const { return last_event_; }
    std::vector<uint8_t> serialize_event(const InputEvent& event) const;

private:
    SDL_Window* window_;
    InputEvent last_event_;
};