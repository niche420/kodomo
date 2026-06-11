import Foundation

extension ServerAPI {
    func fetchActiveProfile(for game: String) async throws -> String? {
        let encoded = game.addingPercentEncoding(withAllowedCharacters: .urlPathAllowed) ?? game
        let url = server.baseURL.appendingPathComponent("/games/\(encoded)/active")
        let (data, _) = try await URLSession.shared.data(from: url)
        let body = try JSONDecoder().decode([String: String?].self, from: data)
        return body["active"] ?? nil
    }
 
    func deleteProfile(game: String, name: String) async throws {
        let encodedGame = game.addingPercentEncoding(withAllowedCharacters: .urlPathAllowed) ?? game
        let encodedName = name.addingPercentEncoding(withAllowedCharacters: .urlPathAllowed) ?? name
        let url = server.baseURL.appendingPathComponent("/games/\(encodedGame)/profiles/\(encodedName)")
        var request = URLRequest(url: url)
        request.httpMethod = "DELETE"
        let (_, response) = try await URLSession.shared.data(for: request)
        guard (response as? HTTPURLResponse)?.statusCode == 200 else {
            throw URLError(.badServerResponse)
        }
    }
}
