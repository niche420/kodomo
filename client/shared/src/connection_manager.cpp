#include "connection_manager.hpp"
#include <iostream>
#include <sstream>
#include <array>
#include <memory>
#include <cstdio>

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
    std::cout << "Resolving Tailscale hostname: " << hostname << std::endl;

    // Check if Tailscale is available
    if (!is_tailscale_available()) {
        std::cerr << "Tailscale is not available on this system" << std::endl;
        return "";
    }

    // Get status
    auto status = get_tailscale_status();
    if (!status || !status->is_connected) {
        std::cerr << "Tailscale is not connected" << std::endl;
        return "";
    }

    std::cout << "Tailscale is connected, IP: " << status->ip_address << std::endl;

    // Method 1: Try 'tailscale ping' to resolve hostname
    std::string ping_output = execute_tailscale_command("ping -c 1 " + hostname);

    if (ping_output.empty()) {
        std::cerr << "Failed to ping " << hostname << std::endl;
        return "";
    }

    // Parse IP from ping output
    // Format: "pong from hostname (IP:PORT) via ..."
    size_t open_paren = ping_output.find('(');
    size_t close_paren = ping_output.find(')', open_paren);

    if (open_paren != std::string::npos && close_paren != std::string::npos) {
        std::string ip_port = ping_output.substr(open_paren + 1, close_paren - open_paren - 1);

        // Extract just the IP (remove port if present)
        size_t colon = ip_port.find(':');
        std::string ip = (colon != std::string::npos) ? ip_port.substr(0, colon) : ip_port;

        std::cout << "Resolved " << hostname << " to " << ip << std::endl;

        // Return IP with default port
        return ip + ":8080";
    }

    // Method 2: Try 'tailscale status' and parse
    std::string status_output = execute_tailscale_command("status");
    std::string resolved_ip = parse_ip_from_status(status_output, hostname);

    if (!resolved_ip.empty()) {
        std::cout << "Resolved " << hostname << " to " << resolved_ip << " (from status)" << std::endl;
        return resolved_ip + ":8080";
    }

    std::cerr << "Could not resolve Tailscale hostname: " << hostname << std::endl;
    return "";
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