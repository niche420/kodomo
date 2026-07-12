import SwiftUI

struct ServerView: View {
    @EnvironmentObject var serverStore: ServerStore
    @State private var showScanner = false
    @State private var selectedServer: PairedServer? = nil

    var body: some View {
        NavigationStack {
            Group {
                if serverStore.servers.isEmpty {
                    emptyState
                } else {
                    serverList
                }
            }
            .navigationTitle("Kodomo")
            .toolbar {
                ToolbarItem(placement: .primaryAction) {
                    Button(action: { showScanner = true }) {
                        Label("Pair Server", systemImage: "qrcode.viewfinder")
                    }
                }
            }
            .sheet(isPresented: $showScanner) {
                ScannerSheet { server in
                    serverStore.add(server)
                    showScanner = false
                }
            }
            .navigationDestination(item: $selectedServer) { server in
                GameListView(server: server)
            }
        }
    }

    private var emptyState: some View {
        VStack(spacing: 16) {
            Image(systemName: "display").font(.system(size: 60)).foregroundStyle(.secondary)
            Text("No servers paired").font(.title2)
            Text("Open Kodomo on your PC and tap the QR button to pair.")
                .multilineTextAlignment(.center).foregroundStyle(.secondary).padding(.horizontal, 40)
            Button("Scan QR Code") { showScanner = true }.buttonStyle(.borderedProminent)
        }
    }

    private var serverList: some View {
        List {
            ForEach(serverStore.servers) { server in
                Button(action: { selectedServer = server }) {
                    HStack {
                        Image(systemName: "display").foregroundStyle(.secondary)
                        VStack(alignment: .leading) {
                            Text(server.name).font(.headline)
                            Text(server.ip).font(.caption).foregroundStyle(.secondary)
                        }
                        Spacer()
                        Image(systemName: "chevron.right").foregroundStyle(.tertiary)
                    }
                }
                .foregroundStyle(.primary)
            }
            .onDelete(perform: serverStore.remove)
        }
    }
}

struct ScannerSheet: View {
    var onPaired: (PairedServer) -> Void
    @Environment(\.dismiss) private var dismiss

    var body: some View {
        NavigationStack {
            QRScannerView { result in
                let server = PairedServer(
                    ip: result.ip,
                    httpPort: result.httpPort,
                    handshakePort: result.handshakePort,
                    videoPort: result.videoPort
                )
                onPaired(server)
            }
            .navigationTitle("Scan QR Code")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .cancellationAction) {
                    Button("Cancel") { dismiss() }
                }
            }
        }
    }
}
