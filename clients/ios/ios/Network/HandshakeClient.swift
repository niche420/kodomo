import Network
import Foundation

class HandshakeClient {
    private let host: String
    private let port: UInt16
    private let token: String
    var onConnected: ((UInt16) -> Void)?  // passes back the assigned input port
    var onFailed: (() -> Void)?
    private var connection: NWConnection?
    private var receiveBuffer = Data()

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
        connection.stateUpdateHandler = { [weak self] state in
            guard let self else { return }
            switch state {
            case .ready:
                let data = (self.token + "\n").data(using: .utf8)!
                connection.send(content: data, completion: .contentProcessed { error in
                    if error == nil {
                        self.readLine { line in self.handleOk(line: line) }
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

    // ── Private ───────────────────────────────────────────────────────────────

    private func readLine(completion: @escaping (String?) -> Void) {
        connection?.receive(minimumIncompleteLength: 1, maximumLength: 65535) { [weak self] data, _, _, error in
            guard let self else { return }
            guard error == nil, let data else {
                completion(nil)
                return
            }
            self.receiveBuffer.append(data)
            if let newlineRange = self.receiveBuffer.range(of: Data([UInt8(ascii: "\n")])) {
                let lineData = self.receiveBuffer[..<newlineRange.lowerBound]
                let line = String(data: lineData, encoding: .utf8)
                self.receiveBuffer.removeSubrange(...newlineRange.lowerBound)
                completion(line)
            } else {
                self.readLine(completion: completion)
            }
        }
    }

    private func handleOk(line: String?) {
        guard let line = line?.trimmingCharacters(in: .whitespacesAndNewlines) else {
            onFailed?()
            return
        }
        // Server sends "ok:<input_port>"
        guard line.hasPrefix("ok:"),
              let inputPort = UInt16(line.dropFirst(3)) else {
            onFailed?()
            return
        }
        onConnected?(inputPort)
    }
}
