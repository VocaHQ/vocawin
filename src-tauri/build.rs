fn main() {
    tauri_build::build();

    // Keep in sync with the whisper-rs `vulkan` feature under
    // `[target.'cfg(windows)'.dependencies]` in Cargo.toml. Catalog strings and
    // use_gpu gating read this cfg so they cannot claim Vulkan on CPU-only builds.
    println!("cargo:rustc-check-cfg=cfg(vocawin_whisper_vulkan)");
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    if target_os == "windows" {
        println!("cargo:rustc-cfg=vocawin_whisper_vulkan");
    }
}
