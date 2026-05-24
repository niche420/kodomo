import Network
import Foundation
import Combine

class UDPReceiver: ObservableObject {
    private var listener: NWListener?
    var onPacketReceived: ((Data) -> Void)?
    
    init(port: UInt16) {
        let udpPort = NWEndpoint.Port(rawValue: port)!
        let params = NWParameters.udp
        listener = try? NWListener(using: params, on: udpPort)
    }
    
    func start() {
        listener?.newConnectionHandler = { [weak self] connection in
            connection.start(queue: .global())
            self?.receive(on: connection)
        }
        listener?.start(queue: .global())
    }
    
    private func receive(on connection: NWConnection) {
        connection.receive(minimumIncompleteLength: 1, maximumLength: 65535) { [weak self] data, _, isComplete, error in
            if let data = data {
                self?.onPacketReceived?(data)
            }
            if error == nil {
                self?.receive(on: connection)
            }
        }
    }
    
    func stop() {
        listener?.cancel()
        listener = nil
    }
}
