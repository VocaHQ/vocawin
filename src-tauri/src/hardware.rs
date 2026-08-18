//! Starting-model recommendation from DXGI adapter memory and name.

use serde::Serialize;

use crate::gpu::{detect_gpu, GpuStatus};

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelRecommendation {
    pub model_id: String,
    pub model_name: String,
    pub reason: String,
    pub vram_mb: u64,
    pub gpu: GpuStatus,
}

/// Pick a sensible first Whisper size. Biased toward a starting size, not the
/// largest model the GPU could theoretically hold.
pub fn recommend_starting_model() -> ModelRecommendation {
    let gpu = detect_gpu();
    let vram_mb = detect_vram_mb();
    let name_l = gpu.name.to_ascii_lowercase();
    let discrete_hint = name_l.contains("rtx")
        || name_l.contains("radeon")
        || name_l.contains("geforce")
        || name_l.contains("arc ");

    let (model_id, model_name, reason) = if !gpu.available {
        (
            "whisper-tiny",
            "Whisper Tiny",
            "No GPU detected, so Tiny stays responsive on CPU.".into(),
        )
    } else if vram_mb > 0 && vram_mb < 2048 {
        (
            "whisper-tiny",
            "Whisper Tiny",
            format!("About {vram_mb} MB VRAM; Tiny is the safe starting size."),
        )
    } else if vram_mb > 0 && vram_mb < 4096 {
        (
            "whisper-base",
            "Whisper Base",
            format!("About {vram_mb} MB VRAM; Base is a solid starting size."),
        )
    } else if (vram_mb >= 4096) || discrete_hint {
        let detail = if vram_mb >= 8192 {
            format!(
                "{} with ~{vram_mb} MB VRAM can run larger models; Small is still the best starting size.",
                gpu.name
            )
        } else if vram_mb >= 4096 {
            format!("About {vram_mb} MB VRAM; Small balances speed and accuracy.")
        } else {
            format!(
                "{} looks discrete; Small is a good starting size (Vulkan).",
                gpu.name
            )
        };
        ("whisper-small", "Whisper Small", detail)
    } else {
        (
            "whisper-base",
            "Whisper Base",
            format!("{} detected; Base is a balanced starting size.", gpu.name),
        )
    };

    ModelRecommendation {
        model_id: model_id.into(),
        model_name: model_name.into(),
        reason,
        vram_mb,
        gpu,
    }
}

#[cfg(windows)]
fn detect_vram_mb() -> u64 {
    use windows::Win32::Graphics::Dxgi::{
        CreateDXGIFactory1, IDXGIAdapter1, IDXGIFactory1, DXGI_ADAPTER_FLAG,
        DXGI_ADAPTER_FLAG_SOFTWARE,
    };

    let factory: IDXGIFactory1 = match unsafe { CreateDXGIFactory1() } {
        Ok(factory) => factory,
        Err(_) => return 0,
    };
    let mut index = 0u32;
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
        let name = {
            let len = desc
                .Description
                .iter()
                .position(|&c| c == 0)
                .unwrap_or(desc.Description.len());
            String::from_utf16_lossy(&desc.Description[..len])
        };
        let software = flags.contains(DXGI_ADAPTER_FLAG_SOFTWARE)
            || name.to_ascii_lowercase().contains("microsoft basic render")
            || name.to_ascii_lowercase().contains("warp");
        if !software {
            return (desc.DedicatedVideoMemory as u64) / (1024 * 1024);
        }
        index += 1;
    }
    0
}

#[cfg(not(windows))]
fn detect_vram_mb() -> u64 {
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recommendation_always_picks_whisper_family() {
        let rec = recommend_starting_model();
        assert!(rec.model_id.starts_with("whisper-"));
        assert!(!rec.reason.is_empty());
    }
}
