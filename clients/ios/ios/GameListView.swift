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

struct GameTarget: Identifiable {
    let id = UUID()
    let server: PairedServer
    let game: ServerAPI.GameEntry
}

struct StreamInitView: View {
    let target: GameTarget
    @Environment(\.dismiss) private var dismiss
    @State private var connectParams: ConnectParams? = nil
    @State private var handshakeClient: HandshakeClient? = nil
    @State private var profile: GameProfile? = nil
    @State private var failed = false
    @State private var errorMessage: String? = nil

    private var api: ServerAPI { ServerAPI(server: target.server) }

    var body: some View {
        Group {
            if let params = connectParams {
                StreamView(params: params, handshakeClient: handshakeClient, profile: profile)
            } else if failed {
                VStack(spacing: 16) {
                    Image(systemName: "xmark.circle")
                        .font(.system(size: 50))
                        .foregroundStyle(.red)
                    Text("Connection failed")
                        .font(.title2)
                    if let msg = errorMessage {
                        Text(msg)
                            .font(.caption)
                            .foregroundStyle(.secondary)
                            .multilineTextAlignment(.center)
                            .padding(.horizontal)
                    }
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
        .task { await startStream() }
    }

    private func startStream() async {
        // 1. Ask the server to prepare a session token for this game
        let streamResponse: StreamResponse
        do {
            streamResponse = try await requestStream()
        } catch {
            errorMessage = error.localizedDescription
            failed = true
            return
        }

        // 2. Fetch the active profile for this game (non-fatal if missing)
        if let profileName = target.game.active_profile {
            profile = try? await api.fetchProfile(
                game: target.game.title,
                name: profileName
            )
        }

        // 3. Run the TCP handshake using the token the server gave us
        let client = HandshakeClient(
            host: target.server.ip,
            port: streamResponse.handshakePort,
            token: streamResponse.token
        )
        handshakeClient = client
        client.onConnected = {
            connectParams = ConnectParams(
                ip: target.server.ip,
                port: target.server.videoPort,
                session: streamResponse.token,
                game: target.game.title
            )
        }
        client.onFailed = {
            errorMessage = "Handshake failed"
            failed = true
        }
        client.connect()
    }

    private func requestStream() async throws -> StreamResponse {
        let url = target.server.baseURL.appendingPathComponent("/stream")
        var request = URLRequest(url: url)
        request.httpMethod = "POST"
        request.setValue("application/json", forHTTPHeaderField: "Content-Type")
        request.httpBody = try JSONEncoder().encode(["game": target.game.title])
        let (data, response) = try await URLSession.shared.data(for: request)
        guard (response as? HTTPURLResponse)?.statusCode == 200 else {
            throw URLError(.badServerResponse)
        }
        return try JSONDecoder().decode(StreamResponse.self, from: data)
    }
}

private struct StreamResponse: Codable {
    let token: String
    let handshake_port: UInt16
    var handshakePort: UInt16 { handshake_port }
}
