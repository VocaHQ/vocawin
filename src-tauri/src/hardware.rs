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
/// largest model the GPU could theoretically hold. Uses the same preferred
/// discrete adapter as Settings (WARP/software already skipped).
pub fn recommend_starting_model() -> ModelRecommendation {
    let gpu = detect_gpu();
    let vram_mb = gpu.vram_mb;
    let discrete = gpu.discrete;

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
    } else if vram_mb >= 4096 || discrete {
        let detail = if vram_mb >= 8192 {
            format!(
                "{} with ~{vram_mb} MB VRAM can run larger models; Small is still the best starting size.",
                gpu.name
            )
        } else if vram_mb >= 4096 {
            format!("About {vram_mb} MB VRAM; Small balances speed and accuracy.")
        } else {
            format!(
                "{} looks discrete; Small is a good starting size.",
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recommendation_always_picks_whisper_family() {
        let rec = recommend_starting_model();
        assert!(rec.model_id.starts_with("whisper-"));
        assert!(!rec.reason.is_empty());
        assert_eq!(rec.vram_mb, rec.gpu.vram_mb);
    }
}
