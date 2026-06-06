import Foundation

struct InputEvent: Codable {
    let action_id: String
    let kind: InputEventKind
}

enum InputEventKind: Codable {
    case ButtonPress
    case ButtonRelease
    case Analog(Float)

    enum CodingKeys: String, CodingKey { case type, value }

    init(from decoder: Decoder) throws {
        let c = try decoder.container(keyedBy: CodingKeys.self)
        switch try c.decode(String.self, forKey: .type) {
        case "ButtonPress":   self = .ButtonPress
        case "ButtonRelease": self = .ButtonRelease
        case "Analog":        self = .Analog(try c.decode(Float.self, forKey: .value))
        default: throw DecodingError.dataCorruptedError(forKey: .type, in: c, debugDescription: "Unknown kind")
        }
    }

    func encode(to encoder: Encoder) throws {
        var c = encoder.container(keyedBy: CodingKeys.self)
        switch self {
        case .ButtonPress:    try c.encode("ButtonPress", forKey: .type)
        case .ButtonRelease:  try c.encode("ButtonRelease", forKey: .type)
        case .Analog(let v):
            try c.encode("Analog", forKey: .type)
            try c.encode(v, forKey: .value)
        }
    }
}
