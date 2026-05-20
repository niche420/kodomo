use std::{env, fs, path::PathBuf};

fn main() {
    #[cfg(target_os = "windows")]
    windows_ffmpeg();
}

#[cfg(target_os = "windows")]
fn windows_ffmpeg() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let vcpkg_root = env::var("VCPKG_ROOT").expect("VCPKG_ROOT not set — copy .cargo/config.toml.example to .cargo/config.toml and fill in your path");
    let ffmpeg_dir = if let Ok(dir) = env::var("FFMPEG_DIR") {
        PathBuf::from(dir)
    } else {
        let vcpkg_root = env::var("VCPKG_ROOT").expect("Either FFMPEG_DIR or VCPKG_ROOT must be set");
        PathBuf::from(vcpkg_root).join("installed\\x64-windows")
    };

    println!("cargo:warning=VCPKG_ROOT={}", vcpkg_root);
    println!("cargo:warning=FFMPEG_DIR={}", ffmpeg_dir.display());
    println!("cargo:rustc-env=FFMPEG_DIR={}", ffmpeg_dir.display());

    let profile = env::var("PROFILE").unwrap_or_else(|_| "debug".to_string());

    let root = env::var_os("CARGO_WORKSPACE_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            manifest_dir
                .parent()
                .expect("kd-server has no parent directory")
                .to_path_buf()
        });

    let target_dir = root.join("target").join(profile);
    let bin_dir = ffmpeg_dir.join("bin");

    let base_names = ["avcodec", "avutil", "avformat", "swscale", "swresample"];

    let entries = fs::read_dir(&bin_dir)
        .unwrap_or_else(|e| panic!("Could not read FFmpeg bin dir {}: {}", bin_dir.display(), e));

    for entry in entries.flatten() {
        let file_name = entry.file_name();
        let name = file_name.to_string_lossy();
        if name.ends_with(".dll") {
            let matches = base_names.iter().any(|base| name.starts_with(base));
            if matches {
                let src = entry.path();
                let dst = target_dir.join(&*name);
                println!("cargo:warning=Copying {} -> {}", src.display(), dst.display());
                if let Err(e) = fs::copy(&src, &dst) {
                    println!("cargo:warning=Failed to copy {}: {}", name, e);
                }
            }
        }
    }
}