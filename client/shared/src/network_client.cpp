#include "network_client.hpp"
#include <iostream>
#include <cstring>

#ifdef _WIN32
    #include <winsock2.h>
    #include <ws2tcpip.h>
    #pragma comment(lib, "ws2_32.lib")
    typedef int socklen_t;
#else
    #include <sys/socket.h>
    #include <arpa/inet.h>
    #include <netinet/in.h>
    #include <unistd.h>
    #include <fcntl.h>
    #define INVALID_SOCKET -1
    #define SOCKET_ERROR -1
    #define closesocket close
#endif

NetworkClient::NetworkClient()
    : socket_fd_(INVALID_SOCKET)
    , connected_(false)
    , stats_{0}
    , server_port_(8080)
{
#ifdef _WIN32
    WSADATA wsa_data;
    WSAStartup(MAKEWORD(2, 2), &wsa_data);
#endif
}

NetworkClient::~NetworkClient() {
    disconnect();

#ifdef _WIN32
    WSACleanup();
#endif
}

bool NetworkClient::connect(const std::string& address) {
    // Parse address (format: "host:port")
    size_t colon_pos = address.find(':');
    if (colon_pos != std::string::npos) {
        server_address_ = address.substr(0, colon_pos);
        server_port_ = static_cast<uint16_t>(std::stoi(address.substr(colon_pos + 1)));
    } else {
        server_address_ = address;
    }

    // Create UDP socket
    socket_fd_ = socket(AF_INET, SOCK_DGRAM, IPPROTO_UDP);
    if (socket_fd_ == INVALID_SOCKET) {
        std::cerr << "Failed to create socket\n";
        return false;
    }

    // Set non-blocking mode
#ifdef _WIN32
    u_long mode = 1;
    ioctlsocket(socket_fd_, FIONBIO, &mode);
#else
    int flags = fcntl(socket_fd_, F_GETFL, 0);
    fcntl(socket_fd_, F_SETFL, flags | O_NONBLOCK);
#endif

    // Set socket options
    int reuse = 1;
    setsockopt(socket_fd_, SOL_SOCKET, SO_REUSEADDR,
               reinterpret_cast<const char*>(&reuse), sizeof(reuse));

    // Set receive buffer size
    int buffer_size = 1024 * 1024; // 1MB
    setsockopt(socket_fd_, SOL_SOCKET, SO_RCVBUF,
               reinterpret_cast<const char*>(&buffer_size), sizeof(buffer_size));

    // Setup server address
    sockaddr_in server_addr{};
    server_addr.sin_family = AF_INET;
    server_addr.sin_port = htons(server_port_);

    if (inet_pton(AF_INET, server_address_.c_str(), &server_addr.sin_addr) <= 0) {
        std::cerr << "Invalid server address\n";
        closesocket(socket_fd_);
        socket_fd_ = INVALID_SOCKET;
        return false;
    }

    // "Connect" UDP socket (sets default destination)
    if (::connect(socket_fd_, reinterpret_cast<sockaddr*>(&server_addr),
                  sizeof(server_addr)) == SOCKET_ERROR) {
        std::cerr << "Failed to connect socket\n";
        closesocket(socket_fd_);
        socket_fd_ = INVALID_SOCKET;
        return false;
    }

    connected_ = true;
    std::cout << "✓ Connected to " << server_address_ << ":" << server_port_ << "\n";

    // Send initial hello packet
    const char hello[] = "HELLO";
    send(socket_fd_, hello, sizeof(hello), 0);

    return true;
}

void NetworkClient::disconnect() {
    if (socket_fd_ != INVALID_SOCKET) {
        // Send goodbye packet
        const char goodbye[] = "GOODBYE";
        send(socket_fd_, goodbye, sizeof(goodbye), 0);

        closesocket(socket_fd_);
        socket_fd_ = INVALID_SOCKET;
    }
    connected_ = false;
}

bool NetworkClient::has_data() const {
    if (socket_fd_ == INVALID_SOCKET) {
        return false;
    }

    fd_set read_fds;
    FD_ZERO(&read_fds);
    FD_SET(socket_fd_, &read_fds);

    timeval timeout{};
    timeout.tv_sec = 0;
    timeout.tv_usec = 0; // Non-blocking

    int result = select(socket_fd_ + 1, &read_fds, nullptr, nullptr, &timeout);
    return result > 0 && FD_ISSET(socket_fd_, &read_fds);
}

std::vector<uint8_t> NetworkClient::receive() {
    if (socket_fd_ == INVALID_SOCKET) {
        return {};
    }

    // Buffer for receiving packets
    std::vector<uint8_t> buffer(65536); // Max UDP packet size

    int bytes_received = recv(socket_fd_,
                              reinterpret_cast<char*>(buffer.data()),
                              buffer.size(),
                              0);

    if (bytes_received > 0) {
        buffer.resize(bytes_received);

        // Update statistics
        stats_.packets_received++;
        stats_.bytes_received += bytes_received;

        return buffer;
    }

    return {};
}

void NetworkClient::send_input(const InputEvent& event) {
    if (socket_fd_ == INVALID_SOCKET || !connected_) {
        return;
    }

    // Simple binary serialization of input event
    std::vector<uint8_t> data;
    data.reserve(32);

    // Header: [type:1]
    data.push_back(static_cast<uint8_t>(event.type));

    // Serialize based on type
    switch (event.type) {
        case InputEvent::KEYBOARD: {
            // [keycode:4][pressed:1][timestamp:8]
            uint32_t keycode = event.keycode;
            data.insert(data.end(),
                       reinterpret_cast<uint8_t*>(&keycode),
                       reinterpret_cast<uint8_t*>(&keycode) + 4);
            data.push_back(event.pressed ? 1 : 0);

            uint64_t ts = event.timestamp;
            data.insert(data.end(),
                       reinterpret_cast<uint8_t*>(&ts),
                       reinterpret_cast<uint8_t*>(&ts) + 8);
            break;
        }

        case InputEvent::MOUSE_MOVE: {
            // [x:4][y:4][timestamp:8]
            int32_t x = event.mouse_x;
            int32_t y = event.mouse_y;
            data.insert(data.end(),
                       reinterpret_cast<uint8_t*>(&x),
                       reinterpret_cast<uint8_t*>(&x) + 4);
            data.insert(data.end(),
                       reinterpret_cast<uint8_t*>(&y),
                       reinterpret_cast<uint8_t*>(&y) + 4);

            uint64_t ts = event.timestamp;
            data.insert(data.end(),
                       reinterpret_cast<uint8_t*>(&ts),
                       reinterpret_cast<uint8_t*>(&ts) + 8);
            break;
        }

        case InputEvent::MOUSE_BUTTON: {
            // [x:4][y:4][button:1][pressed:1][timestamp:8]
            int32_t x = event.mouse_x;
            int32_t y = event.mouse_y;
            data.insert(data.end(),
                       reinterpret_cast<uint8_t*>(&x),
                       reinterpret_cast<uint8_t*>(&x) + 4);
            data.insert(data.end(),
                       reinterpret_cast<uint8_t*>(&y),
                       reinterpret_cast<uint8_t*>(&y) + 4);
            data.push_back(event.mouse_button);
            data.push_back(event.pressed ? 1 : 0);

            uint64_t ts = event.timestamp;
            data.insert(data.end(),
                       reinterpret_cast<uint8_t*>(&ts),
                       reinterpret_cast<uint8_t*>(&ts) + 8);
            break;
        }
    }

    // Send packet
    int bytes_sent = send(socket_fd_,
                         reinterpret_cast<const char*>(data.data()),
                         data.size(),
                         0);

    if (bytes_sent > 0) {
        stats_.packets_sent++;
    }
}