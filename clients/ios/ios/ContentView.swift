import SwiftUI

struct ContentView: View {
    @State private var connectParams: ConnectParams? = nil
    
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
            
            connectParams = ConnectParams(ip: host, port: UInt16(port), session: session, game: game)
        }
    }
}

#Preview {
    ContentView()
}
