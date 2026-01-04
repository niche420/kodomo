#pragma once

#include <string>
#include <vector>
#include <optional>

/**
 * Manages network connections, including Tailscale resolution
 */
class ConnectionManager {
public:
    ConnectionManager();
    ~ConnectionManager();

    /**
     * Resolve a Tailscale hostname to an IP address
     *
     * @param hostname Tailscale hostname (e.g., "my-server")
     * @return IP:port string or empty on failure
     */
    std::string resolve_tailscale_address(const std::string& hostname);

    /**
     * Check if Tailscale is available on this system
     */
    bool is_tailscale_available();

    /**
     * Get current Tailscale status
     */
    struct TailscaleStatus {
        bool is_connected = false;
        std::string ip_address;
        std::string hostname;
        std::vector<std::string> peer_hostnames;
    };

    std::optional<TailscaleStatus> get_tailscale_status();

private:
    std::string execute_tailscale_command(const std::string& command);
    std::string parse_ip_from_status(const std::string& status, const std::string& hostname);
};