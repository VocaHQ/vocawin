//! Start/stop/error dictation cues via PlaySound.
//!
//! SND_MEMORY | SND_ASYNC must not point at a stack buffer. Theme WAVs are
//! compiled in as static bytes so async playback can outlive the call.

const PLAYABLE: &[&str] = &[
    "lift", "flick", "ember", "step", "voca", "soft", "chirp", "scale", "drop", "glass",
];

/// Map a saved or typed theme to a catalog id. Empty follows `sound_effects`.
/// Unknown ids and the old `fifth` name become `voca`.
pub fn parse_theme(raw: &str, sound_effects: bool) -> String {
    let key = raw.trim().to_ascii_lowercase();
    if key.is_empty() {
        return if sound_effects { "voca" } else { "off" }.into();
    }
    if key == "fifth" {
        return "voca".into();
    }
    if key == "off" || PLAYABLE.contains(&key.as_str()) {
        return key;
    }
    "voca".into()
}

/// Keep `sound_theme` and `sound_effects` in sync. Returns true if either changed.
pub fn apply_theme(sound_theme: &mut String, sound_effects: &mut bool) -> bool {
    let next = parse_theme(sound_theme, *sound_effects);
    let enabled = next != "off";
    let changed = *sound_theme != next || *sound_effects != enabled;
    *sound_theme = next;
    *sound_effects = enabled;
    changed
}

pub fn sounds_on(theme: &str) -> bool {
    theme != "off"
}

fn is_playable(id: &str) -> bool {
    PLAYABLE.contains(&id)
}

fn cue_bytes(theme: &str, start: bool) -> Option<&'static [u8]> {
    if theme == "off" {
        return None;
    }
    Some(match (theme, start) {
        ("lift", true) => include_bytes!("../sounds/lift/start.wav"),
        ("lift", false) => include_bytes!("../sounds/lift/stop.wav"),
        ("flick", true) => include_bytes!("../sounds/flick/start.wav"),
        ("flick", false) => include_bytes!("../sounds/flick/stop.wav"),
        ("ember", true) => include_bytes!("../sounds/ember/start.wav"),
        ("ember", false) => include_bytes!("../sounds/ember/stop.wav"),
        ("step", true) => include_bytes!("../sounds/step/start.wav"),
        ("step", false) => include_bytes!("../sounds/step/stop.wav"),
        ("voca", true) => include_bytes!("../sounds/voca/start.wav"),
        ("voca", false) => include_bytes!("../sounds/voca/stop.wav"),
        ("soft", true) => include_bytes!("../sounds/soft/start.wav"),
        ("soft", false) => include_bytes!("../sounds/soft/stop.wav"),
        ("chirp", true) => include_bytes!("../sounds/chirp/start.wav"),
        ("chirp", false) => include_bytes!("../sounds/chirp/stop.wav"),
        ("scale", true) => include_bytes!("../sounds/scale/start.wav"),
        ("scale", false) => include_bytes!("../sounds/scale/stop.wav"),
        ("drop", true) => include_bytes!("../sounds/drop/start.wav"),
        ("drop", false) => include_bytes!("../sounds/drop/stop.wav"),
        ("glass", true) => include_bytes!("../sounds/glass/start.wav"),
        ("glass", false) => include_bytes!("../sounds/glass/stop.wav"),
        (_, true) => include_bytes!("../sounds/voca/start.wav"),
        (_, false) => include_bytes!("../sounds/voca/stop.wav"),
    })
}

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
fn error_wav() -> &'static [u8] {
    use std::sync::OnceLock;
    static WAV: OnceLock<Vec<u8>> = OnceLock::new();
    WAV.get_or_init(|| tone_wav(220.0, 140, 0.25))
}

fn play_cue(theme: &str, start: bool) {
    let id = parse_theme(theme, true);
    let Some(wav) = cue_bytes(&id, start) else {
        return;
    };
    #[cfg(windows)]
    play_static_wav(wav);
    let _ = wav;
}

pub fn play_if_enabled(theme: &str, start: bool) {
    if !sounds_on(theme) {
        return;
    }
    play_cue(theme, start);
}

pub fn play_error_if_enabled(theme: &str) {
    if !sounds_on(theme) {
        return;
    }
    #[cfg(windows)]
    play_static_wav(error_wav());
}

/// Play one half of a pair without starting dictation. Rejects unknown ids.
pub fn preview_theme(theme: &str, start: bool) -> Result<(), String> {
    let key = theme.trim().to_ascii_lowercase();
    if key == "off" {
        return Ok(());
    }
    if !is_playable(&key) {
        return Err(format!("Unknown sound: {theme}"));
    }
    play_cue(&key, start);
    Ok(())
}

#[cfg(test)]
mod tests {
    fn wav_is_pcm_mono(bytes: &[u8]) {
        assert!(bytes.starts_with(b"RIFF"), "missing RIFF");
        assert_eq!(&bytes[8..12], b"WAVE");
        assert!(bytes.len() > 44);
        let channels = u16::from_le_bytes([bytes[22], bytes[23]]);
        let rate = u32::from_le_bytes([bytes[24], bytes[25], bytes[26], bytes[27]]);
        let bits = u16::from_le_bytes([bytes[34], bytes[35]]);
        assert_eq!(channels, 1);
        assert_eq!(bits, 16);
        assert!(
            rate == 16_000 || rate == 22_050 || rate == 44_100 || rate == 48_000,
            "rate {rate}"
        );
    }

    #[test]
    fn theme_wavs_have_sane_headers() {
        for theme in super::PLAYABLE {
            wav_is_pcm_mono(super::cue_bytes(theme, true).expect("start"));
            wav_is_pcm_mono(super::cue_bytes(theme, false).expect("stop"));
        }
    }

    #[test]
    fn parse_theme_defaults_unknown_to_voca() {
        assert_eq!(super::parse_theme("", true), "voca");
        assert_eq!(super::parse_theme("", false), "off");
        assert_eq!(super::parse_theme("fifth", true), "voca");
        assert_eq!(super::parse_theme("nope", true), "voca");
        assert_eq!(super::parse_theme("Lift", true), "lift");
        assert_eq!(super::parse_theme("off", true), "off");
        assert_eq!(super::parse_theme("voca", false), "voca");
    }

    #[test]
    fn apply_theme_derives_sound_effects() {
        let mut theme = String::new();
        let mut enabled = false;
        assert!(super::apply_theme(&mut theme, &mut enabled));
        assert_eq!(theme, "off");
        assert!(!enabled);

        theme = "fifth".into();
        enabled = false;
        assert!(super::apply_theme(&mut theme, &mut enabled));
        assert_eq!(theme, "voca");
        assert!(enabled);

        theme = "ember".into();
        enabled = true;
        assert!(!super::apply_theme(&mut theme, &mut enabled));
        assert!(enabled);
    }

    #[test]
    fn off_plays_nothing() {
        assert!(super::cue_bytes("off", true).is_none());
        assert!(super::cue_bytes("off", false).is_none());
        assert!(!super::sounds_on("off"));
        assert!(super::preview_theme("off", true).is_ok());
    }

    #[test]
    fn preview_rejects_unknown_ids() {
        assert!(super::preview_theme("not-a-tone", true).is_err());
        assert!(super::preview_theme("fifth", false).is_err());
        assert!(super::preview_theme("voca", true).is_ok());
        assert!(super::preview_theme("lift", false).is_ok());
    }

    #[cfg(windows)]
    #[test]
    fn error_tone_wav_is_riff_pcm() {
        wav_is_pcm_mono(&super::tone_wav(440.0, 50, 0.2));
    }
}
