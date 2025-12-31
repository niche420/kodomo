#include "input_handler.hpp"

InputHandler::InputHandler(SDL_Window* window)
    : window_(window)
{
}

void InputHandler::handle_keyboard(const SDL_KeyboardEvent& event, bool pressed) {
    last_event_.type = InputEvent::KEYBOARD;
    last_event_.keycode = event.keysym.sym;
    last_event_.pressed = pressed;
    last_event_.timestamp = SDL_GetTicks64();
}

void InputHandler::handle_mouse_motion(const SDL_MouseMotionEvent& event) {
    last_event_.type = InputEvent::MOUSE_MOVE;
    last_event_.mouse_x = event.x;
    last_event_.mouse_y = event.y;
    last_event_.timestamp = SDL_GetTicks64();
}

void InputHandler::handle_mouse_button(const SDL_MouseButtonEvent& event, bool pressed) {
    last_event_.type = InputEvent::MOUSE_BUTTON;
    last_event_.mouse_x = event.x;
    last_event_.mouse_y = event.y;
    last_event_.mouse_button = event.button;
    last_event_.pressed = pressed;
    last_event_.timestamp = SDL_GetTicks64();
}

std::vector<uint8_t> InputHandler::serialize_event(const InputEvent& event) const {
    std::vector<uint8_t> data;
    // TODO: Serialize to binary format for network transmission
    return data;
}