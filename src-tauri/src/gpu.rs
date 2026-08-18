//! Best-effort GPU readout for Settings. Prefers discrete adapters; skips WARP.

use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GpuStatus {
    pub available: bool,
    pub name: String,
    pub backend: String,
    pub detail: String,
    /// DXGI adapter index preferred for Whisper Vulkan (0 when none/CPU).
    pub device_index: i32,
    pub discrete: bool,
    pub vram_mb: u64,
}

pub fn detect_gpu() -> GpuStatus {
    #[cfg(windows)]
    {
        return detect_windows_gpu();
    }
    #[cfg(not(windows))]
    {
        GpuStatus {
            available: false,
            name: "Not available".into(),
            backend: "CPU".into(),
            detail: "GPU detection runs in Windows builds.".into(),
            device_index: -1,
            discrete: false,
            vram_mb: 0,
        }
    }
}

#[cfg(windows)]
fn detect_windows_gpu() -> GpuStatus {
    use windows::Win32::Graphics::Dxgi::{
        CreateDXGIFactory1, IDXGIAdapter1, IDXGIFactory1, DXGI_ADAPTER_FLAG,
        DXGI_ADAPTER_FLAG_SOFTWARE,
    };

    let factory: IDXGIFactory1 = match unsafe { CreateDXGIFactory1() } {
        Ok(factory) => factory,
        Err(error) => {
            return GpuStatus {
                available: false,
                name: "Unknown".into(),
                backend: "CPU".into(),
                detail: format!("DXGI unavailable ({error}); Whisper falls back to CPU."),
                device_index: -1,
                discrete: false,
                vram_mb: 0,
            };
        }
    };

    struct Candidate {
        index: u32,
        name: String,
        vram_mb: u64,
        discrete: bool,
        score: i64,
    }

    let mut index = 0u32;
    let mut best: Option<Candidate> = None;
    loop {
        let adapter: IDXGIAdapter1 = match unsafe { factory.EnumAdapters1(index) } {
            Ok(adapter) => adapter,
            Err(_) => break,
        };
        let desc = match unsafe { adapter.GetDesc1() } {
            Ok(desc) => desc,
            Err(_) => {
                index += 1;
                continue;
            }
        };
        let flags = DXGI_ADAPTER_FLAG(desc.Flags as i32);
        let name = wchar_to_string(&desc.Description);
        let name_l = name.to_ascii_lowercase();
        let software = flags.contains(DXGI_ADAPTER_FLAG_SOFTWARE)
            || name_l.contains("microsoft basic render")
            || name_l.contains("warp");
        if software {
            index += 1;
            continue;
        }
        let vram_mb = (desc.DedicatedVideoMemory as u64) / (1024 * 1024);
        let discrete = name_l.contains("nvidia")
            || name_l.contains("geforce")
            || name_l.contains("rtx")
            || name_l.contains("radeon")
            || name_l.contains("amd ")
            || name_l.contains("arc ")
            || (vram_mb >= 2048 && !name_l.contains("intel(r) uhd") && !name_l.contains("intel(r) hd"));
        // Prefer discrete, then higher VRAM, then earlier index.
        let score = (if discrete { 1_000_000 } else { 0 }) + vram_mb as i64;
        let candidate = Candidate {
            index,
            name,
            vram_mb,
            discrete,
            score,
        };
        if best.as_ref().map(|b| candidate.score > b.score).unwrap_or(true) {
            best = Some(candidate);
        }
        index += 1;
    }

    match best {
        Some(chosen) => {
            let kind = if chosen.discrete {
                "discrete"
            } else {
                "integrated"
            };
            GpuStatus {
                available: true,
                name: chosen.name.clone(),
                backend: if cfg!(vocawin_whisper_vulkan) {
                    "Vulkan (whisper.cpp) · DirectML (ONNX)".into()
                } else {
                    "CPU".into()
                },
                detail: format!(
                    "Using {kind} adapter “{}” (~{} MB VRAM){}. Software/WARP adapters are skipped.",
                    chosen.name,
                    chosen.vram_mb,
                    if cfg!(vocawin_whisper_vulkan) {
                        " for Whisper Vulkan"
                    } else {
                        ""
                    }
                ),
                device_index: chosen.index as i32,
                discrete: chosen.discrete,
                vram_mb: chosen.vram_mb,
            }
        }
        None => GpuStatus {
            available: false,
            name: "No discrete/integrated GPU found".into(),
            backend: "CPU".into(),
            detail: "Whisper and ONNX will use CPU. Software/WARP adapters were skipped.".into(),
            device_index: -1,
            discrete: false,
            vram_mb: 0,
        },
    }
}

#[cfg(windows)]
fn wchar_to_string(buf: &[u16]) -> String {
    let len = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
    String::from_utf16_lossy(&buf[..len])
}
