#pragma once

#include <string>
#include <memory>
#include <functional>
#include <SDL3/SDL.h>

class IDecoder;
class Renderer;
class InputHandler;
class NetworkClient;
class ConnectionManager;

// Platform-specific callbacks
struct PlatformCallbacks {
    std::function<void(const std::string&)> on_error;
    std::function<void(const std::string&)> on_status_change;
    std::function<void(uint64_t fps, uint64_t latency)> on_stats_update;
    std::function<bool()> should_quit;
};

// Client configuration
struct ClientConfig {
    std::string server_address;
    int width = 1920;
    int height = 1080;
    bool fullscreen = false;
    bool use_tailscale = true;
    std::string tailscale_hostname;

    // Mobile-specific
    bool is_mobile = false;
    bool touch_controls = false;
    float dpi_scale = 1.0f;
};

// Core client that works on all platforms
class ClientCore {
public:
    ClientCore();
    ~ClientCore();

    // Initialize with config and platform callbacks
    bool initialize(const ClientConfig& config, const PlatformCallbacks& callbacks, SDL_Window* window);

    // Connect to server (handles Tailscale if enabled)
    bool connect();

    // Main update loop - call this frequently (e.g., 60Hz)
    // Returns false when should quit
    bool update();

    // Disconnect and cleanup
    void disconnect();

    // Query state
    bool is_connected() const { return connected_; }
    bool is_running() const { return running_; }

    // Get components (for platform-specific extensions)
    Renderer* get_renderer() { return renderer_.get(); }
    InputHandler* get_input_handler() { return input_handler_.get(); }

    // Stats
    struct Stats {
        uint64_t frames_received = 0;
        uint64_t frames_decoded = 0;
        uint64_t frames_rendered = 0;
        uint64_t frames_dropped = 0;
        uint64_t average_latency_ms = 0;
        double average_fps = 0.0;
    };

    Stats get_stats() const;

    // Handle SDL events (call from platform event loop)
    void handle_sdl_event(const SDL_Event& event);

    // Mobile-specific: handle touch events
    void handle_touch_down(float x, float y, int finger_id);
    void handle_touch_up(float x, float y, int finger_id);
    void handle_touch_move(float x, float y, int finger_id);

    // Mobile-specific: handle app lifecycle
    void on_pause();
    void on_resume();
    void on_low_memory();

private:
    void handle_keyboard_event(const SDL_KeyboardEvent& event, bool pressed);
    void handle_mouse_motion(const SDL_MouseMotionEvent& event);
    void handle_mouse_button(const SDL_MouseButtonEvent& event, bool pressed);

    void update_stats();
    void process_packets();

    ClientConfig config_;
    PlatformCallbacks callbacks_;

    bool running_ = false;
    bool connected_ = false;
    bool paused_ = false;

    std::unique_ptr<IDecoder> decoder_;
    std::unique_ptr<Renderer> renderer_;
    std::unique_ptr<InputHandler> input_handler_;
    std::unique_ptr<NetworkClient> network_;
    std::unique_ptr<ConnectionManager> connection_manager_;

    // Statistics
    Stats stats_;
    uint64_t last_stats_time_;
    uint64_t frames_received_since_update_;
    uint64_t frames_decoded_since_update_;
    uint64_t frames_rendered_since_update_;
};