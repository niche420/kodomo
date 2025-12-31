use std::env;
use std::path::PathBuf;

fn main() {
    let crate_dir = env::var("CARGO_MANIFEST_DIR").unwrap();

    // Output directory for C header
    let output_file = PathBuf::from(&crate_dir)
        .join("../../clients/desktop/include/kodomo.h");

    // Create parent directory if it doesn't exist
    if let Some(parent) = output_file.parent() {
        std::fs::create_dir_all(parent).ok();
    }

    cbindgen::Builder::new()
        .with_crate(crate_dir)
        .with_language(cbindgen::Language::C)
        .with_include_guard("KODOMO_H")
        .with_documentation(true)
        .with_style(cbindgen::Style::Both)
        .generate()
        .expect("Unable to generate bindings")
        .write_to_file(&output_file);

    println!("cargo:rerun-if-changed=src/lib.rs");
    println!("Generated C header: {}", output_file.display());
}