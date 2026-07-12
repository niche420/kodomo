import SwiftUI

struct ContentView: View {
    var body: some View {
        ServerView()
    }
}

#Preview {
    ContentView()
        .environmentObject(ServerStore())
}
