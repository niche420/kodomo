import Network
import Foundation

/// Sends UDP packets to the server's input port.
class UDPSender {
    private var connection: NWConnection?

    init(host: String, port: UInt16) {
        connection = NWConnection(
            host: NWEndpoint.Host(host),
            port: NWEndpoint.Port(rawValue: port)!,
            using: .udp
        )
    }

    func start() {
        connection?.start(queue: .global())
    }

    func stop() {
        connection?.cancel()
        connection = nil
    }

    func send(_ event: InputEvent) {
        guard let data = try? JSONEncoder().encode(event) else { return }
        connection?.send(content: data, completion: .idempotent)
    }
}
