# Kodomo

A high-performance, low-latency game streaming system written in Rust with cross-platform support. Kodomo enables real-time screen capture, hardware-accelerated video encoding, and network streaming with adaptive quality optimization.

## Features

### Server
- **Screen Capture**: Platform-native screen capture (Windows, macOS, Linux)
- **Hardware Encoding**: GPU-accelerated encoding (NVENC, VideoToolbox)
- **Multiple Transports**: UDP and WebRTC networking protocols
- **Input Handling**: Remote keyboard, mouse, and gamepad input
- **Metrics & Monitoring**: Real-time performance metrics via HTTP endpoint
- **Adaptive Streaming**: ML-based bitrate and quality optimization

### Desktop Client
- **Cross-Platform**: CMake-based C++ client for Windows, macOS, and Linux
- **Hardware Decoding**: GPU-accelerated video decoding
- **Low-Latency Rendering**: OpenGL/Vulkan rendering pipeline
- **Input Forwarding**: Sends user input back to server
- **Configurable**: Fullscreen mode, custom resolutions, server connection

### ML Optimizer
- **Bitrate Prediction**: LSTM neural network for optimal bitrate
- **Quality Optimization**: DQN reinforcement learning agent
- **Real-time Adaptation**: WebSocket integration with streaming server
- **Online Learning**: Optional real-time model training

## Installation

### Prerequisites

**Server (Rust):**
- Rust 1.75+ (2024 edition)
- CMake 3.20+
- Platform-specific dependencies:
  - Windows: Windows SDK
  - macOS: Xcode Command Line Tools
  - Linux: X11/Wayland dev packages

**Desktop Client (C++):**
- CMake 3.20+
- C++17 compiler
- vcpkg (for dependencies)
- FFmpeg, SDL2, OpenGL

**ML Optimizer (Python):**
- Python 3.8+
- PyTorch, NumPy, WebSocket-client

### Building

#### Server

```bash
# Build the streaming server
cargo build --release -p kd-server

# Run the server
cargo run --release -p kd-server -- --config example.toml
```

#### Desktop Client

```bash
cd clients/desktop

# Install dependencies (Windows)
.\install_deps.bat

# Install dependencies (Linux/macOS)
./install_deps.sh

# Build
cmake --preset release
cmake --build build/release

# Run
./build/release/kodomo-client --server 127.0.0.1:8080
```

#### ML Optimizer

```bash
cd python
python -m venv venv
source venv/bin/activate  # Windows: venv\Scripts\activate
pip install -r requirements.txt
pip install -e .

# Run optimizer
python -m optimizer.optimizer --server ws://localhost:9090/metrics
```

## Configuration

### Server Configuration (TOML)

```toml
[server]
name = "My Streaming Server"
max_clients = 4
metrics_port = 9090

[video]
width = 1920
height = 1080
fps = 60
bitrate_kbps = 10000
codec = "h264"
hw_accel = true

[network]
transport = "udp"
port = 8080
bind_address = "0.0.0.0"

[input]
keyboard_enabled = true
mouse_enabled = true
gamepad_enabled = true
```

See `crates/kd-server/example.toml` for full configuration options.

### Client Options

```bash
kodomo-client [options]
  --server <address>    Server address (default: 127.0.0.1:8080)
  --width <width>       Window width (default: 1920)
  --height <height>     Window height (default: 1080)
  --fullscreen          Start in fullscreen mode
  --help                Show help message
```

## Usage

### Basic Streaming

1. Start the server:
```bash
cargo run --release -p kd-server -- --config config.toml
```

2. Connect with desktop client:
```bash
kodomo-client --server 127.0.0.1:8080 --fullscreen
```

3. (Optional) Run ML optimizer:
```bash
python -m optimizer.optimizer --server ws://localhost:9090/metrics
```

### Development

```bash
# Run tests
cargo test --workspace

# Check code
cargo clippy --workspace

# Format code
cargo fmt --workspace

# Build all crates
cargo build --workspace
```

## Performance

- **Latency**: <20ms end-to-end on local network
- **Encoding**: Hardware acceleration via NVENC (NVIDIA) / VideoToolbox (Apple)
- **Frame Rate**: Up to 120 FPS (hardware dependent)
- **Resolution**: Supports up to 4K streaming
- **Bitrate**: Adaptive 5-50 Mbps based on network conditions

## License

MIT License - See LICENSE file for details

## Contributing

idgaf

## Roadmap

- [ ] Audio streaming support
- [ ] WebRTC transport optimization
- [ ] Multiple monitor support
- [ ] Web-based client
- [ ] Hardware decoding on client
- [ ] Mobile client (iOS/Android)
