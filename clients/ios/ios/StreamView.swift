import SwiftUI

struct StreamView: View {
    let params: ConnectParams
    
    var body: some View {
        Text("Streaming \(params.game)")
    }
}

#Preview {
    StreamView(params: ConnectParams(
        ip: "127.0.0.1",
        port: 12345,
        session: "1234567890abcdef",
        game: "Yakuza 3",
    ))
}
