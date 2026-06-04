import SwiftUI

struct GameListView: View {
    let server: PairedServer
    @State private var games: [ServerAPI.GameEntry] = []
    @State private var isLoading = false
    @State private var error: String? = nil
    @State private var streamTarget: GameTarget? = nil

    private var api: ServerAPI { ServerAPI(server: server) }

    var body: some View {
        Group {
            if isLoading {
                ProgressView("Loading games...")
            } else if let error {
                VStack(spacing: 12) {
                    Image(systemName: "exclamationmark.triangle")
                        .font(.largeTitle)
                        .foregroundStyle(.secondary)
                    Text(error)
                        .multilineTextAlignment(.center)
                    Button("Retry") { Task { await load() } }
                }
                .padding()
            } else if games.isEmpty {
                VStack(spacing: 12) {
                    Image(systemName: "gamecontroller")
                        .font(.system(size: 50))
                        .foregroundStyle(.secondary)
                    Text("No games registered on this server.")
                        .foregroundStyle(.secondary)
                }
            } else {
                List(games) { game in
                    HStack {
                        VStack(alignment: .leading) {
                            Text(game.title)
                                .font(.headline)
                            if let profile = game.active_profile {
                                Text("Profile: \(profile)")
                                    .font(.caption)
                                    .foregroundStyle(.secondary)
                            } else {
                                Text("No profile")
                                    .font(.caption)
                                    .foregroundStyle(.tertiary)
                            }
                        }
                        Spacer()
                        Button("Stream") {
                            streamTarget = GameTarget(server: server, game: game)
                        }
                        .buttonStyle(.borderedProminent)
                    }
                }
            }
        }
        .navigationTitle(server.name)
        .task { await load() }
        .fullScreenCover(item: $streamTarget) { target in
            StreamInitView(target: target)
        }
    }

    private func load() async {
        isLoading = true
        error = nil
        do {
            games = try await api.fetchGames()
        } catch {
            self.error = "Could not reach server: \(error.localizedDescription)"
        }
        isLoading = false
    }
}

/// Everything needed to initiate a stream.
struct GameTarget: Identifiable {
    let id = UUID()
    let server: PairedServer
    let game: ServerAPI.GameEntry
}

/// Handles the handshake then transitions to StreamView.
struct StreamInitView: View {
    let target: GameTarget
    @Environment(\.dismiss) private var dismiss
    @State private var connectParams: ConnectParams? = nil
    @State private var handshakeClient: HandshakeClient? = nil
    @State private var failed = false

    var body: some View {
        Group {
            if let params = connectParams {
                StreamView(params: params, handshakeClient: handshakeClient, profile: nil)
            } else if failed {
                VStack(spacing: 16) {
                    Text("Connection failed")
                        .font(.title2)
                    Button("Dismiss") { dismiss() }
                        .buttonStyle(.borderedProminent)
                }
            } else {
                VStack(spacing: 16) {
                    ProgressView()
                    Text("Connecting to \(target.game.title)...")
                        .foregroundStyle(.secondary)
                }
            }
        }
        .onAppear { startHandshake() }
    }

    private func startHandshake() {
        // The server needs to be in the Connect screen showing the QR for this game.
        // We initiate the handshake directly using the stored server credentials.
        // Session token is generated on the server side when it shows the connect screen —
        // for now we connect with an empty token; this will be replaced once the server
        // exposes a /stream endpoint to issue a session token over HTTP.
        let client = HandshakeClient(
            host: target.server.ip,
            port: target.server.handshakePort,
            token: ""
        )
        handshakeClient = client
        client.onConnected = {
            connectParams = ConnectParams(
                ip: target.server.ip,
                port: target.server.videoPort,
                session: "",
                game: target.game.title
            )
        }
        client.onFailed = {
            failed = true
        }
        client.connect()
    }
}
