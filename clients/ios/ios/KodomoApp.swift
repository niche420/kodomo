import SwiftUI

@main
struct KodomoApp: App {
    @StateObject private var serverStore = ServerStore()

    var body: some Scene {
        WindowGroup {
            ContentView()
                .environmentObject(serverStore)
        }
    }
}
