import SwiftUI

struct ControlOverlayView: View {
    let profile: GameProfile
    let sender: UDPSender

    var body: some View {
        GeometryReader { geo in
            ZStack {
                ForEach(profile.widgets.indices, id: \.self) { i in
                    widgetView(profile.widgets[i], geo: geo)
                }
            }
        }
    }

    @ViewBuilder
    private func widgetView(_ widget: TouchWidget, geo: GeometryProxy) -> some View {
        switch widget {
        case .Button(let id, let label, let rect):
            ButtonWidgetView(
                id: id,
                label: label,
                frame: rect.toFrame(in: geo),
                sender: sender,
                profile: profile
            )
        case .DPad(let id, let rect):
            DPadWidgetView(
                id: id,
                frame: rect.toFrame(in: geo),
                sender: sender,
                profile: profile
            )
        case .Joystick(let id, let rect, let mode):
            JoystickWidgetView(
                id: id,
                frame: rect.toFrame(in: geo),
                mode: mode,
                sender: sender,
                profile: profile
            )
        case .Trigger(let id, let label, let rect):
            TriggerWidgetView(
                id: id,
                label: label,
                frame: rect.toFrame(in: geo),
                sender: sender,
                profile: profile
            )
        }
    }
}

// ─── Rect → CGRect ────────────────────────────────────────────────────────────

extension WidgetRect {
    func toFrame(in geo: GeometryProxy) -> CGRect {
        let w = geo.size.width
        let h = geo.size.height
        let width  = CGFloat(self.w) * w
        let height = CGFloat(self.h) * h
        return CGRect(
            x: CGFloat(self.x) * w - width  / 2,
            y: CGFloat(self.y) * h - height / 2,
            width:  width,
            height: height
        )
    }
}

// ─── Helpers ──────────────────────────────────────────────────────────────────

extension ControlOverlayView {
    func send(slot: String, kind: InputEventKind) {
        guard let action = profile.action(forSlot: slot) else { return }
        sender.send(InputEvent(action_id: action.id, kind: kind))
    }
}

// ─── Button ───────────────────────────────────────────────────────────────────

struct ButtonWidgetView: View {
    let id: String
    let label: String
    let frame: CGRect
    let sender: UDPSender
    let profile: GameProfile
    @State private var pressed = false

    var body: some View {
        Circle()
            .fill(.white.opacity(pressed ? 0.5 : 0.25))
            .overlay(
                Text(label)
                    .font(.system(size: frame.width * 0.3, weight: .bold))
                    .foregroundStyle(.white)
            )
            .frame(width: frame.width, height: frame.height)
            .position(x: frame.midX, y: frame.midY)
            .simultaneousGesture(
                DragGesture(minimumDistance: 0)
                    .onChanged { _ in
                        if !pressed {
                            pressed = true
                            fire(slot: id, kind: .ButtonPress)
                        }
                    }
                    .onEnded { _ in
                        pressed = false
                        fire(slot: id, kind: .ButtonRelease)
                    }
            )
    }

    private func fire(slot: String, kind: InputEventKind) {
        guard let action = profile.action(forSlot: slot) else { return }
        sender.send(InputEvent(action_id: action.id, kind: kind))
    }
}

// ─── DPad ─────────────────────────────────────────────────────────────────────

struct DPadWidgetView: View {
    let id: String
    let frame: CGRect
    let sender: UDPSender
    let profile: GameProfile
    @State private var activeDirection: String? = nil

    var body: some View {
        ZStack {
            // Background
            Circle()
                .fill(.white.opacity(0.15))
            // Direction indicators
            ForEach(["up", "down", "left", "right"], id: \.self) { dir in
                directionArrow(dir)
            }
        }
        .frame(width: frame.width, height: frame.height)
        .position(x: frame.midX, y: frame.midY)
        .simultaneousGesture(
            DragGesture(minimumDistance: 0)
                .onChanged { value in
                    let dir = direction(for: value.location)
                    if dir != activeDirection {
                        // Release old direction
                        if let old = activeDirection {
                            fire(slot: "\(id)_\(old)", kind: .ButtonRelease)
                        }
                        // Press new direction
                        activeDirection = dir
                        if let dir {
                            fire(slot: "\(id)_\(dir)", kind: .ButtonPress)
                        }
                    }
                }
                .onEnded { _ in
                    if let dir = activeDirection {
                        fire(slot: "\(id)_\(dir)", kind: .ButtonRelease)
                    }
                    activeDirection = nil
                }
        )
    }

    private func directionArrow(_ dir: String) -> some View {
        let isActive = activeDirection == dir
        let angle: Double = ["up": -90, "right": 0, "down": 90, "left": 180][dir]!
        return Image(systemName: "arrowtriangle.right.fill")
            .rotationEffect(.degrees(angle))
            .foregroundStyle(.white.opacity(isActive ? 0.9 : 0.4))
            .offset(arrowOffset(dir))
    }

    private func arrowOffset(_ dir: String) -> CGSize {
        let d = frame.width * 0.28
        switch dir {
        case "up":    return CGSize(width: 0, height: -d)
        case "down":  return CGSize(width: 0, height:  d)
        case "left":  return CGSize(width: -d, height: 0)
        case "right": return CGSize(width:  d, height: 0)
        default:      return .zero
        }
    }

    private func direction(for point: CGPoint) -> String? {
        let center = CGPoint(x: frame.width / 2, y: frame.height / 2)
        let dx = point.x - center.x
        let dy = point.y - center.y
        let deadzone = frame.width * 0.15
        guard sqrt(dx*dx + dy*dy) > deadzone else { return nil }
        if abs(dx) > abs(dy) {
            return dx > 0 ? "right" : "left"
        } else {
            return dy > 0 ? "down" : "up"
        }
    }

    private func fire(slot: String, kind: InputEventKind) {
        guard let action = profile.action(forSlot: slot) else { return }
        sender.send(InputEvent(action_id: action.id, kind: kind))
    }
}

// ─── Joystick ─────────────────────────────────────────────────────────────────

struct JoystickWidgetView: View {
    let id: String
    let frame: CGRect
    let mode: JoystickMode
    let sender: UDPSender
    let profile: GameProfile
    @State private var thumbOffset: CGSize = .zero
    @State private var activeDirection: String? = nil
    // For MouseLook — tracks last position to compute delta
    @State private var lastLocation: CGPoint? = nil

    private let maxRadius: CGFloat = 40

    var body: some View {
        ZStack {
            // Base
            Circle()
                .fill(.white.opacity(0.15))
            // Thumb nub
            Circle()
                .fill(.white.opacity(0.5))
                .frame(width: frame.width * 0.35, height: frame.height * 0.35)
                .offset(thumbOffset)
        }
        .frame(width: frame.width, height: frame.height)
        .position(x: frame.midX, y: frame.midY)
        .simultaneousGesture(
            DragGesture(minimumDistance: 0)
                .onChanged { value in handleDrag(value) }
                .onEnded { _ in handleEnd() }
        )
    }

    private func handleDrag(_ value: DragGesture.Value) {
        let dx = value.translation.width
        let dy = value.translation.height
        let dist = sqrt(dx*dx + dy*dy)
        let clamped = min(dist, maxRadius)
        let angle = atan2(dy, dx)
        let clampedX = clamped * cos(angle)
        let clampedY = clamped * sin(angle)
        thumbOffset = CGSize(width: clampedX, height: clampedY)

        let nx = Float(clampedX / maxRadius)
        let ny = Float(clampedY / maxRadius)

        switch mode {
        case .GamepadStick:
            fire(slot: "\(id)_x", kind: .Analog(nx))
            fire(slot: "\(id)_y", kind: .Analog(ny))

        case .MouseLook:
            if let last = lastLocation {
                let deltaX = Float(value.location.x - last.x)
                let deltaY = Float(value.location.y - last.y)
                fire(slot: "\(id)_x", kind: .Analog(deltaX))
                fire(slot: "\(id)_y", kind: .Analog(deltaY))
            }
            lastLocation = value.location

        case .Directional:
            let dir = snapDirection(nx: nx, ny: ny)
            if dir != activeDirection {
                if let old = activeDirection {
                    fire(slot: "\(id)_\(old)", kind: .ButtonRelease)
                }
                activeDirection = dir
                if let dir {
                    fire(slot: "\(id)_\(dir)", kind: .ButtonPress)
                }
            }
        }
    }

    private func handleEnd() {
        thumbOffset = .zero
        lastLocation = nil

        switch mode {
        case .GamepadStick:
            fire(slot: "\(id)_x", kind: .Analog(0))
            fire(slot: "\(id)_y", kind: .Analog(0))
        case .MouseLook:
            break
        case .Directional:
            if let dir = activeDirection {
                fire(slot: "\(id)_\(dir)", kind: .ButtonRelease)
            }
            activeDirection = nil
        }
    }

    private func snapDirection(nx: Float, ny: Float) -> String? {
        let deadzone: Float = 0.2
        guard sqrt(nx*nx + ny*ny) > deadzone else { return nil }
        if abs(nx) > abs(ny) {
            return nx > 0 ? "right" : "left"
        } else {
            return ny > 0 ? "down" : "up"
        }
    }

    private func fire(slot: String, kind: InputEventKind) {
        guard let action = profile.action(forSlot: slot) else { return }
        sender.send(InputEvent(action_id: action.id, kind: kind))
    }
}

// ─── Trigger ──────────────────────────────────────────────────────────────────

struct TriggerWidgetView: View {
    let id: String
    let label: String
    let frame: CGRect
    let sender: UDPSender
    let profile: GameProfile
    @State private var value: Float = 0

    var body: some View {
        ZStack(alignment: .bottom) {
            RoundedRectangle(cornerRadius: 8)
                .fill(.white.opacity(0.15))
            RoundedRectangle(cornerRadius: 8)
                .fill(.white.opacity(0.4))
                .frame(height: frame.height * CGFloat(value))
            Text(label)
                .font(.system(size: frame.width * 0.3, weight: .bold))
                .foregroundStyle(.white)
        }
        .frame(width: frame.width, height: frame.height)
        .position(x: frame.midX, y: frame.midY)
        .simultaneousGesture(
            DragGesture(minimumDistance: 0)
                .onChanged { v in
                    let raw = Float(-v.translation.height / frame.height)
                    value = max(0, min(1, raw))
                    fire(slot: id, kind: .Analog(value))
                }
                .onEnded { _ in
                    value = 0
                    fire(slot: id, kind: .Analog(0))
                }
        )
    }

    private func fire(slot: String, kind: InputEventKind) {
        guard let action = profile.action(forSlot: slot) else { return }
        sender.send(InputEvent(action_id: action.id, kind: kind))
    }
}
