#include "connection_manager.hpp"
#include <iostream>
#include <sstream>
#include <array>
#include <memory>
#include <cstdio>

#ifdef __ANDROID__
#include <sys/types.h>
#include <sys/socket.h>
#include <netdb.h>
#include <arpa/inet.h>
#endif

#ifdef _WIN32
#include <windows.h>
#else
#include <unistd.h>
#include <sys/wait.h>
#endif

ConnectionManager::ConnectionManager() {
}

ConnectionManager::~ConnectionManager() {
}

std::string ConnectionManager::execute_tailscale_command(const std::string& command) {
#ifdef _WIN32
    // Windows: use popen
    std::array<char, 128> buffer;
    std::string result;

    std::string full_command = "tailscale " + command + " 2>&1";

    std::unique_ptr<FILE, decltype(&_pclose)> pipe(
        _popen(full_command.c_str(), "r"),
        _pclose
    );

    if (!pipe) {
        return "";
    }

    while (fgets(buffer.data(), buffer.size(), pipe.get()) != nullptr) {
        result += buffer.data();
    }

    return result;
#else
    // Unix-like: use popen
    std::array<char, 128> buffer;
    std::string result;

    std::string full_command = "tailscale " + command + " 2>&1";

    std::unique_ptr<FILE, decltype(&pclose)> pipe(
        popen(full_command.c_str(), "r"),
        pclose
    );

    if (!pipe) {
        return "";
    }

    while (fgets(buffer.data(), buffer.size(), pipe.get()) != nullptr) {
        result += buffer.data();
    }

    return result;
#endif
}

bool ConnectionManager::is_tailscale_available() {
    std::string output = execute_tailscale_command("version");
    return !output.empty() && output.find("tailscale") != std::string::npos;
}

std::optional<ConnectionManager::TailscaleStatus> ConnectionManager::get_tailscale_status() {
    std::string output = execute_tailscale_command("status --json");

    if (output.empty()) {
        return std::nullopt;
    }

    // Simple JSON parsing (in production, use a proper JSON library)
    TailscaleStatus status;

    // Check if connected
    status.is_connected = output.find("\"Online\":true") != std::string::npos ||
                         output.find("\"online\":true") != std::string::npos;

    // Parse IP (look for "TailscaleIPs")
    size_t ip_pos = output.find("\"TailscaleIPs\":");
    if (ip_pos != std::string::npos) {
        size_t start = output.find("\"", ip_pos + 16);
        size_t end = output.find("\"", start + 1);
        if (start != std::string::npos && end != std::string::npos) {
            status.ip_address = output.substr(start + 1, end - start - 1);
        }
    }

    return status;
}

std::string ConnectionManager::resolve_tailscale_address(const std::string& hostname) {
#ifdef __ANDROID__
    // If input is already an IP with optional port, return as-is
    bool has_port = hostname.find(':') != std::string::npos;
    sockaddr_in sa{};
    if (inet_pton(AF_INET, hostname.c_str(), &sa.sin_addr) == 1) {
        return hostname; // Valid IPv4, just use it
    }

    // Otherwise, try getaddrinfo to resolve hostname
    struct addrinfo hints{}, *result = nullptr;
    hints.ai_family = AF_INET;
    hints.ai_socktype = SOCK_DGRAM;
    hints.ai_flags = AI_ADDRCONFIG;

    int ret = getaddrinfo(hostname.c_str(), "8080", &hints, &result);
    if (ret != 0 || !result) {
        return hostname + ":8080"; // Last resort, return with default port
    }

    char ip_str[INET_ADDRSTRLEN];
    sockaddr_in* addr = (sockaddr_in*)result->ai_addr;
    inet_ntop(AF_INET, &addr->sin_addr, ip_str, sizeof(ip_str));
    freeaddrinfo(result);

    return std::string(ip_str) + ":8080";
#else
    // Desktop code unchanged
    return hostname;
#endif
}

std::string ConnectionManager::parse_ip_from_status(
    const std::string& status,
    const std::string& hostname
) {
    std::istringstream stream(status);
    std::string line;

    // Parse tailscale status output
    // Format: "IP              HOSTNAME        OS            RELAY   ONLINE"
    // Example: "100.64.0.2      my-server      linux         -       active"

    while (std::getline(stream, line)) {
        // Look for hostname in line
        if (line.find(hostname) != std::string::npos) {
            // Extract IP (first token)
            std::istringstream line_stream(line);
            std::string ip;
            line_stream >> ip;

            // Validate IP format (simple check)
            if (ip.find('.') != std::string::npos) {
                return ip;
            }
        }
    }

    return "";
}