import Network
import Foundation

class UDPReceiver {
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
    
    func stop() {
        // cancel listener
    }
}
