#include "client_core.hpp"
#include "decoder.hpp"
#include "renderer.hpp"
#include "input_handler.hpp"
#include "network_client.hpp"
#include "connection_manager.hpp"
#include <iostream>

ClientCore::ClientCore()
    : last_stats_time_(0)
    , frames_received_since_update_(0)
    , frames_decoded_since_update_(0)
    , frames_rendered_since_update_(0)
{
}

ClientCore::~ClientCore() {
    disconnect();
}

bool ClientCore::initialize(const ClientConfig& config, const PlatformCallbacks& callbacks, SDL_Window* window) {
    config_ = config;
    callbacks_ = callbacks;

    std::cout << "✓ Initializing Client Core" << std::endl;

    try {
        // Create decoder
        decoder_ = create_platform_decoder();
        if (!decoder_) {
            if (callbacks_.on_error) {
                callbacks_.on_error("Failed to create decoder");
            }
            return false;
        }

        // SIMPLIFIED: Initialize decoder immediately with default params
        // Don't wait for SPS/PPS, let FFmpeg handle it
        std::cout << "Initializing decoder..." << std::endl;
        if (!decoder_->initialize()) {
            if (callbacks_.on_error) {
                callbacks_.on_error("Failed to initialize decoder");
            }
            return false;
        }
        std::cout << "✓ Decoder initialized" << std::endl;

        // Create renderer
        renderer_ = std::make_unique<Renderer>(window);
        if (window != nullptr && !renderer_->initialize()) {
            if (callbacks_.on_error) {
                callbacks_.on_error("Failed to initialize renderer");
            }
            return false;
        }
        std::cout << "✓ Renderer initialized" << std::endl;

        // Create input handler
        input_handler_ = std::make_unique<InputHandler>(window);
        std::cout << "✓ Input handler created" << std::endl;

        // Create network client
        network_ = std::make_unique<NetworkClient>();
        std::cout << "✓ Network client created" << std::endl;

        // Create connection manager
        connection_manager_ = std::make_unique<ConnectionManager>();
        std::cout << "✓ Connection manager created" << std::endl;

        running_ = true;
        last_stats_time_ = 0;

        return true;

    }
    catch (const std::exception& e) {
        std::cerr << "Exception during initialization: " << e.what() << std::endl;
        if (callbacks_.on_error) {
            callbacks_.on_error(std::string("Init exception: ") + e.what());
        }
        return false;
    }
}

bool ClientCore::connect() {
    if (connected_) {
        return true;
    }

    std::string server_address = config_.server_address;

    if (config_.use_tailscale && !config_.tailscale_hostname.empty()) {
        std::cout << "Resolving Tailscale address: " << config_.tailscale_hostname << std::endl;

        auto tailscale_addr = connection_manager_->resolve_tailscale_address(
            config_.tailscale_hostname
        );

        if (tailscale_addr.empty()) {
            std::cerr << "Failed to resolve Tailscale address" << std::endl;
            if (callbacks_.on_error) {
                callbacks_.on_error("Failed to resolve Tailscale address");
            }
            return false;
        }

        server_address = tailscale_addr;
        std::cout << "✓ Resolved to: " << server_address << std::endl;
    }

    std::cout << "Connecting to: " << server_address << std::endl;

    if (!network_->connect(server_address)) {
        std::cerr << "Connection failed" << std::endl;
        if (callbacks_.on_error) {
            callbacks_.on_error("Failed to connect");
        }
        return false;
    }

    connected_ = true;

    if (callbacks_.on_status_change) {
        callbacks_.on_status_change("Connected");
    }

    std::cout << "✓ Connected successfully" << std::endl;
    return true;
}

bool ClientCore::update() {
    if (!running_) {
        return false;
    }

    if (callbacks_.should_quit && callbacks_.should_quit()) {
        running_ = false;
        return false;
    }

    if (paused_) {
        SDL_Delay(1);
        return true;
    }

    process_packets();

    if (last_stats_time_ == 0) {
        last_stats_time_ = SDL_GetPerformanceCounter();
    }

    uint64_t current_time = SDL_GetPerformanceCounter();
    uint64_t frequency = SDL_GetPerformanceFrequency();
    uint64_t elapsed_ms = ((current_time - last_stats_time_) * 1000) / frequency;

    if (elapsed_ms >= 1000) {
        update_stats();
        last_stats_time_ = current_time;
    }

    SDL_Delay(1);

    return connected_ && running_;
}

void ClientCore::process_packets() {
    const int max_packets_per_frame = 10;
    int packets_processed = 0;

    while (network_->has_data() && packets_processed < max_packets_per_frame) {
        auto packet_data = network_->receive();
        if (packet_data.empty()) break;
        packets_processed++;

        // SIMPLIFIED: Parse packet header (18 bytes)
        if (packet_data.size() < 18) continue;

        uint8_t packet_type = packet_data[0];
        uint32_t sequence =
            (packet_data[1] << 24) |
            (packet_data[2] << 16) |
            (packet_data[3] << 8) |
            packet_data[4];
        uint32_t payload_len =
            (packet_data[14] << 24) |
            (packet_data[15] << 16) |
            (packet_data[16] << 8) |
            packet_data[17];

        if (packet_data.size() < 18 + payload_len) continue;

        std::vector<uint8_t> payload(
            packet_data.begin() + 18,
            packet_data.begin() + 18 + payload_len
        );

        if (packet_type != 0x01) continue; // Video only

        frames_received_since_update_++;
        stats_.frames_received++;

        // SIMPLIFIED: Decode immediately, no state machine
        // Just pass the NAL unit to decoder
        auto frame = decoder_->decode(payload);

        if (!frame) {
            // Decoder needs more data or error
            continue;
        }

        frames_decoded_since_update_++;
        stats_.frames_decoded++;

        // Render
        if (renderer_) {
            renderer_->render(*frame);
            frames_rendered_since_update_++;
            stats_.frames_rendered++;
        }
    }
}

void ClientCore::update_stats() {
    stats_.average_fps = frames_rendered_since_update_;

    auto net_stats = network_->get_stats();
    stats_.average_latency_ms = static_cast<uint64_t>(net_stats.rtt_ms);

    if (callbacks_.on_stats_update) {
        callbacks_.on_stats_update(stats_.average_fps, stats_.average_latency_ms);
    }

    frames_received_since_update_ = 0;
    frames_decoded_since_update_ = 0;
    frames_rendered_since_update_ = 0;
}

void ClientCore::disconnect() {
    if (network_) {
        network_->disconnect();
    }
    connected_ = false;
    running_ = false;
}

ClientCore::Stats ClientCore::get_stats() const {
    return stats_;
}

void ClientCore::handle_sdl_event(const SDL_Event& event) {
    if (!input_handler_) return;

    switch (event.type) {
    case SDL_EVENT_KEY_DOWN:
        handle_keyboard_event(event.key, true);
        break;

    case SDL_EVENT_KEY_UP:
        handle_keyboard_event(event.key, false);
        break;

    case SDL_EVENT_MOUSE_MOTION:
        handle_mouse_motion(event.motion);
        break;

    case SDL_EVENT_MOUSE_BUTTON_DOWN:
        handle_mouse_button(event.button, true);
        break;

    case SDL_EVENT_MOUSE_BUTTON_UP:
        handle_mouse_button(event.button, false);
        break;
    }
}

void ClientCore::handle_keyboard_event(const SDL_KeyboardEvent& event, bool pressed) {
    input_handler_->handle_keyboard(event, pressed);
    if (network_ && connected_) {
        network_->send_input(input_handler_->get_last_event());
    }
}

void ClientCore::handle_mouse_motion(const SDL_MouseMotionEvent& event) {
    input_handler_->handle_mouse_motion(event);
    if (network_ && connected_) {
        network_->send_input(input_handler_->get_last_event());
    }
}

void ClientCore::handle_mouse_button(const SDL_MouseButtonEvent& event, bool pressed) {
    input_handler_->handle_mouse_button(event, pressed);
    if (network_ && connected_) {
        network_->send_input(input_handler_->get_last_event());
    }
}

void ClientCore::handle_touch_down(float x, float y, int finger_id) {
    if (!config_.is_mobile || !input_handler_) return;

    SDL_MouseButtonEvent button_event{};
    button_event.x = x * config_.width;
    button_event.y = y * config_.height;
    button_event.button = SDL_BUTTON_LEFT;

    handle_mouse_button(button_event, true);
}

void ClientCore::handle_touch_up(float x, float y, int finger_id) {
    if (!config_.is_mobile || !input_handler_) return;

    SDL_MouseButtonEvent button_event{};
    button_event.x = x * config_.width;
    button_event.y = y * config_.height;
    button_event.button = SDL_BUTTON_LEFT;

    handle_mouse_button(button_event, false);
}

void ClientCore::handle_touch_move(float x, float y, int finger_id) {
    if (!config_.is_mobile || !input_handler_) return;

    SDL_MouseMotionEvent motion_event{};
    motion_event.x = x * config_.width;
    motion_event.y = y * config_.height;

    handle_mouse_motion(motion_event);
}

void ClientCore::on_pause() {
    std::cout << "App paused" << std::endl;
    paused_ = true;
}

void ClientCore::on_resume() {
    std::cout << "App resumed" << std::endl;
    paused_ = false;
}

void ClientCore::on_low_memory() {
    std::cout << "Low memory warning" << std::endl;
}