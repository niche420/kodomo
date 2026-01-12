#include "network_client.hpp"
#include <iostream>
#include <cstring>

#ifdef __ANDROID__
#include <android/log.h>
#define LOGI(...) __android_log_print(ANDROID_LOG_INFO, "NetworkClient", __VA_ARGS__)
#define LOGE(...) __android_log_print(ANDROID_LOG_ERROR, "NetworkClient", __VA_ARGS__)
#else
#define LOGI(...) std::cout << __VA_ARGS__ << std::endl
#define LOGE(...) std::cerr << __VA_ARGS__ << std::endl
#endif

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
    LOGI("NetworkClient created");
}

NetworkClient::~NetworkClient() {
    disconnect();

#ifdef _WIN32
    WSACleanup();
#endif
}

bool NetworkClient::connect(const std::string& address) {
    LOGI("╔════════════════════════════════════════╗");
    LOGI("║   NetworkClient::connect START         ║");
    LOGI("╚════════════════════════════════════════╝");
    LOGI("Input address: %s", address.c_str());

    // Parse address - format should be "IP:PORT"
    size_t colon_pos = address.find_last_of(':');
    if (colon_pos == std::string::npos) {
        LOGE("❌ Invalid address format (no port): %s", address.c_str());
        LOGE("Expected format: IP:PORT (e.g., 192.168.1.100:8080)");
        return false;
    }

    server_address_ = address.substr(0, colon_pos);
    std::string port_str = address.substr(colon_pos + 1);

    LOGI("Parsed IP: %s", server_address_.c_str());
    LOGI("Parsed port string: %s", port_str.c_str());

    try {
        server_port_ = static_cast<uint16_t>(std::stoi(port_str));
        LOGI("Parsed port number: %u", server_port_);
    } catch (...) {
        LOGE("❌ Invalid port number: %s", port_str.c_str());
        return false;
    }

    // Create socket address structure
    sockaddr_in server_addr{};
    server_addr.sin_family = AF_INET;
    server_addr.sin_port = htons(server_port_);

    LOGI("Converting IP address to binary...");
    int pton_result = inet_pton(AF_INET, server_address_.c_str(), &server_addr.sin_addr);
    if (pton_result <= 0) {
        if (pton_result == 0) {
            LOGE("❌ inet_pton: Invalid IP address format: %s", server_address_.c_str());
        } else {
            LOGE("❌ inet_pton error: %s", strerror(errno));
        }
        return false;
    }
    LOGI("✅ IP address converted successfully");

    // Create socket
    LOGI("Creating UDP socket...");
    socket_fd_ = socket(AF_INET, SOCK_DGRAM, IPPROTO_UDP);
    if (socket_fd_ == INVALID_SOCKET) {
        LOGE("❌ socket() failed: %s", strerror(errno));
        return false;
    }
    LOGI("✅ Socket created: fd=%d", socket_fd_);

    // Set non-blocking mode
    LOGI("Setting socket to non-blocking...");
#ifdef _WIN32
    u_long mode = 1;
    if (ioctlsocket(socket_fd_, FIONBIO, &mode) != 0) {
        LOGE("❌ ioctlsocket failed: %d", WSAGetLastError());
        closesocket(socket_fd_);
        socket_fd_ = INVALID_SOCKET;
        return false;
    }
#else
    int flags = fcntl(socket_fd_, F_GETFL, 0);
    if (flags == -1) {
        LOGE("❌ fcntl F_GETFL failed: %s", strerror(errno));
        closesocket(socket_fd_);
        socket_fd_ = INVALID_SOCKET;
        return false;
    }
    if (fcntl(socket_fd_, F_SETFL, flags | O_NONBLOCK) == -1) {
        LOGE("❌ fcntl F_SETFL failed: %s", strerror(errno));
        closesocket(socket_fd_);
        socket_fd_ = INVALID_SOCKET;
        return false;
    }
#endif
    LOGI("✅ Socket set to non-blocking");

    // Connect socket
    LOGI("Connecting to %s:%u...", server_address_.c_str(), server_port_);
    int connect_result = ::connect(socket_fd_,
                                   reinterpret_cast<sockaddr*>(&server_addr),
                                   sizeof(server_addr));

    if (connect_result == SOCKET_ERROR) {
#ifdef _WIN32
        int err = WSAGetLastError();
        if (err != WSAEWOULDBLOCK && err != WSAEINPROGRESS) {
            LOGE("❌ connect() failed: error code %d", err);
            closesocket(socket_fd_);
            socket_fd_ = INVALID_SOCKET;
            return false;
        }
#else
        int err = errno;
        if (err != EINPROGRESS && err != EAGAIN && err != EWOULDBLOCK) {
            LOGE("❌ connect() failed: %s (errno=%d)", strerror(err), err);
            closesocket(socket_fd_);
            socket_fd_ = INVALID_SOCKET;
            return false;
        }
#endif
        LOGI("⏳ Connection in progress (non-blocking socket)");
    } else {
        LOGI("✅ Connected immediately");
    }

    connected_ = true;

    LOGI("╔════════════════════════════════════════╗");
    LOGI("║   NetworkClient::connect SUCCESS       ║");
    LOGI("║   Connected to %s:%u", server_address_.c_str(), server_port_);
    LOGI("╚════════════════════════════════════════╝");

    return true;
}

void NetworkClient::disconnect() {
    if (socket_fd_ != INVALID_SOCKET) {
        LOGI("Disconnecting...");

        // Send goodbye packet
        const char goodbye[] = "GOODBYE";
        send(socket_fd_, goodbye, sizeof(goodbye), 0);

        closesocket(socket_fd_);
        socket_fd_ = INVALID_SOCKET;
        LOGI("✅ Disconnected");
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