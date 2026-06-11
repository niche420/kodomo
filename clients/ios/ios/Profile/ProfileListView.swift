import SwiftUI

struct ProfileListView: View {
    let server: PairedServer
    let game: ServerAPI.GameEntry

    @State private var profiles: [String] = []
    @State private var activeProfile: String? = nil
    @State private var isLoading = false
    @State private var error: String? = nil

    // Create
    @State private var showingCreate = false
    @State private var newProfileName = ""

    // Rename
    @State private var renamingProfile: String? = nil
    @State private var renameText = ""

    // Editor navigation
    @State private var editingProfile: ProfileEditorTarget? = nil

    private var api: ServerAPI { ServerAPI(server: server) }

    var body: some View {
        Group {
            if isLoading {
                ProgressView("Loading profiles...")
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
            } else if profiles.isEmpty {
                emptyState
            } else {
                list
            }
        }
        .navigationTitle(game.title)
        .navigationBarTitleDisplayMode(.inline)
        .toolbar {
            ToolbarItem(placement: .primaryAction) {
                Button(action: { newProfileName = ""; showingCreate = true }) {
                    Label("New Profile", systemImage: "plus")
                }
            }
        }
        .task { await load() }
        // Create alert
        .alert("New Profile", isPresented: $showingCreate) {
            TextField("Profile name", text: $newProfileName)
            Button("Create") { Task { await createProfile(name: newProfileName) } }
                .disabled(newProfileName.trimmingCharacters(in: .whitespaces).isEmpty)
            Button("Cancel", role: .cancel) {}
        }
        // Rename alert
        .alert("Rename Profile", isPresented: Binding(
            get: { renamingProfile != nil },
            set: { if !$0 { renamingProfile = nil } }
        )) {
            TextField("New name", text: $renameText)
            Button("Rename") {
                guard let old = renamingProfile else { return }
                Task { await renameProfile(old: old, new: renameText) }
            }
            .disabled(renameText.trimmingCharacters(in: .whitespaces).isEmpty)
            Button("Cancel", role: .cancel) { renamingProfile = nil }
        }
        .navigationDestination(item: $editingProfile) { target in
            ProfileEditorView(server: server, game: game, profileName: target.name)
        }
    }

    // MARK: - Subviews

    private var emptyState: some View {
        VStack(spacing: 16) {
            Image(systemName: "gamecontroller")
                .font(.system(size: 50))
                .foregroundStyle(.secondary)
            Text("No profiles yet")
                .font(.title2)
            Text("Create a profile to map touch controls for \(game.title).")
                .multilineTextAlignment(.center)
                .foregroundStyle(.secondary)
                .padding(.horizontal, 40)
            Button("Create Profile") {
                newProfileName = ""
                showingCreate = true
            }
            .buttonStyle(.borderedProminent)
        }
    }

    private var list: some View {
        List {
            ForEach(profiles, id: \.self) { name in
                HStack {
                    VStack(alignment: .leading, spacing: 2) {
                        Text(name)
                            .font(.headline)
                        if name == activeProfile {
                            Text("Active")
                                .font(.caption)
                                .foregroundStyle(.secondary)
                        }
                    }
                    Spacer()
                    // Set active
                    if name != activeProfile {
                        Button("Set Active") {
                            Task { await setActive(name: name) }
                        }
                        .buttonStyle(.bordered)
                        .controlSize(.small)
                    }
                    // Edit
                    Button("Edit") {
                        editingProfile = ProfileEditorTarget(name: name)
                    }
                    .buttonStyle(.borderedProminent)
                    .controlSize(.small)
                }
                .swipeActions(edge: .trailing, allowsFullSwipe: false) {
                    Button("Delete", role: .destructive) {
                        Task { await deleteProfile(name: name) }
                    }
                    Button("Rename") {
                        renameText = name
                        renamingProfile = name
                    }
                    .tint(.orange)
                }
            }
        }
    }

    // MARK: - Actions

    private func load() async {
        isLoading = true
        error = nil
        do {
            async let p = api.fetchProfiles(for: game.title)
            async let a = api.fetchActiveProfile(for: game.title)
            (profiles, activeProfile) = try await (p, a)
        } catch {
            self.error = error.localizedDescription
        }
        isLoading = false
    }

    private func createProfile(name: String) async {
        let trimmed = name.trimmingCharacters(in: .whitespaces)
        guard !trimmed.isEmpty else { return }
        let blank = GameProfile(
            game_title: game.title,
            widgets: [],
            actions: [],
            bindings: []
        )
        do {
            try await api.saveProfile(game: game.title, name: trimmed, profile: blank)
            await load()
        } catch {
            self.error = error.localizedDescription
        }
    }

    private func deleteProfile(name: String) async {
        do {
            try await api.deleteProfile(game: game.title, name: name)
            await load()
        } catch {
            self.error = error.localizedDescription
        }
    }

    private func renameProfile(old: String, new: String) async {
        let trimmed = new.trimmingCharacters(in: .whitespaces)
        guard !trimmed.isEmpty else { return }
        do {
            // Fetch old, save under new name, delete old
            let profile = try await api.fetchProfile(game: game.title, name: old)
            try await api.saveProfile(game: game.title, name: trimmed, profile: profile)
            try await api.deleteProfile(game: game.title, name: old)
            if activeProfile == old {
                try await api.setActiveProfile(game: game.title, name: trimmed)
            }
            await load()
        } catch {
            self.error = error.localizedDescription
        }
    }

    private func setActive(name: String) async {
        do {
            try await api.setActiveProfile(game: game.title, name: name)
            activeProfile = name
        } catch {
            self.error = error.localizedDescription
        }
    }
}

struct ProfileEditorTarget: Identifiable, Hashable {
    let id = UUID()
    let name: String
}
