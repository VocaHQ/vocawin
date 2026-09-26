fn main() {
    // tauri-build embeds its Common Controls v6 manifest into the app binary
    // only, so `cargo test` binaries that link Tauri's dialog code fail to
    // start (comctl32 without v6 has no TaskDialogIndirect:
    // STATUS_ENTRYPOINT_NOT_FOUND). Embed the same manifest through the
    // linker for every MSVC binary instead, the app and tests alike.
    let target_env = std::env::var("CARGO_CFG_TARGET_ENV").unwrap_or_default();
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    if target_os == "windows" && target_env == "msvc" {
        let manifest = std::path::Path::new(&std::env::var("CARGO_MANIFEST_DIR").unwrap())
            .join("windows")
            .join("app.manifest");
        println!("cargo:rerun-if-changed={}", manifest.display());
        println!("cargo:rustc-link-arg=/MANIFEST:EMBED");
        println!("cargo:rustc-link-arg=/MANIFESTINPUT:{}", manifest.display());
        let attributes = tauri_build::Attributes::new()
            .windows_attributes(tauri_build::WindowsAttributes::new_without_app_manifest());
        tauri_build::try_build(attributes).expect("failed to run tauri-build");
    } else {
        tauri_build::build();
    }

    // Keep in sync with the transcribe-cpp `vulkan` feature under
    // `[target.'cfg(windows)'.dependencies]` in Cargo.toml. Catalog strings and
    // use_gpu gating read this cfg so they cannot claim Vulkan on CPU-only builds.
    println!("cargo:rustc-check-cfg=cfg(vocawin_whisper_vulkan)");
    if target_os == "windows" {
        println!("cargo:rustc-cfg=vocawin_whisper_vulkan");
    }
}
