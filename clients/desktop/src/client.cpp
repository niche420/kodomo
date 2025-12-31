#include "client.hpp"
#include "decoder.hpp"
#include "renderer.hpp"
#include "input_handler.hpp"
#include "network_client.hpp"
#include <iostream>
#include <chrono>

StreamingClient::StreamingClient()
    : window_(nullptr)
    , fullscreen_(false)
    , connected_(false)
    , frames_received_(0)
    , frames_decoded_(0)
    , frames_rendered_(0)
    , last_stats_time_(0)
{
}

StreamingClient::~StreamingClient() {
    disconnect();

    if (window_) {
        SDL_DestroyWindow(window_);
    }

    SDL_Quit();
}

bool StreamingClient::initialize(int width, int height, bool fullscreen) {
    // Initialize SDL
    if (SDL_Init(SDL_INIT_VIDEO | SDL_INIT_EVENTS) < 0) {
        std::cerr << "SDL init failed: " << SDL_GetError() << std::endl;
        return false;
    }

    // Create window
    Uint32 flags = SDL_WINDOW_SHOWN | SDL_WINDOW_RESIZABLE;
    if (fullscreen) {
        flags |= SDL_WINDOW_FULLSCREEN_DESKTOP;
    }

    window_ = SDL_CreateWindow(
        "Game Streaming Client",
        SDL_WINDOWPOS_CENTERED, SDL_WINDOWPOS_CENTERED,
        width, height,
        flags
    );

    if (!window_) {
        std::cerr << "Window creation failed: " << SDL_GetError() << std::endl;
        return false;
    }

    fullscreen_ = fullscreen;

    // Create components
    decoder_ = std::make_unique<Decoder>();
    renderer_ = std::make_unique<Renderer>(window_);
    input_handler_ = std::make_unique<InputHandler>(window_);
    network_ = std::make_unique<NetworkClient>();

    if (!decoder_->initialize()) {
        std::cerr << "Failed to initialize decoder\n";
        return false;
    }

    if (!renderer_->initialize()) {
        std::cerr << "Failed to initialize renderer\n";
        return false;
    }

    std::cout << "✓ Client initialized\n";
    return true;
}

bool StreamingClient::connect(const std::string& server_address) {
    if (!network_->connect(server_address)) {
        return false;
    }

    connected_ = true;
    last_stats_time_ = SDL_GetTicks64();

    return true;
}

bool StreamingClient::update() {
    handle_events();

    // Receive and process all available packets
    int packets_processed = 0;
    const int max_packets_per_frame = 10; // Prevent blocking too long

    while (network_->has_data() && packets_processed < max_packets_per_frame) {
        auto packet_data = network_->receive();
        if (packet_data.empty()) {
            break;
        }

        packets_processed++;

        // Parse packet header
        // Format: [type:1][seq:4][timestamp:8][flags:1][payload_len:4][payload:N]
        if (packet_data.size() < 18) {
            std::cerr << "Packet too short: " << packet_data.size() << " bytes\n";
            continue;
        }

        // Extract packet type
        uint8_t packet_type = packet_data[0];

        // Extract payload length (bytes 14-17)
        uint32_t payload_len =
            (static_cast<uint32_t>(packet_data[14]) << 24) |
            (static_cast<uint32_t>(packet_data[15]) << 16) |
            (static_cast<uint32_t>(packet_data[16]) << 8) |
            static_cast<uint32_t>(packet_data[17]);

        // Verify packet size
        if (packet_data.size() < 18 + payload_len) {
            std::cerr << "Incomplete packet: expected " << (18 + payload_len)
                      << " bytes, got " << packet_data.size() << " bytes\n";
            continue;
        }

        // Extract payload
        std::vector<uint8_t> payload(
            packet_data.begin() + 18,
            packet_data.begin() + 18 + payload_len
        );

        // Only process video packets
        if (packet_type == 0x01) { // Video packet
            frames_received_++;

            // Decode frame
            auto frame = decoder_->decode(payload);
            if (frame) {
                frames_decoded_++;
                renderer_->render(*frame);
                frames_rendered_++;
            }
        }
    }

    // If no packets received, just present last frame
    if (packets_processed == 0) {
        renderer_->present();
    }

    // Update stats every second
    uint64_t current_time = SDL_GetTicks64();
    if (current_time - last_stats_time_ >= 1000) {
        update_stats();
        last_stats_time_ = current_time;
    }

    // Small delay to not burn CPU
    SDL_Delay(1);

    return connected_;
}

void StreamingClient::handle_events() {
    SDL_Event event;

    while (SDL_PollEvent(&event)) {
        switch (event.type) {
            case SDL_QUIT:
                connected_ = false;
                break;

            case SDL_KEYDOWN:
                if (event.key.keysym.sym == SDLK_ESCAPE) {
                    connected_ = false;
                } else if (event.key.keysym.sym == SDLK_F11) {
                    toggle_fullscreen();
                } else {
                    input_handler_->handle_keyboard(event.key, true);
                    network_->send_input(input_handler_->get_last_event());
                }
                break;

            case SDL_KEYUP:
                input_handler_->handle_keyboard(event.key, false);
                network_->send_input(input_handler_->get_last_event());
                break;

            case SDL_MOUSEMOTION:
                input_handler_->handle_mouse_motion(event.motion);
                network_->send_input(input_handler_->get_last_event());
                break;

            case SDL_MOUSEBUTTONDOWN:
            case SDL_MOUSEBUTTONUP:
                input_handler_->handle_mouse_button(
                    event.button,
                    event.type == SDL_MOUSEBUTTONDOWN
                );
                network_->send_input(input_handler_->get_last_event());
                break;

            case SDL_WINDOWEVENT:
                if (event.window.event == SDL_WINDOWEVENT_RESIZED) {
                    renderer_->resize(event.window.data1, event.window.data2);
                }
                break;
        }
    }
}

void StreamingClient::toggle_fullscreen() {
    fullscreen_ = !fullscreen_;

    if (fullscreen_) {
        SDL_SetWindowFullscreen(window_, SDL_WINDOW_FULLSCREEN_DESKTOP);
    } else {
        SDL_SetWindowFullscreen(window_, 0);
    }

    std::cout << "Fullscreen: " << (fullscreen_ ? "ON" : "OFF") << "\n";
}

void StreamingClient::update_stats() {
    auto net_stats = network_->get_stats();

    std::cout << "📊 FPS: RX=" << frames_received_
              << " Decoded=" << frames_decoded_
              << " Rendered=" << frames_rendered_
              << " | Network: " << net_stats.packets_received << " pkts, "
              << (net_stats.bytes_received / 1000) << " KB"
              << "\n";

    // Reset counters
    frames_received_ = 0;
    frames_decoded_ = 0;
    frames_rendered_ = 0;
}

void StreamingClient::disconnect() {
    if (network_) {
        network_->disconnect();
    }
    connected_ = false;
}