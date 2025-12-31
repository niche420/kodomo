#pragma once

#include <string>
#include <vector>
#include <cstdint>
#include "input_handler.hpp"

struct NetworkStats {
    double rtt_ms;
    double packet_loss_percent;
    uint64_t bytes_received;
    uint64_t packets_received;
    uint64_t packets_sent;
};

class NetworkClient {
public:
    NetworkClient();
    ~NetworkClient();
    
    bool connect(const std::string& address);
    void disconnect();
    
    bool has_data() const;
    std::vector<uint8_t> receive();
    void send_input(const InputEvent& event);
    
    NetworkStats get_stats() const { return stats_; }
    
private:
    int socket_fd_;
    bool connected_;
    NetworkStats stats_;
    std::string server_address_;
    uint16_t server_port_;
};