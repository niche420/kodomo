#include "client.hpp"
#include "decoder.hpp"
#include "renderer.hpp"
#include "input_handler.hpp"
#include "network_client.hpp"
#include <iostream>
#include <chrono>

// Packet flags (must match Rust side)
const uint8_t FLAG_KEYFRAME = 0x01;
const uint8_t FLAG_FRAGMENT = 0x02;
const uint8_t FLAG_LAST_FRAGMENT = 0x04;

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
    // Initialize SDL3
    if (!SDL_Init(SDL_INIT_VIDEO | SDL_INIT_EVENTS)) {
        std::cerr << "SDL init failed: " << SDL_GetError() << std::endl;
        return false;
    }

    // Create window (SDL3 uses flags differently)
    SDL_WindowFlags flags = SDL_WINDOW_MINIMIZED | SDL_WINDOW_RESIZABLE;
    if (fullscreen) {
        flags = SDL_WINDOW_FULLSCREEN;
    }

    window_ = SDL_CreateWindow(
        "Game Streaming Client",
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
    last_stats_time_ = SDL_GetTicks();

    return true;
}

bool StreamingClient::update() {
    handle_events();

    // Fragment reassembly state
    static std::vector<uint8_t> fragment_buffer;
    static uint32_t expected_sequence = 0;
    static bool reassembling = false;

    // Receive and process all available packets
    int packets_processed = 0;
    const int max_packets_per_frame = 10;

    while (network_->has_data() && packets_processed < max_packets_per_frame) {
        auto packet_data = network_->receive();
        if (packet_data.empty()) {
            break;
        }

        packets_processed++;

        // Parse packet header
        if (packet_data.size() < 18) {
            std::cerr << "Packet too short: " << packet_data.size() << " bytes\n";
            continue;
        }

        // Extract fields
        uint8_t packet_type = packet_data[0];

        uint32_t sequence =
            (static_cast<uint32_t>(packet_data[1]) << 24) |
            (static_cast<uint32_t>(packet_data[2]) << 16) |
            (static_cast<uint32_t>(packet_data[3]) << 8) |
            static_cast<uint32_t>(packet_data[4]);

        uint8_t flags = packet_data[13];

        uint32_t payload_len =
            (static_cast<uint32_t>(packet_data[14]) << 24) |
            (static_cast<uint32_t>(packet_data[15]) << 16) |
            (static_cast<uint32_t>(packet_data[16]) << 8) |
            static_cast<uint32_t>(packet_data[17]);

        if (packet_data.size() < 18 + payload_len) {
            std::cerr << "Incomplete packet: expected " << (18 + payload_len)
                << " bytes, got " << packet_data.size() << " bytes\n";
            continue;
        }

        std::vector<uint8_t> payload(
            packet_data.begin() + 18,
            packet_data.begin() + 18 + payload_len
        );

        if (packet_type != 0x01) {
            continue;
        }

        bool is_fragment = (flags & FLAG_FRAGMENT) != 0;
        bool is_last_fragment = (flags & FLAG_LAST_FRAGMENT) != 0;

        if (is_fragment) {
            if (!reassembling) {
                std::cout << "Starting fragment reassembly at sequence " << sequence << "\n";
                fragment_buffer.clear();
                expected_sequence = sequence;
                reassembling = true;
            }

            if (sequence != expected_sequence) {
                std::cerr << "Fragment sequence mismatch! Expected " << expected_sequence
                    << ", got " << sequence << ". Resetting.\n";
                fragment_buffer.clear();
                reassembling = false;
                continue;
            }

            fragment_buffer.insert(fragment_buffer.end(), payload.begin(), payload.end());
            expected_sequence++;

            std::cout << "Received fragment " << sequence << ", total size: "
                << fragment_buffer.size() << " bytes, last: " << is_last_fragment << "\n";

            if (is_last_fragment) {
                std::cout << "Fragment reassembly complete: " << fragment_buffer.size()
                    << " bytes total\n";

                frames_received_++;

                auto frame = decoder_->decode(fragment_buffer);
                if (frame) {
                    frames_decoded_++;
                    renderer_->render(*frame);
                    frames_rendered_++;
                }
                else {
                    std::cerr << "Failed to decode reassembled frame\n";
                }

                fragment_buffer.clear();
                reassembling = false;
            }
        }
        else {
            if (reassembling) {
                std::cerr << "Received non-fragmented packet while reassembling. Resetting.\n";
                fragment_buffer.clear();
                reassembling = false;
            }

            frames_received_++;

            auto frame = decoder_->decode(payload);
            if (frame) {
                frames_decoded_++;
                renderer_->render(*frame);
                frames_rendered_++;
            }
        }
    }

    if (packets_processed == 0) {
        renderer_->present();
    }

    // Update stats every second
    Uint64 current_time = SDL_GetTicks();
    if (current_time - last_stats_time_ >= 1000) {
        update_stats();
        last_stats_time_ = current_time;
    }

    SDL_Delay(1);

    return connected_;
}

void StreamingClient::handle_events() {
    SDL_Event event;

    while (SDL_PollEvent(&event)) {
        switch (event.type) {
        case SDL_EVENT_QUIT:
            connected_ = false;
            break;

        case SDL_EVENT_KEY_DOWN:
            if (event.key.key == SDLK_ESCAPE) {
                connected_ = false;
            }
            else if (event.key.key == SDLK_F11) {
                toggle_fullscreen();
            }
            else {
                input_handler_->handle_keyboard(event.key, true);
                network_->send_input(input_handler_->get_last_event());
            }
            break;

        case SDL_EVENT_KEY_UP:
            input_handler_->handle_keyboard(event.key, false);
            network_->send_input(input_handler_->get_last_event());
            break;

        case SDL_EVENT_MOUSE_MOTION:
            input_handler_->handle_mouse_motion(event.motion);
            network_->send_input(input_handler_->get_last_event());
            break;

        case SDL_EVENT_MOUSE_BUTTON_DOWN:
        case SDL_EVENT_MOUSE_BUTTON_UP:
            input_handler_->handle_mouse_button(
                event.button,
                event.type == SDL_EVENT_MOUSE_BUTTON_DOWN
            );
            network_->send_input(input_handler_->get_last_event());
            break;

        case SDL_EVENT_WINDOW_RESIZED:
            renderer_->resize(event.window.data1, event.window.data2);
            break;
        }
    }
}

void StreamingClient::toggle_fullscreen() {
    fullscreen_ = !fullscreen_;
    SDL_SetWindowFullscreen(window_, fullscreen_);
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