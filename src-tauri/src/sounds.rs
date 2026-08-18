//! Start/stop/error dictation cues via PlaySound.
//!
//! SND_MEMORY | SND_ASYNC must not point at a stack buffer. The WAV bytes live
//! in process-static storage for as long as async playback can run.

#[cfg(windows)]
fn tone_wav(frequency_hz: f32, duration_ms: u32, volume: f32) -> Vec<u8> {
    const SAMPLE_RATE: u32 = 16_000;
    let samples = (SAMPLE_RATE as u64 * duration_ms as u64 / 1000) as usize;
    let data_bytes = (samples * 2) as u32;
    let mut wav = Vec::with_capacity(44 + samples * 2);
    wav.extend_from_slice(b"RIFF");
    wav.extend_from_slice(&(36 + data_bytes).to_le_bytes());
    wav.extend_from_slice(b"WAVEfmt ");
    wav.extend_from_slice(&16u32.to_le_bytes());
    wav.extend_from_slice(&1u16.to_le_bytes()); // PCM
    wav.extend_from_slice(&1u16.to_le_bytes()); // mono
    wav.extend_from_slice(&SAMPLE_RATE.to_le_bytes());
    wav.extend_from_slice(&(SAMPLE_RATE * 2).to_le_bytes());
    wav.extend_from_slice(&2u16.to_le_bytes());
    wav.extend_from_slice(&16u16.to_le_bytes());
    wav.extend_from_slice(b"data");
    wav.extend_from_slice(&data_bytes.to_le_bytes());
    for index in 0..samples {
        let t = index as f32 / SAMPLE_RATE as f32;
        let fade = if index < 48 {
            index as f32 / 48.0
        } else if index + 48 >= samples {
            (samples - index) as f32 / 48.0
        } else {
            1.0
        };
        let sample =
            (t * frequency_hz * std::f32::consts::TAU).sin() * volume * fade * i16::MAX as f32;
        wav.extend_from_slice(&(sample as i16).to_le_bytes());
    }
    wav
}

#[cfg(windows)]
fn play_static_wav(wav: &'static [u8]) {
    use windows::core::PCSTR;
    use windows::Win32::Media::Audio::{PlaySoundA, SND_ASYNC, SND_MEMORY, SND_NODEFAULT};
    unsafe {
        let _ = PlaySoundA(
            PCSTR(wav.as_ptr()),
            None,
            SND_MEMORY | SND_ASYNC | SND_NODEFAULT,
        );
    }
}

#[cfg(windows)]
fn start_wav() -> &'static [u8] {
    use std::sync::OnceLock;
    static WAV: OnceLock<Vec<u8>> = OnceLock::new();
    WAV.get_or_init(|| tone_wav(880.0, 70, 0.22))
}

#[cfg(windows)]
fn stop_wav() -> &'static [u8] {
    use std::sync::OnceLock;
    static WAV: OnceLock<Vec<u8>> = OnceLock::new();
    WAV.get_or_init(|| tone_wav(520.0, 90, 0.2))
}

#[cfg(windows)]
fn error_wav() -> &'static [u8] {
    use std::sync::OnceLock;
    static WAV: OnceLock<Vec<u8>> = OnceLock::new();
    WAV.get_or_init(|| tone_wav(220.0, 140, 0.25))
}

#[cfg(windows)]
pub fn play_start() {
    play_static_wav(start_wav());
}

#[cfg(windows)]
pub fn play_stop() {
    play_static_wav(stop_wav());
}

#[cfg(windows)]
pub fn play_error() {
    play_static_wav(error_wav());
}

#[cfg(not(windows))]
pub fn play_start() {}

#[cfg(not(windows))]
pub fn play_stop() {}

#[cfg(not(windows))]
pub fn play_error() {}

pub fn play_if_enabled(enabled: bool, start: bool) {
    if !enabled {
        return;
    }
    if start {
        play_start();
    } else {
        play_stop();
    }
}

pub fn play_error_if_enabled(enabled: bool) {
    if enabled {
        play_error();
    }
}

#[cfg(test)]
mod tests {
    #[cfg(windows)]
    #[test]
    fn tone_wav_is_riff_pcm() {
        let wav = super::tone_wav(440.0, 50, 0.2);
        assert!(wav.starts_with(b"RIFF"));
        assert!(wav.len() > 44);
    }
}
