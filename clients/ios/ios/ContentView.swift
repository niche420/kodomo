import SwiftUI

struct ContentView: View {
    @State private var connectParams: ConnectParams? = nil
    @State private var handshakeClient: HandshakeClient? = nil
    
    var body: some View {
        Group {
            if let params = connectParams {
                StreamView(params: params)
            } else {
                WaitingView()
            }
        }
        .onOpenURL { url in
            guard let components = URLComponents(url: url, resolvingAgainstBaseURL: false),
                  let host = components.host,
                  let port = components.port,
                  let queryItems = components.queryItems else { return }
            
            let session = queryItems.first(where: { $0.name == "session" })?.value ?? ""
            let game = queryItems.first(where: { $0.name == "game" })?.value ?? ""
            guard let handshake_port_str = queryItems.first(where: { $0.name == "handshake_port" })?.value, let handshake_port = UInt16(handshake_port_str) else { return }
            handshakeClient = HandshakeClient(host: host, port: handshake_port, token: session)
            handshakeClient?.onConnected = {
                connectParams = ConnectParams(ip: host, port: UInt16(port), session: session, game: game)
            }
            handshakeClient?.connect()
        }
    }
}

#Preview {
    ContentView()
}
