#include <iostream>
#include <string>
#include <csignal>
#include <atomic>
#include "client.hpp"
#include <SDL3/SDL_main.h>

std::atomic<bool> g_running{true};

void signal_handler(int signal) {
    if (signal == SIGINT || signal == SIGTERM) {
        std::cout << "\nReceived signal, shutting down..." << std::endl;
        g_running = false;
    }
}

void print_usage(const char* program_name) {
    std::cout << "Usage: " << program_name << " [options]\n"
              << "Options:\n"
              << "  --server <address>    Server address (default: 127.0.0.1:8080)\n"
              << "  --width <width>       Window width (default: 1920)\n"
              << "  --height <height>     Window height (default: 1080)\n"
              << "  --fullscreen          Start in fullscreen mode\n"
              << "  --help                Show this help message\n";
}

int main(int argc, char* argv[]) {
    // Parse command line arguments
    std::string server_address = "127.0.0.1:8080";
    int width = 1920;
    int height = 1080;
    bool fullscreen = false;

    for (int i = 1; i < argc; ++i) {
        std::string arg = argv[i];

        if (arg == "--help") {
            print_usage(argv[0]);
            return 0;
        } else if (arg == "--server" && i + 1 < argc) {
            server_address = argv[++i];
        } else if (arg == "--width" && i + 1 < argc) {
            width = std::stoi(argv[++i]);
        } else if (arg == "--height" && i + 1 < argc) {
            height = std::stoi(argv[++i]);
        } else if (arg == "--fullscreen") {
            fullscreen = true;
        }
    }

    std::cout << "🎮 Game Streaming Client v0.1.0\n";
    std::cout << "Connecting to: " << server_address << "\n";
    std::cout << "Resolution: " << width << "x" << height << "\n";

    // Setup signal handlers
    std::signal(SIGINT, signal_handler);
    std::signal(SIGTERM, signal_handler);

    try {
        // Create and initialize client
        StreamingClient client;

        if (!client.initialize(width, height, fullscreen)) {
            std::cerr << "Failed to initialize client\n";
            return 1;
        }

        if (!client.connect(server_address)) {
            std::cerr << "Failed to connect to server\n";
            return 1;
        }

        std::cout << "✓ Connected successfully\n";
        std::cout << "Controls:\n";
        std::cout << "  F11 - Toggle fullscreen\n";
        std::cout << "  ESC - Disconnect and exit\n";
        std::cout << "  Ctrl+C - Force quit\n\n";

        // Main loop
        while (g_running && client.is_connected()) {
            if (!client.update()) {
                break;
            }
        }

        client.disconnect();
        std::cout << "Disconnected from server\n";

    } catch (const std::exception& e) {
        std::cerr << "Error: " << e.what() << "\n";
        return 1;
    }

    return 0;
}