# Kodomo

Stream PC games to your phone over LAN.

![server-connect.png](screenshots/server-connect.png)

## Architecture

```
kd-server   - Windows capture, encode, and stream server (Rust + egui)
kd-shared   - RTP packetization, depacketization, session types (Rust)
kd-ffi      - C exports of kd-shared for iOS (Rust)
kd-input    - Windows kernel-mode virtual HID driver (C / KMDF)
clients/ios - iOS receiver, VideoToolbox decode, Metal render (Swift)
```

## Pipeline

```
SERVER
  DXGI/WGC Capture -> FrameSlot -> FFmpeg/NVENC Encode -> PacketQueue -> UDP Send
  UDP Recv <- InputEvent (JSON) <- kd-shared dispatch <- DriverInjector IOCTL

        ^ UDP video                    | UDP input
        |                              v

CLIENT
  UDPReceiver -> kd-ffi Depacketizer -> VideoToolbox Decode -> Metal Render
  ControlOverlayView -> UDPSender -> InputEvent (JSON)
```

## Connection Flow

```
1. Server UI: user clicks a game -> navigates to Connect screen
   OR
   iOS: user taps "Stream" on GameListView -> POST /stream -> server navigates to Connect screen

2. Connect screen spawns HandshakeListener on TCP handshake_port

3. Server displays QR code encoding:
   kodomo://<ip>:<video_port>?session=<token>&game=<title>
                              &handshake_port=<p>&http_port=<p>&input_port=<p>

4. iOS scans QR (or already has params from /stream response)
   -> TCP handshake: send token, receive "ok", send "ready"

5. Server HandshakeListener records client IP, fires PipelineStart event

6. Pipeline starts 4 threads:
   capture | encode+packetize | network send | input receive
```

## Requirements

### Server
- Windows 11, NVIDIA GPU (NVENC) or software fallback via libx264
- LLVM installed
- vcpkg with `ffmpeg[nvcodec]:x64-windows` or manual FFmpeg install
- kd-input.sys installed and running (see kd-input/)

### Client
- Xcode 26+
- iOS 26+ device

## Build

### Server
Copy `.cargo/config.toml.example` to `.cargo/config.toml` and fill in your paths:

```toml
[env]
VCPKG_ROOT = "C:\\path\\to\\vcpkg"    # remove if using FFMPEG_DIR
FFMPEG_DIR = "C:\\path\\to\\ffmpeg"   # remove if using VCPKG_ROOT
LIBCLANG_PATH = "C:\\Program Files\\LLVM\\bin"
```

Then:
```
cargo build
```

### Driver (kd-input)
Open `kd-input/kd-input.slnx` in Visual Studio with the WDK installed.
Build for x64 or ARM64. Then install with:
```
devcon install kd-input.inf KdInput\VirtualHID
```
The driver must be test-signed or deployed via WHQL. On a dev machine, enable
test signing with `bcdedit /set testsigning on` and restart.

### Client
1. `cargo build --release -p kd-ffi --target aarch64-apple-ios`
2. Copy `target/aarch64-apple-ios/release/libkd_ffi.a` to `clients/ios/kodomo/`
3. Copy `kd-ffi/target/kd-ffi.h` to `clients/ios/kodomo/ffi/`
4. Open `clients/ios` in Xcode, set your development team, install to device.

## Run
1. Ensure kd-input.sys is installed and the device appears in Device Manager.
2. Run `kd-server`.
3. Click **+ Add Game** and select a game `.exe`.
4. Click the game title to go to the Connect screen.
5. Scan the QR code with the Kodomo iOS app.
6. The app completes the TCP handshake and streaming begins.

Alternatively, from the iOS app:
1. Pair your server by scanning the server's pairing QR (from the toolbar button).
2. Open the game list, tap **Profiles** to create and configure a control layout.
3. Tap **Stream** to start streaming.

## Status

- Milestone 1 - Video streaming **COMPLETE**
- Milestone 2 - Input streaming **IN PROGRESS**
  - [x] kd-input.sys virtual HID driver (gamepad, keyboard, mouse)
  - [x] DriverInjector IOCTL wrappers
  - [x] Input dispatch (action_id -> PhysicalInput -> injector call)
  - [x] iOS control overlay widgets (Button, DPad, Joystick, Trigger)
  - [x] UDP input sender on iOS
  - [x] UDP input receiver + JSON deserialize on server
  - [x] Profile editor (iOS, HTTP-backed)
  - [ ] Window capture (currently captures full monitor, needs per-window WGC)
  - [ ] QR vs /stream flow unification (Connect screen race condition)
  - [ ] input_port missing from QR URL parse on iOS
  - [ ] MouseLook joystick sensitivity tuning
  - [ ] Gamepad trigger Y-axis inversion for some games