import Network
import Foundation

class HandshakeClient {
    private let host: String
    private let port: UInt16
    private let token: String
    var onConnected: (() -> Void)?
    var onFailed: (() -> Void)?
    private var connection: NWConnection?
    
    init(host: String, port: UInt16, token: String) {
        self.host = host
        self.port = port
        self.token = token
    }
    
    func connect() {
        let connection = NWConnection(
            host: NWEndpoint.Host(host),
            port: NWEndpoint.Port(rawValue: port)!,
            using: .tcp
        )
        self.connection = connection
        connection.stateUpdateHandler = { state in
            switch state {
            case .ready:
                let data = (self.token + "\n").data(using: .utf8)!
                connection.send(content: data, completion: .contentProcessed { error in
                    if error == nil {
                        self.receive(on: connection)
                    } else {
                        self.onFailed?()
                    }
                })
            case .failed:
                self.onFailed?()
            default:
                break
            }
        }
        
        connection.start(queue: .global())
    }
    
    func sendReady() {
        let data = "ready\n".data(using: .utf8)!
        connection?.send(content: data, completion: .idempotent)
    }
    
    private func receive(on connection: NWConnection) {
        connection.receive(minimumIncompleteLength: 1, maximumLength: 10) { data, _, _, error in
            if let data = data, let response = String(data: data, encoding: .utf8) {
                if response.trimmingCharacters(in: .whitespacesAndNewlines) == "ok" {
                    self.onConnected?()
                } else {
                    self.onFailed?()
                }
            } else {
                self.onFailed?()
            }
            connection.cancel()
        }
    }
}
