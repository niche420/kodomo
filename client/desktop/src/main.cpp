#include <iostream>
#include <string>
#include <csignal>
#include <atomic>
#include <SDL3/SDL.h>
#include <SDL3/SDL_main.h>

#include "client_core.hpp"
#include "desktop_window.hpp"

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
              << "  --server <address>        Server address (default: 127.0.0.1:8080)\n"
              << "  --tailscale <hostname>    Connect via Tailscale hostname\n"
              << "  --width <width>           Window width (default: 1920)\n"
              << "  --height <height>         Window height (default: 1080)\n"
              << "  --fullscreen              Start in fullscreen mode\n"
              << "  --help                    Show this help message\n";
}

int main(int argc, char* argv[]) {
    // Parse command line arguments
    std::string server_address = "127.0.0.1:8080";
    std::string tailscale_hostname;
    bool use_tailscale = false;
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
        } else if (arg == "--tailscale" && i + 1 < argc) {
            tailscale_hostname = argv[++i];
            use_tailscale = true;
        } else if (arg == "--width" && i + 1 < argc) {
            width = std::stoi(argv[++i]);
        } else if (arg == "--height" && i + 1 < argc) {
            height = std::stoi(argv[++i]);
        } else if (arg == "--fullscreen") {
            fullscreen = true;
        }
    }

    std::cout << "🎮 Kodomo Desktop Client v0.1.0\n";
    std::cout << "Server: " << (use_tailscale ? tailscale_hostname : server_address) << "\n";
    std::cout << "Resolution: " << width << "x" << height << "\n";
    std::cout << "Tailscale: " << (use_tailscale ? "Enabled" : "Disabled") << "\n\n";

    // Setup signal handlers
    std::signal(SIGINT, signal_handler);
    std::signal(SIGTERM, signal_handler);

    try {
        // Initialize SDL
        if (!SDL_Init(SDL_INIT_VIDEO | SDL_INIT_EVENTS)) {
            std::cerr << "SDL_Init failed: " << SDL_GetError() << std::endl;
            return 1;
        }

        // Create desktop window
        DesktopWindow window;
        if (!window.create(width, height, fullscreen)) {
            std::cerr << "Failed to create window\n";
            SDL_Quit();
            return 1;
        }

        // Create client config
        ClientConfig config;
        config.server_address = server_address;
        config.width = width;
        config.height = height;
        config.fullscreen = fullscreen;
        config.use_tailscale = use_tailscale;
        config.tailscale_hostname = tailscale_hostname;
        config.is_mobile = false;
        config.touch_controls = false;

        // Setup callbacks
        PlatformCallbacks callbacks;

        callbacks.on_error = [](const std::string& error) {
            std::cerr << "❌ Error: " << error << std::endl;
        };

        callbacks.on_status_change = [](const std::string& status) {
            std::cout << "ℹ️  Status: " << status << std::endl;
        };

        callbacks.on_stats_update = [](uint64_t fps, uint64_t latency) {
            // Update window title with stats
            static uint64_t last_print = 0;
            uint64_t now = SDL_GetTicks();
            if (now - last_print >= 1000) {
                std::cout << "📊 FPS: " << fps << ", Latency: " << latency << "ms\n";
                last_print = now;
            }
        };

        callbacks.should_quit = [&window]() {
            return window.should_quit();
        };

        // Create and initialize client
        ClientCore client;
        if (!client.initialize(config, callbacks, window.get_window())) {
            std::cerr << "Failed to initialize client\n";
            return 1;
        }

        // Get renderer and attach to window
        window.set_renderer(client.get_renderer());

        // Connect to server
        if (!client.connect()) {
            std::cerr << "Failed to connect to server\n";
            return 1;
        }

        std::cout << "✓ Connected successfully\n";
        std::cout << "Controls:\n";
        std::cout << "  F11 - Toggle fullscreen\n";
        std::cout << "  ESC - Disconnect and exit\n";
        std::cout << "  Ctrl+C - Force quit\n\n";

        // Main event loop
        while (g_running && client.is_connected()) {
            // Handle window events
            SDL_Event event;
            while (SDL_PollEvent(&event)) {
                if (event.type == SDL_EVENT_QUIT) {
                    g_running = false;
                    break;
                }

                if (event.type == SDL_EVENT_KEY_DOWN && event.key.key == SDLK_ESCAPE) {
                    g_running = false;
                    break;
                }

                if (event.type == SDL_EVENT_KEY_DOWN && event.key.key == SDLK_F11) {
                    window.toggle_fullscreen();
                }

                // Forward event to client
                client.handle_sdl_event(event);
            }

            // Update client
            if (!client.update()) {
                break;
            }
        }

        client.disconnect();
        std::cout << "Disconnected from server\n";

        window.destroy();
        SDL_Quit();

    } catch (const std::exception& e) {
        std::cerr << "Error: " << e.what() << "\n";
        SDL_Quit();
        return 1;
    }

    return 0;
}