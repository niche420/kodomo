use std::{env, fs, path::PathBuf};

fn main() {
    // Path to the vcpkg FFmpeg install (relative to repo root)
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let ffmpeg_dir = manifest_dir
        .join("..\\..\\..\\vcpkg\\installed\\x64-windows"); // Adjust relative path from kd-encoder

    // Export FFMPEG_DIR for ffmpeg-next
    println!("cargo:rustc-env=FFMPEG_DIR={}", ffmpeg_dir.display());

    // Copy DLLs to target directory for runtime
    let profile = env::var("PROFILE").expect("PROFILE env var is always set, darling—what's the drama?");

    // Hunt for workspace root: env var if set, else hop up from manifest_dir
    let root = env::var_os("CARGO_WORKSPACE_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            let mut root = manifest_dir.clone();
            // Two hops: kd-encoder -> crates -> server
            for _ in 0..2 {
                if let Some(parent) = root.parent() {
                    root = parent.to_path_buf();
                } else {
                    panic!("Couldn't climb to workspace root, honey—check your dir structure!");
                }
            }
            root
        });

    let target_dir = root.join("target").join(profile);

    // List of common FFmpeg DLLs used by ffmpeg-next
    let dlls = ["avcodec-62.dll", "avutil-60.dll", "avformat-62.dll", "swscale-9.dll", "swresample-6.dll"];

    for dll in dlls.iter() {
        let src = ffmpeg_dir.join("bin").join(dll);
        println!("cargo:warning=Looking in {}", src.display());
        let dst = target_dir.join(dll);
        println!("cargo:warning=Putting in {}", dst.display());
        if let Err(e) = fs::copy(&src, &dst) {
            println!("cargo:warning=Failed to copy {}: {}", dll, e);
        }
    }
}