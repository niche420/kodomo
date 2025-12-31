#pragma once

#include <string>
#include <memory>
#include <SDL2/SDL.h>

class Decoder;
class Renderer;
class InputHandler;
class NetworkClient;

class StreamingClient {
public:
    StreamingClient();
    ~StreamingClient();

    bool initialize(int width, int height, bool fullscreen);
    bool connect(const std::string& server_address);
    bool update();
    void disconnect();

    bool is_connected() const { return connected_; }

private:
    void handle_events();
    void render_frame();
    void update_stats();
    void toggle_fullscreen();

    SDL_Window* window_;
    bool fullscreen_;
    bool connected_;

    std::unique_ptr<Decoder> decoder_;
    std::unique_ptr<Renderer> renderer_;
    std::unique_ptr<InputHandler> input_handler_;
    std::unique_ptr<NetworkClient> network_;

    // Statistics
    uint64_t frames_received_;
    uint64_t frames_decoded_;
    uint64_t frames_rendered_;
    uint64_t last_stats_time_;
};