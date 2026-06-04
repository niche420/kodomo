import Foundation
import Combine
import SwiftUI

struct PairedServer: Codable, Identifiable, Hashable {
    let id: UUID
    var name: String
    let ip: String
    let httpPort: UInt16
    let handshakePort: UInt16
    let videoPort: UInt16

    init(ip: String, httpPort: UInt16, handshakePort: UInt16, videoPort: UInt16) {
        self.id = UUID()
        self.name = ip
        self.ip = ip
        self.httpPort = httpPort
        self.handshakePort = handshakePort
        self.videoPort = videoPort
    }

    var baseURL: URL {
        URL(string: "http://\(ip):\(httpPort)")!
    }
}

class ServerStore: ObservableObject {
    @Published var servers: [PairedServer] = []
    private let key = "kd_paired_servers"

    init() { load() }

    func add(_ server: PairedServer) {
        if !servers.contains(where: { $0.ip == server.ip && $0.httpPort == server.httpPort }) {
            servers.append(server)
            save()
        }
    }

    func remove(at offsets: IndexSet) {
        servers.remove(atOffsets: offsets)
        save()
    }

    private func save() {
        if let data = try? JSONEncoder().encode(servers) {
            UserDefaults.standard.set(data, forKey: key)
        }
    }

    private func load() {
        guard let data = UserDefaults.standard.data(forKey: key),
              let decoded = try? JSONDecoder().decode([PairedServer].self, from: data)
        else { return }
        servers = decoded
    }
}

class ServerAPI {
    let server: PairedServer

    init(server: PairedServer) {
        self.server = server
    }

    struct GameEntry: Codable, Identifiable {
        var id: String { title }
        let title: String
        let active_profile: String?
    }

    func fetchGames() async throws -> [GameEntry] {
        let url = server.baseURL.appendingPathComponent("/games")
        let (data, _) = try await URLSession.shared.data(from: url)
        return try JSONDecoder().decode([GameEntry].self, from: data)
    }

    func fetchProfiles(for game: String) async throws -> [String] {
        let encoded = game.addingPercentEncoding(withAllowedCharacters: .urlPathAllowed) ?? game
        let url = server.baseURL.appendingPathComponent("/games/\(encoded)/profiles")
        let (data, _) = try await URLSession.shared.data(from: url)
        return try JSONDecoder().decode([String].self, from: data)
    }

    func fetchProfile(game: String, name: String) async throws -> GameProfile {
        let encodedGame = game.addingPercentEncoding(withAllowedCharacters: .urlPathAllowed) ?? game
        let encodedName = name.addingPercentEncoding(withAllowedCharacters: .urlPathAllowed) ?? name
        let url = server.baseURL.appendingPathComponent("/games/\(encodedGame)/profiles/\(encodedName)")
        let (data, _) = try await URLSession.shared.data(from: url)
        return try JSONDecoder().decode(GameProfile.self, from: data)
    }

    func saveProfile(game: String, name: String, profile: GameProfile) async throws {
        let encodedGame = game.addingPercentEncoding(withAllowedCharacters: .urlPathAllowed) ?? game
        let encodedName = name.addingPercentEncoding(withAllowedCharacters: .urlPathAllowed) ?? name
        let url = server.baseURL.appendingPathComponent("/games/\(encodedGame)/profiles/\(encodedName)")
        var request = URLRequest(url: url)
        request.httpMethod = "PUT"
        request.setValue("application/json", forHTTPHeaderField: "Content-Type")
        request.httpBody = try JSONEncoder().encode(profile)
        let (_, response) = try await URLSession.shared.data(for: request)
        guard (response as? HTTPURLResponse)?.statusCode == 200 else {
            throw URLError(.badServerResponse)
        }
    }

    func setActiveProfile(game: String, name: String) async throws {
        let encodedGame = game.addingPercentEncoding(withAllowedCharacters: .urlPathAllowed) ?? game
        let url = server.baseURL.appendingPathComponent("/games/\(encodedGame)/active")
        var request = URLRequest(url: url)
        request.httpMethod = "PUT"
        request.setValue("application/json", forHTTPHeaderField: "Content-Type")
        request.httpBody = try JSONEncoder().encode(["active": name])
        _ = try await URLSession.shared.data(for: request)
    }
}
