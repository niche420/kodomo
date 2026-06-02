use std::path::PathBuf;
use kd_shared::profile::GameProfile;

fn profile_dir(game_title: &str) -> PathBuf {
    let safe: String = game_title.chars()
        .map(|c| if c.is_alphanumeric() || c == '-' { c } else { '_' })
        .collect();
    PathBuf::from("kd-server-data/profiles").join(safe)
}

fn profile_path(game_title: &str, profile_name: &str) -> PathBuf {
    let safe: String = profile_name.chars()
        .map(|c| if c.is_alphanumeric() || c == '-' { c } else { '_' })
        .collect();
    profile_dir(game_title).join(format!("{}.json", safe))
}

pub fn list_profiles(game_title: &str) -> Vec<String> {
    let dir = profile_dir(game_title);
    let Ok(entries) = std::fs::read_dir(&dir) else { return vec![] };
    entries
        .flatten()
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().into_owned();
            name.ends_with(".json").then(|| name.trim_end_matches(".json").to_string())
        })
        .collect()
}

pub fn load_profile(game_title: &str, profile_name: &str) -> Option<GameProfile> {
    let data = std::fs::read_to_string(profile_path(game_title, profile_name)).ok()?;
    serde_json::from_str(&data).ok()
}

pub fn save_profile(profile: &GameProfile) -> anyhow::Result<()> {
    // Infer profile name from game_title — used when saving from connect.rs
    save_profile_named(&profile.game_title, "Default", profile)
}

pub fn save_profile_named(game_title: &str, name: &str, profile: &GameProfile) -> anyhow::Result<()> {
    let path = profile_path(game_title, name);
    std::fs::create_dir_all(path.parent().unwrap())?;
    std::fs::write(path, serde_json::to_string_pretty(profile)?)?;
    Ok(())
}

pub fn delete_profile(game_title: &str, profile_name: &str) {
    let _ = std::fs::remove_file(profile_path(game_title, profile_name));
}