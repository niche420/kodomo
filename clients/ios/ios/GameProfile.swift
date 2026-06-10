import Foundation

// Mirrors kd-shared/src/profile.rs exactly.
// These types are received as JSON over TCP during the handshake.

// --- Physical inputs ---

enum MouseAxis: String, Codable { case X, Y }

enum GamepadAxis: String, Codable {
    case LeftX, LeftY, RightX, RightY
}

enum GamepadTrigger: String, Codable {
    case Left, Right
}

enum GamepadButton: String, Codable {
    case South, East, West, North
    case LBumper, RBumper
    case LStick, RStick
    case DPadUp, DPadDown, DPadLeft, DPadRight
    case Start, Select
}

enum PhysicalInput: Codable {
    case Key(UInt16)
    case MButton(UInt8)
    case MAxis(MouseAxis)
    case GPadButton(GamepadButton)
    case GPadAxis(GamepadAxis)
    case GPadTrigger(GamepadTrigger)

    enum CodingKeys: String, CodingKey { case type, value }

    init(from decoder: Decoder) throws {
        let c = try decoder.container(keyedBy: CodingKeys.self)
        let type_ = try c.decode(String.self, forKey: .type)
        switch type_ {
        case "Key":            self = .Key(try c.decode(UInt16.self, forKey: .value))
        case "MouseButton":    self = .MButton(try c.decode(UInt8.self, forKey: .value))
        case "MouseAxis":      self = .MAxis(try c.decode(MouseAxis.self, forKey: .value))
        case "GamepadButton":  self = .GPadButton(try c.decode(GamepadButton.self, forKey: .value))
        case "GamepadAxis":    self = .GPadAxis(try c.decode(GamepadAxis.self, forKey: .value))
        case "GamepadTrigger": self = .GPadTrigger(try c.decode(GamepadTrigger.self, forKey: .value))
        default: throw DecodingError.dataCorruptedError(forKey: .type, in: c, debugDescription: "Unknown PhysicalInput type: \(type_)")
        }
    }

    func encode(to encoder: Encoder) throws {
        var c = encoder.container(keyedBy: CodingKeys.self)
        switch self {
        case .Key(let v):          try c.encode("Key", forKey: .type);           try c.encode(v, forKey: .value)
        case .MButton(let v):      try c.encode("MouseButton", forKey: .type);   try c.encode(v, forKey: .value)
        case .MAxis(let v):        try c.encode("MouseAxis", forKey: .type);     try c.encode(v, forKey: .value)
        case .GPadButton(let v):   try c.encode("GamepadButton", forKey: .type); try c.encode(v, forKey: .value)
        case .GPadAxis(let v):     try c.encode("GamepadAxis", forKey: .type);   try c.encode(v, forKey: .value)
        case .GPadTrigger(let v):  try c.encode("GamepadTrigger", forKey: .type);try c.encode(v, forKey: .value)
        }
    }
}

// --- Touch widgets ---

struct WidgetRect: Codable {
    let x, y, w, h: Float
}

enum JoystickMode: Codable {
    case GamepadStick(x: GamepadAxis, y: GamepadAxis)
    case MouseLook
    case Directional

    enum CodingKeys: String, CodingKey { case type, x, y }

    init(from decoder: Decoder) throws {
        let c = try decoder.container(keyedBy: CodingKeys.self)
        switch try c.decode(String.self, forKey: .type) {
        case "GamepadStick":
            self = .GamepadStick(
                x: try c.decode(GamepadAxis.self, forKey: .x),
                y: try c.decode(GamepadAxis.self, forKey: .y)
            )
        case "MouseLook":   self = .MouseLook
        case "Directional": self = .Directional
        default: throw DecodingError.dataCorruptedError(forKey: .type, in: c, debugDescription: "Unknown JoystickMode")
        }
    }

    func encode(to encoder: Encoder) throws {
        var c = encoder.container(keyedBy: CodingKeys.self)
        switch self {
        case .GamepadStick(let x, let y):
            try c.encode("GamepadStick", forKey: .type)
            try c.encode(x, forKey: .x)
            try c.encode(y, forKey: .y)
        case .MouseLook:   try c.encode("MouseLook", forKey: .type)
        case .Directional: try c.encode("Directional", forKey: .type)
        }
    }
}

enum TouchWidget: Codable {
    case Button(id: String, label: String, rect: WidgetRect)
    case DPad(id: String, rect: WidgetRect)
    case Joystick(id: String, rect: WidgetRect, mode: JoystickMode)
    case Trigger(id: String, label: String, rect: WidgetRect)

    enum CodingKeys: String, CodingKey { case kind, id, label, rect, mode }

    var id: String {
        switch self {
        case .Button(let id, _, _):   return id
        case .DPad(let id, _):        return id
        case .Joystick(let id, _, _): return id
        case .Trigger(let id, _, _):  return id
        }
    }

    init(from decoder: Decoder) throws {
        let c = try decoder.container(keyedBy: CodingKeys.self)
        switch try c.decode(String.self, forKey: .kind) {
        case "Button":
            self = .Button(
                id:    try c.decode(String.self,     forKey: .id),
                label: try c.decode(String.self,     forKey: .label),
                rect:  try c.decode(WidgetRect.self, forKey: .rect)
            )
        case "DPad":
            self = .DPad(
                id:   try c.decode(String.self,     forKey: .id),
                rect: try c.decode(WidgetRect.self, forKey: .rect)
            )
        case "Joystick":
            self = .Joystick(
                id:   try c.decode(String.self,       forKey: .id),
                rect: try c.decode(WidgetRect.self,   forKey: .rect),
                mode: try c.decode(JoystickMode.self, forKey: .mode)
            )
        case "Trigger":
            self = .Trigger(
                id:    try c.decode(String.self,     forKey: .id),
                label: try c.decode(String.self,     forKey: .label),
                rect:  try c.decode(WidgetRect.self, forKey: .rect)
            )
        default:
            throw DecodingError.dataCorruptedError(forKey: .kind, in: c, debugDescription: "Unknown TouchWidget kind")
        }
    }

    func encode(to encoder: Encoder) throws {
        var c = encoder.container(keyedBy: CodingKeys.self)
        switch self {
        case .Button(let id, let label, let rect):
            try c.encode("Button", forKey: .kind)
            try c.encode(id, forKey: .id)
            try c.encode(label, forKey: .label)
            try c.encode(rect, forKey: .rect)
        case .DPad(let id, let rect):
            try c.encode("DPad", forKey: .kind)
            try c.encode(id, forKey: .id)
            try c.encode(rect, forKey: .rect)
        case .Joystick(let id, let rect, let mode):
            try c.encode("Joystick", forKey: .kind)
            try c.encode(id, forKey: .id)
            try c.encode(rect, forKey: .rect)
            try c.encode(mode, forKey: .mode)
        case .Trigger(let id, let label, let rect):
            try c.encode("Trigger", forKey: .kind)
            try c.encode(id, forKey: .id)
            try c.encode(label, forKey: .label)
            try c.encode(rect, forKey: .rect)
        }
    }
}

// --- Actions, bindings, profile ---

struct Action: Codable {
    let id: String
    let label: String
    let input: PhysicalInput
}

// Renamed from Binding to WidgetBinding to avoid shadowing SwiftUI.Binding
struct WidgetBinding: Codable {
    let widget_slot: String
    let action_id: String
}

struct GameProfile: Codable {
    let game_title: String
    let widgets: [TouchWidget]
    let actions: [Action]
    let bindings: [WidgetBinding]

    func action(forSlot slot: String) -> Action? {
        guard let binding = bindings.first(where: { $0.widget_slot == slot }) else { return nil }
        return actions.first(where: { $0.id == binding.action_id })
    }
}
