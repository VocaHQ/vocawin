//! VocaWin's platform shell. Recognition engines are deliberately behind a small
//! catalog/adapter boundary so model downloads never require a cloud account.

mod autopause;
mod autostart;
mod chunking;
mod cleanup;
mod devices;
mod dictionary;
mod ducking;
mod gpu;
mod hardware;
mod hinglish;
mod history;
mod hook;
mod hotkey;
mod lang_id;
mod live_preview;
mod logbuf;
mod machine;
mod model_slot;
mod output;
mod overlay;
mod pipeline;
mod power;
mod silence;
mod sounds;
mod spoken_emoji;
mod spoken_numbers;
mod stats;
mod vad;
mod vocabulary;
mod whisper_cache;

#[cfg(windows)]
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use serde::{Deserialize, Serialize};
#[cfg(windows)]
use std::sync::mpsc;
#[cfg(windows)]
use std::sync::Arc;
use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, AtomicU64, Ordering},
    sync::Mutex,
};
use tauri::{AppHandle, Emitter, Manager, State};
use transcribe_rs::SpeechModel;

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct Model {
    id: &'static str,
    name: &'static str,
    engine: &'static str,
    size: &'static str,
    languages: &'static str,
    acceleration: &'static str,
    description: &'static str,
}

/// Compile-time honesty for Whisper catalog labels. Set by `build.rs` only when
/// the Windows target enables `transcribe-cpp/vulkan`.
fn whisper_acceleration() -> &'static str {
    if cfg!(vocawin_whisper_vulkan) {
        "CPU · Vulkan"
    } else {
        "CPU"
    }
}

fn onnx_acceleration(model_id: &str) -> &'static str {
    if onnx_uses_directml(model_id) && cfg!(windows) {
        "CPU · DirectML"
    } else {
        "CPU"
    }
}

fn gpu_backends_summary() -> Vec<&'static str> {
    let mut backends = Vec::new();
    if cfg!(vocawin_whisper_vulkan) {
        backends.push("Vulkan (transcribe.cpp)");
    }
    if cfg!(windows) {
        backends.push("DirectML (ONNX Runtime)");
    }
    backends.push("CPU fallback");
    backends
}

/// The catalog is intentionally engine-neutral. The transcription adapter can
/// select whisper.cpp (GGUF), ONNX Runtime, or Vosk without changing the UI.
fn model_catalog() -> Vec<Model> {
    let whisper_accel = whisper_acceleration();
    vec![
        Model {
            id: "whisper-tiny",
            name: "Whisper Tiny",
            engine: "whisper.cpp",
            size: "75 MB",
            languages: "99 languages",
            acceleration: whisper_accel,
            description: "Fastest Whisper option; included as the first-run recommendation.",
        },
        Model {
            id: "whisper-base",
            name: "Whisper Base",
            engine: "whisper.cpp",
            size: "142 MB",
            languages: "99 languages",
            acceleration: whisper_accel,
            description: "Balanced accuracy for everyday dictation.",
        },
        Model {
            id: "whisper-small",
            name: "Whisper Small",
            engine: "whisper.cpp",
            size: "466 MB",
            languages: "99 languages",
            acceleration: whisper_accel,
            description: "Higher accuracy on modern PCs.",
        },
        Model {
            id: "whisper-medium",
            name: "Whisper Medium",
            engine: "whisper.cpp",
            size: "1.5 GB",
            languages: "99 languages",
            acceleration: whisper_accel,
            description: "Excellent multilingual recognition.",
        },
        Model {
            id: "whisper-large-v3",
            name: "Whisper Large v3",
            engine: "whisper.cpp",
            size: "3.1 GB",
            languages: "99 languages",
            acceleration: whisper_accel,
            description: "Maximum Whisper accuracy.",
        },
        Model {
            id: "whisper-large-v3-turbo",
            name: "Whisper Large v3 Turbo",
            engine: "whisper.cpp",
            size: "1.6 GB",
            languages: "99 languages",
            acceleration: whisper_accel,
            description: "Large-v3 quality tuned for lower latency.",
        },
        Model {
            id: "distil-whisper-large-v3",
            name: "Distil-Whisper Large v3",
            engine: "whisper.cpp",
            size: "1.5 GB",
            languages: "English",
            acceleration: whisper_accel,
            description: "Fast English-only Whisper derivative.",
        },
        Model {
            id: VOCA_HINGLISH,
            name: "Voca Hinglish",
            engine: "whisper.cpp",
            size: "834 MB",
            languages: "Hindi, English",
            acceleration: whisper_accel,
            description: "Writes Hindi speech in Roman script (Hinglish). Always decodes as English, so it ignores the language setting.",
        },
        Model {
            id: "parakeet-tdt-0.6b-v3",
            name: "Parakeet TDT 0.6B v3",
            engine: "ONNX Runtime",
            size: "478 MB",
            languages: "25 languages",
            acceleration: onnx_acceleration("parakeet-tdt-0.6b-v3"),
            description: "High-speed multilingual dictation.",
        },
        Model {
            id: "moonshine-tiny",
            name: "Moonshine Tiny",
            engine: "ONNX Runtime",
            size: "145 MB",
            languages: "English",
            acceleration: onnx_acceleration("moonshine-tiny"),
            description: "Low-memory, quick English notes.",
        },
        Model {
            id: "moonshine-base",
            name: "Moonshine Base",
            engine: "ONNX Runtime",
            size: "190 MB",
            languages: "English",
            acceleration: onnx_acceleration("moonshine-base"),
            description: "Compact English model.",
        },
        Model {
            id: "sensevoice-small",
            name: "SenseVoice Small",
            engine: "ONNX Runtime",
            size: "240 MB",
            languages: "Chinese · Japanese · Korean · Cantonese · English",
            acceleration: onnx_acceleration("sensevoice-small"),
            description: "East Asian language specialist.",
        },
        Model {
            id: "gigaam-v3",
            name: "GigaAM v3",
            engine: "ONNX Runtime",
            size: "225 MB",
            languages: "Russian",
            acceleration: onnx_acceleration("gigaam-v3"),
            description: "Russian recognition with punctuation.",
        },
        Model {
            id: "canary-180m",
            name: "Canary 180M Flash",
            engine: "ONNX Runtime",
            size: "150 MB",
            languages: "English · Spanish · German · French",
            acceleration: onnx_acceleration("canary-180m"),
            description: "Fast four-language transcription.",
        },
        Model {
            id: "canary-1b-v2",
            name: "Canary 1B v2",
            engine: "ONNX Runtime",
            size: "982 MB",
            languages: "25 European languages",
            acceleration: onnx_acceleration("canary-1b-v2"),
            description: "Larger Canary for 25 European languages. Slower than 180M Flash.",
        },
        Model {
            id: "cohere-transcribe",
            name: "Cohere Transcribe",
            engine: "ONNX Runtime",
            size: "1.9 GB",
            languages: "14 languages",
            acceleration: onnx_acceleration("cohere-transcribe"),
            description: "Cohere's 2B model (int4) for 14 languages, including Arabic, Chinese, Japanese and Korean.",
        },
    ]
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct Settings {
    hotkey: String,
    activation_mode: String,
    language: String,
    silence_seconds: f32,
    #[serde(default = "default_max_recording_seconds")]
    max_recording_seconds: f32,
    launch_at_login: bool,
    #[serde(default = "default_true")]
    sound_effects: bool,
    /// Catalog id. Empty on old files; `load_settings` fills it from `sound_effects`.
    #[serde(default)]
    sound_theme: String,
    #[serde(default = "default_true")]
    append_trailing_space: bool,
    #[serde(default = "default_true")]
    auto_capitalize: bool,
    selected_model: String,
    /// Empty string means the WASAPI default capture device.
    #[serde(default)]
    input_device: String,
    #[serde(default)]
    auto_pause_enabled: bool,
    /// Newline / comma separated process names (e.g. `obs64.exe`).
    #[serde(default)]
    auto_pause_apps: String,
    #[serde(default)]
    idle_unload_enabled: bool,
    #[serde(default = "default_idle_unload_seconds")]
    idle_unload_seconds: u32,
    /// First-run welcome was dismissed.
    #[serde(default)]
    welcome_dismissed: bool,
    #[serde(default = "default_true")]
    history_enabled: bool,
    /// When false, Debug shows error and warning only.
    #[serde(default)]
    debug_logging: bool,
    /// Raw Custom Vocabulary list (Mac/Phone UX). Parsed like VocaPhone
    /// `CustomVocabulary`, then sent to whisper.cpp as `initial_prompt`.
    #[serde(default)]
    custom_vocabulary: String,
    /// VocaLinux `copy_to_clipboard`: leave the transcript on the clipboard.
    /// Off by default so insertion does not take over whatever was copied.
    #[serde(default)]
    copy_to_clipboard: bool,
    /// VocaMac cleanup level without a model: `medium` removes "um"/"uh",
    /// single-letter stutters, and spoken corrections; `none` keeps every word.
    #[serde(default = "default_cleanup_level")]
    cleanup_level: String,
    /// "twenty three" → 23. Off by default, like VocaMac.
    #[serde(default)]
    numbers_as_digits: bool,
    /// With digits: "50%", "$5.50", "21st", "June 22".
    #[serde(default)]
    number_symbols: bool,
    /// "party emoji" → 🎉.
    #[serde(default)]
    spoken_emoji: bool,
    /// Personal dictionary replacements, for every engine.
    #[serde(default)]
    replacements: Vec<dictionary::Replacement>,
    #[serde(default)]
    snippets: Vec<dictionary::Snippet>,
    /// Escape throws away a dictation while it records or transcribes.
    #[serde(default = "default_true")]
    escape_cancels: bool,
    /// Separate start/stop shortcut. Empty means none.
    #[serde(default)]
    hands_free_hotkey: String,
    /// Types the last dictation again. Empty means none.
    #[serde(default)]
    paste_last_hotkey: String,
    /// `middle`, `x1`, `x2`, or empty for no mouse button.
    #[serde(default)]
    mouse_button: String,
    /// On-screen pill while dictating: `minimal` or `off`.
    #[serde(default = "default_overlay_style")]
    overlay_style: String,
    /// `bottom` or `top` of the screen.
    #[serde(default = "default_overlay_position")]
    overlay_position: String,
    /// "VocaWin is ready" pill for a few seconds after launch.
    #[serde(default = "default_true")]
    ready_pill: bool,
    /// Cut silence before the model hears the recording.
    #[serde(default = "default_true")]
    skip_silence: bool,
    /// Show text in the pill while speaking (costs CPU; off by default).
    #[serde(default)]
    live_preview: bool,
    /// Mute other apps' sound while recording.
    #[serde(default)]
    mute_other_audio: bool,
    /// Days to keep history; 0 keeps it forever.
    #[serde(default = "default_history_retention_days")]
    history_retention_days: u32,
    /// Keep recent takes' audio for replay and retry.
    #[serde(default = "default_true")]
    history_keep_audio: bool,
    /// `type` (SendInput, clipboard untouched) or `paste` (clipboard paste,
    /// then the previous clipboard is restored).
    #[serde(default = "default_insertion_mode")]
    insertion_mode: String,
    /// Newline/comma separated process names that always get a paste: apps
    /// that drop typed characters.
    #[serde(default)]
    paste_apps: String,
}

fn default_true() -> bool {
    true
}

fn default_cleanup_level() -> String {
    "medium".into()
}

fn default_overlay_style() -> String {
    "minimal".into()
}

fn default_overlay_position() -> String {
    "bottom".into()
}

fn default_history_retention_days() -> u32 {
    30
}

fn default_insertion_mode() -> String {
    "type".into()
}

fn default_max_recording_seconds() -> f32 {
    60.0
}

fn default_idle_unload_seconds() -> u32 {
    300
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            hotkey: "AltRight".into(),
            activation_mode: "pushToTalk".into(),
            language: "Auto-detect".into(),
            silence_seconds: 1.5,
            max_recording_seconds: 60.0,
            launch_at_login: false,
            sound_effects: true,
            sound_theme: "voca".into(),
            append_trailing_space: true,
            auto_capitalize: true,
            selected_model: "whisper-tiny".into(),
            input_device: String::new(),
            auto_pause_enabled: false,
            auto_pause_apps: String::new(),
            idle_unload_enabled: false,
            idle_unload_seconds: 300,
            welcome_dismissed: false,
            history_enabled: true,
            debug_logging: false,
            custom_vocabulary: String::new(),
            copy_to_clipboard: false,
            cleanup_level: default_cleanup_level(),
            numbers_as_digits: false,
            number_symbols: false,
            spoken_emoji: false,
            replacements: Vec::new(),
            snippets: Vec::new(),
            escape_cancels: true,
            hands_free_hotkey: String::new(),
            paste_last_hotkey: String::new(),
            mouse_button: String::new(),
            overlay_style: default_overlay_style(),
            overlay_position: default_overlay_position(),
            ready_pill: true,
            skip_silence: true,
            live_preview: false,
            mute_other_audio: false,
            history_retention_days: default_history_retention_days(),
            history_keep_audio: true,
            insertion_mode: default_insertion_mode(),
            paste_apps: String::new(),
        }
    }
}

/// How the running session was started, so only the matching release or
/// shortcut ends it (a hands-free take is not stopped by a stray hotkey tap).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Trigger {
    /// Push-to-talk hold of the main hotkey.
    #[default]
    Hold,
    /// Main hotkey in toggle mode.
    Toggle,
    HandsFree,
    /// Mouse button; held or toggled like the hotkey.
    Mouse,
    /// Tray menu or the VocaWin window.
    Manual,
}

/// WASAPI `cpal::Stream` is intentionally `!Send`/`!Sync` across platforms.
/// Keep the live stream on one dedicated thread and expose only channel handles
/// to Tauri `State`, which requires `Send + Sync`.
#[cfg(windows)]
enum AudioCommand {
    Start {
        silence_seconds: f32,
        max_seconds: f32,
        device_name: String,
        /// Toggle/double-tap only. Push-to-talk ignores silence and stops on key-up.
        silence_auto_stop: bool,
        /// Level meter only: no silence auto-stop and no transcription handoff.
        meter_only: bool,
        /// The take's id, handed back with its audio when it auto-stops.
        session: u64,
        reply: mpsc::Sender<Result<(), String>>,
    },
    Stop {
        reply: mpsc::Sender<Result<(Vec<f32>, u32), String>>,
    },
    Level {
        reply: mpsc::Sender<f32>,
    },
    /// True while a dictation capture stream is open (not the mic-test meter).
    IsLive {
        reply: mpsc::Sender<bool>,
    },
    /// Close a microphone kept open after a take (auto-pause).
    ReleaseMicrophone,
    /// The last `seconds` of the take in progress, for the live preview.
    Snapshot {
        seconds: f32,
        reply: mpsc::Sender<Option<(Vec<f32>, u32)>>,
    },
}

/// Where the last `seconds` of `len` samples at `rate` begin.
fn tail_start(len: usize, rate: u32, seconds: f32) -> usize {
    len.saturating_sub((rate as f32 * seconds) as usize)
}

const AUDIO_REPLY_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(4);

fn recv_audio_reply<T>(
    response: std::sync::mpsc::Receiver<T>,
    timeout: std::time::Duration,
    what: &str,
) -> Result<T, String> {
    response.recv_timeout(timeout).map_err(|err| match err {
        std::sync::mpsc::RecvTimeoutError::Timeout => {
            format!("Microphone {what} timed out. The audio thread did not reply in time.")
        }
        std::sync::mpsc::RecvTimeoutError::Disconnected => "Audio thread did not respond".into(),
    })
}

#[cfg(windows)]
struct AudioRecorder {
    commands: mpsc::Sender<AudioCommand>,
}

#[cfg(windows)]
fn note_audio_sample(
    mono: f32,
    last_voice_ms: &Arc<Mutex<u128>>,
    heard_speech: &Arc<Mutex<bool>>,
    peak_level: &Arc<Mutex<f32>>,
) {
    const VOICE_THRESHOLD: f32 = 0.015;
    let level = mono.abs();
    if let Ok(mut peak) = peak_level.lock() {
        *peak = (*peak * 0.92).max(level);
    }
    if level >= VOICE_THRESHOLD {
        if let Ok(mut heard) = heard_speech.lock() {
            *heard = true;
        }
        if let Ok(mut last) = last_voice_ms.lock() {
            *last = now_ms();
        }
    }
}

/// Appends a callback's samples to the take. The store flag is read under
/// the buffer lock, and `take_recording` clears it under the same lock, so
/// no frame lands in a buffer that was already handed off.
#[cfg(windows)]
fn store_frames(samples: &Arc<Mutex<Vec<f32>>>, store: &AtomicBool, frames: &[f32]) {
    if let Ok(mut buffer) = samples.lock() {
        if store.load(Ordering::Relaxed) {
            buffer.extend_from_slice(frames);
        }
    }
}

#[cfg(windows)]
fn open_input_stream(
    samples: Arc<Mutex<Vec<f32>>>,
    last_voice_ms: Arc<Mutex<u128>>,
    heard_speech: Arc<Mutex<bool>>,
    peak_level: Arc<Mutex<f32>>,
    store: Arc<AtomicBool>,
    last_audio_ms: Arc<std::sync::atomic::AtomicU64>,
    device_name: &str,
) -> Result<(cpal::Stream, u32, String), String> {
    let host = cpal::default_host();
    let device = if device_name.trim().is_empty() {
        host.default_input_device()
            .ok_or("No microphone was found. Connect or enable an input device and try again.")?
    } else {
        host.input_devices()
            .map_err(|error| format!("Could not enumerate microphones: {error}"))?
            .find(|candidate| candidate.name().ok().as_deref() == Some(device_name))
            .ok_or_else(|| {
                format!(
                    "Microphone `{device_name}` was not found. Pick another device in Settings."
                )
            })?
    };
    let supported = device
        .default_input_config()
        .map_err(|error| format!("Could not read microphone format: {error}"))?;
    let sample_rate = supported.sample_rate().0;
    let channels = supported.channels() as usize;
    samples
        .lock()
        .map_err(|_| "Audio lock was poisoned")?
        .clear();
    if let Ok(mut peak) = peak_level.lock() {
        *peak = 0.0;
    }
    let error_callback = |error| eprintln!("VocaWin audio input error: {error}");
    let config: cpal::StreamConfig = supported.clone().into();
    let stream = match supported.sample_format() {
        cpal::SampleFormat::F32 => {
            let samples = Arc::clone(&samples);
            let last_voice_ms = Arc::clone(&last_voice_ms);
            let heard_speech = Arc::clone(&heard_speech);
            let peak_level = Arc::clone(&peak_level);
            let store = Arc::clone(&store);
            let last_audio_ms = Arc::clone(&last_audio_ms);
            device.build_input_stream(
                &config,
                move |data: &[f32], _| {
                    last_audio_ms.store(now_ms() as u64, Ordering::Relaxed);
                    let mut frames = Vec::with_capacity(data.len() / channels.max(1));
                    for frame in data.chunks(channels) {
                        let mono = frame.iter().sum::<f32>() / frame.len() as f32;
                        note_audio_sample(mono, &last_voice_ms, &heard_speech, &peak_level);
                        frames.push(mono);
                    }
                    store_frames(&samples, &store, &frames);
                },
                error_callback,
                None,
            )
        }
        cpal::SampleFormat::I16 => {
            let samples = Arc::clone(&samples);
            let last_voice_ms = Arc::clone(&last_voice_ms);
            let heard_speech = Arc::clone(&heard_speech);
            let peak_level = Arc::clone(&peak_level);
            let store = Arc::clone(&store);
            let last_audio_ms = Arc::clone(&last_audio_ms);
            device.build_input_stream(
                &config,
                move |data: &[i16], _| {
                    last_audio_ms.store(now_ms() as u64, Ordering::Relaxed);
                    let mut frames = Vec::with_capacity(data.len() / channels.max(1));
                    for frame in data.chunks(channels) {
                        let mono = frame
                            .iter()
                            .map(|sample| *sample as f32 / i16::MAX as f32)
                            .sum::<f32>()
                            / frame.len() as f32;
                        note_audio_sample(mono, &last_voice_ms, &heard_speech, &peak_level);
                        frames.push(mono);
                    }
                    store_frames(&samples, &store, &frames);
                },
                error_callback,
                None,
            )
        }
        cpal::SampleFormat::U16 => {
            let samples = Arc::clone(&samples);
            let last_voice_ms = Arc::clone(&last_voice_ms);
            let heard_speech = Arc::clone(&heard_speech);
            let peak_level = Arc::clone(&peak_level);
            let store = Arc::clone(&store);
            let last_audio_ms = Arc::clone(&last_audio_ms);
            device.build_input_stream(
                &config,
                move |data: &[u16], _| {
                    last_audio_ms.store(now_ms() as u64, Ordering::Relaxed);
                    let mut frames = Vec::with_capacity(data.len() / channels.max(1));
                    for frame in data.chunks(channels) {
                        let mono = frame
                            .iter()
                            .map(|sample| (*sample as f32 / u16::MAX as f32) * 2.0 - 1.0)
                            .sum::<f32>()
                            / frame.len() as f32;
                        note_audio_sample(mono, &last_voice_ms, &heard_speech, &peak_level);
                        frames.push(mono);
                    }
                    store_frames(&samples, &store, &frames);
                },
                error_callback,
                None,
            )
        }
        format => return Err(format!("Unsupported microphone sample format: {format:?}")),
    }
    .map_err(|error| format!("Could not open microphone: {error}"))?;
    stream
        .play()
        .map_err(|error| format!("Could not start microphone: {error}"))?;
    Ok((stream, sample_rate, device.name().unwrap_or_default()))
}

/// The device a Start asks for, by name: the named one, or whichever is the
/// Windows default right now (it can change between takes).
#[cfg(windows)]
fn requested_device_name(device_name: &str) -> Option<String> {
    if !device_name.trim().is_empty() {
        return Some(device_name.to_string());
    }
    cpal::default_host().default_input_device()?.name().ok()
}

#[cfg(windows)]
fn now_ms() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0)
}

/// An open microphone stream kept after a take (Handy's lazy close), so the
/// next take starts at once instead of waiting for WASAPI to open.
#[cfg(windows)]
struct WarmStream {
    stream: cpal::Stream,
    rate: u32,
    device: String,
    idle_since_ms: u128,
}

#[cfg(windows)]
fn audio_thread_main(commands: mpsc::Receiver<AudioCommand>, app: AppHandle) {
    let samples = Arc::new(Mutex::new(Vec::<f32>::new()));
    let last_voice_ms = Arc::new(Mutex::new(now_ms()));
    let heard_speech = Arc::new(Mutex::new(false));
    let peak_level = Arc::new(Mutex::new(0.0_f32));
    // Whether the callback keeps samples (a take) or only meters (warm).
    let store = Arc::new(AtomicBool::new(false));
    // When the stream last delivered audio; a warm stream that went quiet
    // (sleep, unplugged mic) is not reused.
    let last_audio_ms = Arc::new(std::sync::atomic::AtomicU64::new(0));
    let mut warm: Option<WarmStream> = None;
    let mut stream_device = String::new();
    let mut stream: Option<cpal::Stream> = None;
    let mut sample_rate: Option<u32> = None;
    let mut started_ms: Option<u128> = None;
    let mut silence_seconds = 1.5_f32;
    let mut max_seconds = 60.0_f32;
    let mut meter_only = false;
    let mut silence_auto_stop = false;
    let mut current_session = 0_u64;

    loop {
        let timed_out = match commands.recv_timeout(std::time::Duration::from_millis(100)) {
            Ok(command) => {
                match command {
                    AudioCommand::Start {
                        silence_seconds: silence,
                        max_seconds: max,
                        device_name,
                        silence_auto_stop: enable_silence,
                        meter_only: meter,
                        session,
                        reply,
                    } => {
                        meter_only = meter_only_after_start(stream.is_some(), meter, meter_only);
                        let result = if stream.is_some() {
                            Err("A recording is already in progress".into())
                        } else {
                            silence_seconds = silence.clamp(0.3, 10.0);
                            max_seconds = max.clamp(3.0, 300.0);
                            silence_auto_stop = enable_silence;
                            current_session = session;
                            // The mic test always gets its own meter stream.
                            // Compared by the device's real name, so a new
                            // Windows default is not served by the old stream.
                            let reusable = warm.take().filter(|kept| {
                                !meter
                                    && requested_device_name(&device_name).is_some_and(|wanted| {
                                        reuse_warm_stream(
                                            &kept.device,
                                            &wanted,
                                            last_audio_ms.load(Ordering::Relaxed) as u128,
                                            now_ms(),
                                        )
                                    })
                            });
                            let opened = match reusable {
                                Some(kept) => {
                                    if let Ok(mut buffer) = samples.lock() {
                                        buffer.clear();
                                        store.store(true, Ordering::Relaxed);
                                    }
                                    logbuf::debug("Microphone was still open; recording at once.");
                                    Ok((kept.stream, kept.rate, kept.device))
                                }
                                None => {
                                    // Storing is on before the stream starts,
                                    // so its first frames are kept.
                                    store.store(!meter, Ordering::Relaxed);
                                    open_input_stream(
                                        Arc::clone(&samples),
                                        Arc::clone(&last_voice_ms),
                                        Arc::clone(&heard_speech),
                                        Arc::clone(&peak_level),
                                        Arc::clone(&store),
                                        Arc::clone(&last_audio_ms),
                                        &device_name,
                                    )
                                    .inspect(|_| logbuf::debug("Microphone opened."))
                                }
                            };
                            match opened {
                                Ok((next_stream, rate, resolved)) => {
                                    stream_device = resolved;
                                    sample_rate = Some(rate);
                                    stream = Some(next_stream);
                                    started_ms = Some(now_ms());
                                    *last_voice_ms.lock().unwrap() = now_ms();
                                    *heard_speech.lock().unwrap() = false;
                                    Ok(())
                                }
                                Err(error) => {
                                    store.store(false, Ordering::Relaxed);
                                    meter_only = false;
                                    Err(error)
                                }
                            }
                        };
                        // Reply before any UI work. Tray/emit stay on the
                        // command side so a blocked start_recording cannot
                        // deadlock the event loop.
                        let _ = reply.send(result);
                    }
                    AudioCommand::Stop { reply } => {
                        let meter = meter_only;
                        meter_only = false;
                        let result = if meter {
                            store.store(false, Ordering::Relaxed);
                            stream.take();
                            sample_rate.take();
                            started_ms = None;
                            if let Ok(mut buffer) = samples.lock() {
                                buffer.clear();
                            }
                            Ok((Vec::new(), 16_000))
                        } else {
                            take_recording(
                                &mut stream,
                                &mut warm,
                                &stream_device,
                                &store,
                                &mut sample_rate,
                                &mut started_ms,
                                &samples,
                            )
                        };
                        if let Ok(mut peak) = peak_level.lock() {
                            *peak = 0.0;
                        }
                        let _ = reply.send(result);
                    }
                    AudioCommand::Level { reply } => {
                        let level = peak_level.lock().map(|v| *v).unwrap_or(0.0);
                        let _ = reply.send(level);
                    }
                    AudioCommand::ReleaseMicrophone => {
                        if warm.take().is_some() {
                            logbuf::debug("Microphone closed.");
                        }
                    }
                    AudioCommand::Snapshot { seconds, reply } => {
                        let snapshot = if stream.is_some() && !meter_only {
                            samples.lock().ok().map(|buffer| {
                                let rate = sample_rate.unwrap_or(16_000);
                                let start = tail_start(buffer.len(), rate, seconds);
                                (buffer[start..].to_vec(), rate)
                            })
                        } else {
                            None
                        };
                        let _ = reply.send(snapshot);
                    }
                    AudioCommand::IsLive { reply } => {
                        let _ = reply.send(stream.is_some() && !meter_only);
                    }
                }
                false
            }
            Err(mpsc::RecvTimeoutError::Timeout) => true,
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        };

        if warm
            .as_ref()
            .is_some_and(|kept| now_ms().saturating_sub(kept.idle_since_ms) >= WARM_MICROPHONE_MS)
        {
            warm = None;
            logbuf::debug("Microphone closed after staying open unused.");
        }

        if timed_out && stream.is_some() && !meter_only {
            let started = started_ms.unwrap_or_else(now_ms);
            let elapsed = (now_ms().saturating_sub(started)) as f32 / 1000.0;
            let last_voice = *last_voice_ms.lock().unwrap_or_else(|e| e.into_inner());
            let quiet_for = (now_ms().saturating_sub(last_voice)) as f32 / 1000.0;
            let heard = *heard_speech.lock().unwrap_or_else(|e| e.into_inner());
            let silence_hit = silence_auto_stop && heard && quiet_for >= silence_seconds;
            let max_hit = elapsed >= max_seconds;
            if silence_hit || max_hit {
                match take_recording(
                    &mut stream,
                    &mut warm,
                    &stream_device,
                    &store,
                    &mut sample_rate,
                    &mut started_ms,
                    &samples,
                ) {
                    Ok((pcm, rate)) => {
                        let app_for_finish = app.clone();
                        let session = current_session;
                        std::thread::spawn(move || {
                            finish_captured_audio(&app_for_finish, pcm, rate, session);
                        });
                    }
                    Err(_) => {
                        // take_recording already ended the take.
                        clear_recording_after_capture_drop(&app);
                    }
                }
            }
        }
    }
}

/// Ends the take and keeps its stream open, no longer storing, for the next.
#[cfg(windows)]
fn take_recording(
    stream: &mut Option<cpal::Stream>,
    warm: &mut Option<WarmStream>,
    device: &str,
    store: &AtomicBool,
    sample_rate: &mut Option<u32>,
    started_ms: &mut Option<u128>,
    samples: &Arc<Mutex<Vec<f32>>>,
) -> Result<(Vec<f32>, u32), String> {
    let Some(live) = stream.take() else {
        store.store(false, Ordering::Relaxed);
        return Err("No recording is in progress".into());
    };
    *started_ms = None;
    let rate = sample_rate.take().unwrap_or(16_000);
    *warm = Some(WarmStream {
        stream: live,
        rate,
        device: device.to_string(),
        idle_since_ms: now_ms(),
    });
    match samples.lock() {
        Ok(mut buffer) => {
            // Under the buffer lock, so no callback appends after the hand-off.
            store.store(false, Ordering::Relaxed);
            let captured = std::mem::take(&mut *buffer);
            if captured.is_empty() {
                Err("No microphone audio was captured".into())
            } else {
                Ok((captured, rate))
            }
        }
        Err(_) => Err("Audio lock was poisoned".into()),
    }
}

#[cfg(windows)]
impl AudioRecorder {
    fn new(app: AppHandle) -> Self {
        let (commands, receiver) = mpsc::channel();
        std::thread::Builder::new()
            .name("vocawin-audio".into())
            .spawn(move || audio_thread_main(receiver, app))
            .expect("Could not start VocaWin audio thread");
        Self { commands }
    }

    fn start(
        &self,
        silence_seconds: f32,
        max_seconds: f32,
        device_name: String,
        silence_auto_stop: bool,
        session: u64,
    ) -> Result<(), String> {
        let (reply, response) = mpsc::channel();
        self.commands
            .send(AudioCommand::Start {
                silence_seconds,
                max_seconds,
                device_name,
                silence_auto_stop,
                meter_only: false,
                session,
                reply,
            })
            .map_err(|_| "Audio thread is not running".to_string())?;
        match recv_audio_reply(response, AUDIO_REPLY_TIMEOUT, "start") {
            Ok(result) => result,
            Err(error) => {
                logbuf::error(error.clone());
                let (stop_reply, stop_rx) = mpsc::channel();
                let _ = self.commands.send(AudioCommand::Stop { reply: stop_reply });
                let _ = stop_rx.recv_timeout(std::time::Duration::from_millis(400));
                Err(error)
            }
        }
    }

    fn start_meter(&self, device_name: String) -> Result<(), String> {
        let (reply, response) = mpsc::channel();
        self.commands
            .send(AudioCommand::Start {
                silence_seconds: 10.0,
                max_seconds: 300.0,
                device_name,
                silence_auto_stop: false,
                meter_only: true,
                session: 0,
                reply,
            })
            .map_err(|_| "Audio thread is not running".to_string())?;
        match recv_audio_reply(response, AUDIO_REPLY_TIMEOUT, "start") {
            Ok(result) => result,
            Err(error) => {
                logbuf::error(error.clone());
                let (stop_reply, stop_rx) = mpsc::channel();
                let _ = self.commands.send(AudioCommand::Stop { reply: stop_reply });
                let _ = stop_rx.recv_timeout(std::time::Duration::from_millis(400));
                Err(error)
            }
        }
    }

    fn stop(&self) -> Result<(Vec<f32>, u32), String> {
        let (reply, response) = mpsc::channel();
        self.commands
            .send(AudioCommand::Stop { reply })
            .map_err(|_| "Audio thread is not running".to_string())?;
        recv_audio_reply(response, AUDIO_REPLY_TIMEOUT, "stop")?
    }

    fn level(&self) -> f32 {
        let (reply, response) = mpsc::channel();
        if self.commands.send(AudioCommand::Level { reply }).is_err() {
            return 0.0;
        }
        response
            .recv_timeout(std::time::Duration::from_millis(200))
            .unwrap_or(0.0)
    }

    /// Closes a microphone kept open after a take.
    fn release_microphone(&self) {
        let _ = self.commands.send(AudioCommand::ReleaseMicrophone);
    }

    fn capture_live(&self) -> bool {
        let (reply, response) = mpsc::channel();
        if self.commands.send(AudioCommand::IsLive { reply }).is_err() {
            return false;
        }
        response
            .recv_timeout(std::time::Duration::from_millis(200))
            .unwrap_or(false)
    }

    /// The last `seconds` of the take in progress, if one is recording.
    fn snapshot(&self, seconds: f32) -> Option<(Vec<f32>, u32)> {
        let (reply, response) = mpsc::channel();
        self.commands
            .send(AudioCommand::Snapshot { seconds, reply })
            .ok()?;
        response
            .recv_timeout(std::time::Duration::from_millis(500))
            .ok()
            .flatten()
    }
}

/// Non-Windows builds keep an unavailable mic stub so Linux/macOS CI can validate
/// the shared UI and command layer. Real capture lives behind `cfg(windows)`.
#[cfg(not(windows))]
struct AudioRecorder;
#[cfg(not(windows))]
impl AudioRecorder {
    fn new(_: AppHandle) -> Self {
        Self
    }
    fn start(&self, _: f32, _: f32, _: String, _: bool, _: u64) -> Result<(), String> {
        Err("Microphone capture is available in Windows builds only.".into())
    }
    fn start_meter(&self, _: String) -> Result<(), String> {
        Err("Microphone capture is available in Windows builds only.".into())
    }
    fn stop(&self) -> Result<(Vec<f32>, u32), String> {
        Err("Microphone capture is available in Windows builds only.".into())
    }
    fn level(&self) -> f32 {
        0.0
    }
    fn capture_live(&self) -> bool {
        false
    }
    fn release_microphone(&self) {}
    fn snapshot(&self, _: f32) -> Option<(Vec<f32>, u32)> {
        None
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
enum ParkReason {
    #[default]
    None,
    IdleTimeout,
    AutoPause(String),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TrayPhase {
    Idle,
    Listening,
    Processing,
    Parked,
}

fn tray_park_tooltip(reason: &ParkReason) -> String {
    match reason {
        ParkReason::IdleTimeout => "VocaWin - Unloaded to save RAM (idle timeout)".into(),
        ParkReason::AutoPause(app) => {
            format!("VocaWin - Paused because {app} is running")
        }
        ParkReason::None => "VocaWin".into(),
    }
}

fn set_tray_phase(app: &AppHandle, phase: TrayPhase) {
    if let Some(tray) = app.tray_by_id("main") {
        let park = app
            .try_state::<AppState>()
            .and_then(|state| state.park_reason.lock().ok().map(|guard| guard.clone()))
            .unwrap_or_default();
        let (tip, bytes) = match phase {
            TrayPhase::Idle => (
                "VocaWin".into(),
                include_bytes!("../icons/tray-idle.png").as_slice(),
            ),
            TrayPhase::Listening => (
                "VocaWin - Listening".into(),
                include_bytes!("../icons/tray-listening.png").as_slice(),
            ),
            TrayPhase::Processing => (
                "VocaWin - Processing".into(),
                include_bytes!("../icons/tray-processing.png").as_slice(),
            ),
            TrayPhase::Parked => (
                tray_park_tooltip(&park),
                include_bytes!("../icons/tray-parked.png").as_slice(),
            ),
        };
        let _ = tray.set_tooltip(Some(tip.as_str()));
        if let Ok(icon) = tauri::image::Image::from_bytes(bytes) {
            let _ = tray.set_icon(Some(icon));
        }
        if let (Some(window), Ok(icon)) = (
            app.get_webview_window("main"),
            tauri::image::Image::from_bytes(bytes),
        ) {
            let _ = window.set_icon(icon);
        }
    }
    let _ = refresh_tray_menu(app);
}

fn apply_ready_or_parked_tray(app: &AppHandle) {
    let parked = app
        .try_state::<AppState>()
        .and_then(|state| {
            let reason = state.park_reason.lock().ok()?.clone();
            Some(reason != ParkReason::None)
        })
        .unwrap_or(false);
    set_tray_phase(
        app,
        if parked {
            TrayPhase::Parked
        } else {
            TrayPhase::Idle
        },
    );
}

/// Silence or the max-recording limit ended the take on the audio thread.
#[cfg(windows)]
/// `session` is the id the take was started with, carried from the audio
/// thread: another take may have started by the time this runs.
fn finish_captured_audio(app: &AppHandle, samples: Vec<f32>, sample_rate: u32, session: u64) {
    let state = app.state::<AppState>();
    state.processing.store(true, Ordering::SeqCst);
    // A newer take may already be recording; its flag and indicator are not
    // this take's to clear.
    if state.session_id.load(Ordering::SeqCst) == session {
        set_recording_flag(&state, false);
        let _ = app.emit("recording-changed", false);
    }
    let sound = state
        .settings
        .lock()
        .map(|settings| settings.sound_theme.clone())
        .unwrap_or_else(|_| "voca".into());
    sounds::play_if_enabled(&sound, false);
    let inject = session_injects(&state);
    match complete_take(app, samples, sample_rate, true, session) {
        Ok(text) => {
            let event = if inject {
                "dictation-finished"
            } else {
                "test-dictation-finished"
            };
            let _ = app.emit(event, text);
        }
        Err(error) => {
            sounds::play_error_if_enabled(&sound);
            logbuf::error_and_emit(app, format!("Dictation error: {error}"));
            let _ = app.emit("dictation-error", error);
        }
    }
}

fn session_injects(state: &AppState) -> bool {
    *state
        .inject_on_auto_stop
        .lock()
        .unwrap_or_else(|e| e.into_inner())
}

fn overlay_on(settings: &Settings) -> bool {
    settings.overlay_style != "off"
}

/// Everything after the microphone closed: ducking, the pill, transcription,
/// the cancel check, typing, and stats. With `type_it` false the text is
/// handed back instead (the VocaWin window types it itself).
fn complete_take(
    app: &AppHandle,
    samples: Vec<f32>,
    sample_rate: u32,
    type_it: bool,
    session: u64,
) -> Result<String, String> {
    // The preview must let go of the model before the take decodes.
    live_preview::stop();
    let state = app.state::<AppState>();
    let settings = state.settings.lock().map(|s| s.clone()).unwrap_or_default();
    let inject = session_injects(&state);
    state.ducker.restore();
    state.processing.store(true, Ordering::SeqCst);
    set_tray_phase(app, TrayPhase::Processing);
    if inject {
        overlay::show(app, overlay::Phase::Processing, overlay_on(&settings));
    }
    let result = transcribe_samples(&state, samples, sample_rate);
    // Decide on cancellation while still marked as transcribing, so an
    // Escape that lands now is either seen here or finds nothing to stop.
    let cancelled = cancellations(&state).settle(session);
    state.processing.store(false, Ordering::SeqCst);
    let recording_again = *state.recording.lock().unwrap_or_else(|e| e.into_inner());
    hook::disarm_cancel_for(session);
    let outcome = match result {
        Ok(take) if cancelled => {
            if let Some(id) = take.history_id {
                let _ = state.history.set_status(id, history::STATUS_CANCELLED);
            }
            logbuf::info_and_emit(app, "Cancelled take was not typed.");
            Ok(String::new())
        }
        Ok(take) => {
            // A take with no words is an error, not a silent no-op, so the
            // overlay, the error sound and the window all say so.
            let mut delivered = if take.text.trim().is_empty() {
                Err(NO_SPEECH.to_string())
            } else {
                Ok(take.text.clone())
            };
            if inject && !take.text.is_empty() {
                if let Ok(mut last) = state.last_dictation.lock() {
                    *last = take.text.clone();
                }
                if type_it {
                    match inject_transcript(&state, &take.text) {
                        Ok(()) => record_stats(&state, &take.text, take.speech_ms),
                        Err(error) => delivered = Err(format!("Could not type the text: {error}")),
                    }
                } else if let Ok(mut pending) = state.pending_window_take.lock() {
                    // The window types this itself (`inject_text`); count it then.
                    *pending = Some((take.text.clone(), take.speech_ms));
                }
            }
            match &delivered {
                Err(error) if inject => {
                    overlay::show(app, overlay::Phase::Error(error.clone()), overlay_on(&settings))
                }
                _ if inject && !recording_again => overlay::hide(app),
                _ => {}
            }
            delivered
        }
        Err(error) => {
            if inject {
                overlay::show(app, overlay::Phase::Error(error.clone()), overlay_on(&settings));
            }
            Err(error)
        }
    };
    let _ = app.emit("history-changed", ());
    apply_ready_or_parked_tray(app);
    outcome
}

/// The session ended without audio to transcribe: put the chrome back.
fn release_session_chrome(app: &AppHandle) {
    live_preview::stop();
    let state = app.state::<AppState>();
    state.ducker.restore();
    state.processing.store(false, Ordering::SeqCst);
    hook::disarm_cancel_for(state.session_id.load(Ordering::SeqCst));
    if session_injects(&state) {
        overlay::hide(app);
    }
    apply_ready_or_parked_tray(app);
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ModelInstallStatus {
    installed: bool,
    downloadable: bool,
    downloading: bool,
    progress: u8,
    message: Option<String>,
    #[serde(default)]
    bytes_on_disk: u64,
}

struct AppState {
    settings: Mutex<Settings>,
    settings_path: PathBuf,
    history: history::HistoryStore,
    stats_path: PathBuf,
    models_path: PathBuf,
    downloads: Mutex<HashMap<String, ModelInstallStatus>>,
    recorder: AudioRecorder,
    recording: Mutex<bool>,
    /// True while WASAPI is opening. Release during this window is not lost.
    session_opening: AtomicBool,
    release_during_open: AtomicBool,
    /// When false, silence/max auto-stop must not inject (Test Dictation).
    inject_on_auto_stop: Mutex<bool>,
    registered_hotkey: Mutex<String>,
    dictation_paused: Mutex<bool>,
    park_reason: Mutex<ParkReason>,
    /// Last poll of Whisper residency, used to spot idle unload.
    saw_model_loaded: Mutex<bool>,
    whisper_cache: whisper_cache::WhisperCache,
    /// How the running (or last) session started.
    session_trigger: Mutex<Trigger>,
    /// True while a finished take is being transcribed.
    processing: AtomicBool,
    /// Id of the newest take, bumped when one starts.
    session_id: AtomicU64,
    /// Which takes Escape cancelled and which already committed to typing.
    /// One lock decides between the two, so Escape either stops a take or
    /// finds it too late, never both.
    cancellations: Mutex<Cancellations>,
    /// For paste-last; kept even when history is off.
    last_dictation: Mutex<String>,
    /// A take the VocaWin window will type itself: counted in stats when
    /// `inject_text` succeeds with this text.
    pending_window_take: Mutex<Option<(String, u64)>>,
    ducker: ducking::Ducker,
}

/// A malformed or partially-written settings file must never prevent dictation
/// from starting. In that case we retain the file for diagnosis and use safe
/// defaults until the user saves settings again.
fn load_settings(path: &std::path::Path) -> Settings {
    let contents = fs::read_to_string(path).unwrap_or_default();
    let mut settings: Settings = serde_json::from_str(&contents).unwrap_or_default();
    sounds::apply_theme(&mut settings.sound_theme, &mut settings.sound_effects);
    settings
}

fn persist_settings(path: &std::path::Path, settings: &Settings) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or("Settings path has no parent directory")?;
    fs::create_dir_all(parent)
        .map_err(|error| format!("Could not create settings directory: {error}"))?;
    let serialized = serde_json::to_vec_pretty(settings)
        .map_err(|error| format!("Could not serialize settings: {error}"))?;
    let temporary = path.with_extension("json.tmp");
    fs::write(&temporary, serialized)
        .map_err(|error| format!("Could not write settings: {error}"))?;
    // `fs::rename` replaces the existing file in one step on Windows
    // (MoveFileEx with REPLACE_EXISTING), so a crash never leaves no
    // settings.json behind.
    fs::rename(temporary, path).map_err(|error| format!("Could not finalize settings: {error}"))
}

#[tauri::command]
fn get_settings(state: State<'_, AppState>) -> Result<Settings, String> {
    state
        .settings
        .lock()
        .map(|settings| settings.clone())
        .map_err(|_| "Settings lock was poisoned".into())
}

#[tauri::command]
fn save_settings(
    mut settings: Settings,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    if !model_catalog()
        .iter()
        .any(|model| model.id == settings.selected_model)
    {
        return Err("Unknown speech model".into());
    }
    if !(0.3..=10.0).contains(&settings.silence_seconds) {
        return Err("Silence timeout must be between 0.3 and 10 seconds".into());
    }
    if !(3.0..=300.0).contains(&settings.max_recording_seconds) {
        return Err("Max recording duration must be between 3 and 300 seconds".into());
    }
    if settings.activation_mode != "pushToTalk" && settings.activation_mode != "toggle" {
        return Err("Activation mode must be pushToTalk or toggle".into());
    }
    if settings.idle_unload_enabled && !(30..=3600).contains(&settings.idle_unload_seconds) {
        return Err("Idle unload must be between 30 and 3600 seconds".into());
    }
    if settings.auto_pause_apps.trim().is_empty() {
        settings.auto_pause_enabled = false;
    } else {
        settings.auto_pause_enabled = true;
    }
    sounds::apply_theme(&mut settings.sound_theme, &mut settings.sound_effects);
    settings.hotkey = hotkey::canonicalize(&settings.hotkey)?;
    normalize_extra_settings(&mut settings)?;
    let (previous_launch, previous_model) = {
        let previous = state
            .settings
            .lock()
            .map_err(|_| "Settings lock was poisoned")?;
        (previous.launch_at_login, previous.selected_model.clone())
    };
    if previous_model != settings.selected_model {
        // Free the old model now rather than at the next take.
        ONNX_MODELS.unload();
    }
    persist_settings(&state.settings_path, &settings)?;
    let launch_error = match apply_launch_at_login(&app, settings.launch_at_login) {
        Ok(()) => None,
        Err(error) => {
            settings.launch_at_login = previous_launch;
            persist_settings(&state.settings_path, &settings)?;
            Some(error)
        }
    };
    // Disk is the source of truth after persist. Refresh AppState before
    // hotkey so a later side-effect error cannot leave start_recording
    // and the UI on different selected models.
    *state
        .settings
        .lock()
        .map_err(|_| "Settings lock was poisoned")? = settings.clone();
    state
        .whisper_cache
        .configure_idle(settings.idle_unload_enabled, settings.idle_unload_seconds);
    if !settings.idle_unload_enabled {
        // Never keeps the model in RAM. Only clear an idle-park banner.
        let mut park = state
            .park_reason
            .lock()
            .map_err(|_| "Park lock was poisoned")?;
        if *park == ParkReason::IdleTimeout {
            *park = ParkReason::None;
        }
    }
    logbuf::set_debug_enabled(settings.debug_logging);
    apply_extra_shortcuts(&settings);
    if let Err(error) = state.history.prune(settings.history_retention_days) {
        logbuf::warn(error);
    }
    if !overlay_on(&settings) {
        overlay::hide(&app);
    }
    logbuf::debug(format!(
        "Settings saved (model {}, hotkey {}, debug={})",
        settings.selected_model, settings.hotkey, settings.debug_logging
    ));
    let paused = *state
        .dictation_paused
        .lock()
        .map_err(|_| "Pause lock was poisoned")?;
    if !paused {
        register_dictation_hotkey(&app, &settings.hotkey)?;
    }
    *state
        .registered_hotkey
        .lock()
        .map_err(|_| "Hotkey lock was poisoned")? = settings.hotkey.clone();
    emit_runtime(&app);
    apply_ready_or_parked_tray(&app);
    if let Some(error) = launch_error {
        let _ = app.emit("settings-changed", settings);
        return Err(error);
    }
    Ok(())
}

/// Longest dictionary or snippet list kept.
const MAX_DICTIONARY_ENTRIES: usize = 500;

/// Checks and tidies the settings added for VocaMac parity: shortcuts,
/// overlay, history retention, text rules, dictionary, and snippets.
fn normalize_extra_settings(settings: &mut Settings) -> Result<(), String> {
    if !matches!(settings.cleanup_level.as_str(), "none" | "medium") {
        return Err("Cleanup must be none or medium".into());
    }
    if !matches!(settings.overlay_style.as_str(), "minimal" | "off") {
        return Err("Recording overlay must be minimal or off".into());
    }
    if !matches!(settings.overlay_position.as_str(), "bottom" | "top") {
        return Err("Overlay position must be bottom or top".into());
    }
    if !matches!(settings.mouse_button.as_str(), "" | "middle" | "x1" | "x2") {
        return Err("Mouse button must be middle, x1, x2, or none".into());
    }
    if !matches!(settings.insertion_mode.as_str(), "type" | "paste") {
        return Err("Text must be typed or pasted".into());
    }
    if !matches!(settings.history_retention_days, 0 | 1 | 7 | 30) {
        return Err("Keep history for 1, 7, or 30 days, or forever".into());
    }
    settings.hands_free_hotkey = validate_extra_shortcut(
        &settings.hands_free_hotkey,
        "Hands-free shortcut",
        &[&settings.hotkey],
    )?;
    settings.paste_last_hotkey = validate_extra_shortcut(
        &settings.paste_last_hotkey,
        "Paste-last shortcut",
        &[&settings.hotkey, &settings.hands_free_hotkey],
    )?;
    let mut replacements: Vec<dictionary::Replacement> = settings
        .replacements
        .iter()
        .map(|entry| dictionary::Replacement {
            heard: entry.heard.trim().to_string(),
            replacement: entry.replacement.trim().to_string(),
        })
        .filter(|entry| !entry.heard.is_empty() && !entry.replacement.is_empty())
        .collect();
    replacements.truncate(MAX_DICTIONARY_ENTRIES);
    settings.replacements = replacements;
    let mut snippets: Vec<dictionary::Snippet> = settings
        .snippets
        .iter()
        .map(|entry| dictionary::Snippet {
            trigger: entry.trigger.trim().to_string(),
            expansion: entry.expansion.clone(),
        })
        .filter(|entry| !entry.trigger.is_empty() && !entry.expansion.trim().is_empty())
        .collect();
    snippets.truncate(MAX_DICTIONARY_ENTRIES);
    settings.snippets = snippets;
    Ok(())
}

/// Plugin disable() can fail when the login item or .desktop file is already
/// gone ("no such file" / Windows ERROR_FILE_NOT_FOUND). That is
/// already-disabled, not a failed settings save.
#[cfg(any(test, not(windows)))]
fn autostart_disable_error_is_missing(error: impl std::fmt::Display) -> bool {
    let text = error.to_string().to_ascii_lowercase();
    text.contains("os error 2")
        || text.contains("cannot find the file specified")
        || text.contains("no such file or directory")
}

fn apply_launch_at_login(app: &AppHandle, enabled: bool) -> Result<(), String> {
    #[cfg(windows)]
    {
        let _ = app;
        autostart::apply(enabled).map_err(|error| {
            if enabled {
                format!("Could not enable launch at login: {error}")
            } else {
                format!("Could not disable launch at login: {error}")
            }
        })
    }
    #[cfg(not(windows))]
    {
        use tauri_plugin_autostart::ManagerExt;
        let autolaunch = app.autolaunch();
        if enabled {
            autolaunch
                .enable()
                .map_err(|error| format!("Could not enable launch at login: {error}"))
        } else {
            match autolaunch.disable() {
                Ok(()) => Ok(()),
                Err(error) if autostart_disable_error_is_missing(&error) => Ok(()),
                Err(error) => Err(format!("Could not disable launch at login: {error}")),
            }
        }
    }
}

/// When the saved selection is unknown or not on disk, and whisper-tiny is
/// installed, use whisper-tiny. Returns true if selected_model changed.
fn fallback_selected_model_if_needed(settings: &mut Settings, models_path: &Path) -> bool {
    let selected_ok = model_catalog()
        .iter()
        .any(|model| model.id == settings.selected_model)
        && model_is_installed(models_path, &settings.selected_model);
    if selected_ok {
        return false;
    }
    if !model_is_installed(models_path, "whisper-tiny") {
        return false;
    }
    settings.selected_model = "whisper-tiny".into();
    true
}

#[tauri::command]
fn get_history(state: State<'_, AppState>) -> Vec<history::HistoryEntry> {
    state.history.load()
}

#[tauri::command]
fn clear_history(state: State<'_, AppState>) -> Result<(), String> {
    sounds::stop_file();
    state.history.clear()
}

#[tauri::command]
fn delete_history_entry(id: u64, state: State<'_, AppState>) -> Result<(), String> {
    sounds::stop_file();
    state.history.delete(id as u128)
}

/// Transcribe a saved take again with the current model and settings. The
/// entry's text is replaced; nothing is typed.
#[tauri::command]
async fn retry_history_entry(id: u64, app: AppHandle) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<AppState>();
        let id = id as u128;
        let path = state
            .history
            .audio_path(id)
            .ok_or("This take has no saved audio to retry.")?;
        let pcm = history::read_wav(&path)?;
        let settings = state
            .settings
            .lock()
            .map_err(|_| "Settings lock was poisoned")?
            .clone();
        if !model_is_installed(&state.models_path, &settings.selected_model) {
            return Err("Install the selected speech model before retrying.".into());
        }
        logbuf::debug_and_emit(&app, format!("Retry history take with {}", settings.selected_model));
        let result = recognize_and_format(&state, &settings, &pcm);
        match &result {
            Ok(text) if !text.trim().is_empty() => state.history.finish(
                id,
                text,
                &settings.selected_model,
                history::STATUS_OK,
                None,
            )?,
            Ok(_) => state.history.finish(
                id,
                "",
                &settings.selected_model,
                history::STATUS_FAILED,
                Some("No speech was recognized.".into()),
            )?,
            Err(error) => state.history.finish(
                id,
                "",
                &settings.selected_model,
                history::STATUS_FAILED,
                Some(error.clone()),
            )?,
        }
        result
    })
    .await
    .map_err(|error| format!("Retry was cancelled: {error}"))?
}

#[tauri::command]
fn play_history_audio(id: u64, state: State<'_, AppState>) -> Result<(), String> {
    let path = state
        .history
        .audio_path(id as u128)
        .ok_or("This take has no saved audio.")?;
    sounds::play_file(&path)
}

#[tauri::command]
fn stop_history_audio() {
    sounds::stop_file();
}

#[tauri::command]
fn get_stats(state: State<'_, AppState>) -> stats::Summary {
    stats::summary(&state.stats_path)
}

#[tauri::command]
fn reset_stats(state: State<'_, AppState>) -> Result<(), String> {
    stats::reset(&state.stats_path)
}

/// Run text through the same rules a dictation gets, for the Try box.
#[tauri::command]
fn preview_text(text: String, state: State<'_, AppState>) -> Result<String, String> {
    let settings = state
        .settings
        .lock()
        .map_err(|_| "Settings lock was poisoned")?
        .clone();
    Ok(format_transcript(&settings, &text))
}

/// Settings backup: a JSON file in Downloads. There are no secrets in it.
#[tauri::command]
fn export_settings(app: AppHandle, state: State<'_, AppState>) -> Result<String, String> {
    let settings = state
        .settings
        .lock()
        .map_err(|_| "Settings lock was poisoned")?
        .clone();
    let folder = app
        .path()
        .download_dir()
        .or_else(|_| app.path().document_dir())
        .map_err(|error| format!("Could not find the Downloads folder: {error}"))?;
    fs::create_dir_all(&folder)
        .map_err(|error| format!("Could not open the Downloads folder: {error}"))?;
    let stamp = chrono::Local::now().format("%Y%m%d-%H%M%S");
    let path = folder.join(format!("vocawin-settings-{stamp}.json"));
    let mut value = serde_json::to_value(&settings)
        .map_err(|error| format!("Could not export settings: {error}"))?;
    if let Some(object) = value.as_object_mut() {
        // First-run state belongs to this PC, not to the backup.
        object.remove("welcomeDismissed");
        object.insert("vocawinSettingsVersion".into(), 1.into());
    }
    let serialized = serde_json::to_vec_pretty(&value)
        .map_err(|error| format!("Could not export settings: {error}"))?;
    fs::write(&path, serialized).map_err(|error| format!("Could not export settings: {error}"))?;
    Ok(path.display().to_string())
}

/// Parse a backup from `export_settings` into full settings for this PC. The
/// caller saves them, so every value goes through `save_settings` checks.
#[tauri::command]
fn parse_settings_backup(contents: String, state: State<'_, AppState>) -> Result<Settings, String> {
    let mut value: serde_json::Value = serde_json::from_str(&contents)
        .map_err(|_| "That file is not a VocaWin settings backup.".to_string())?;
    let object = value
        .as_object_mut()
        .filter(|object| object.contains_key("vocawinSettingsVersion"))
        .ok_or("That file is not a VocaWin settings backup.")?;
    object.remove("vocawinSettingsVersion");
    let current = state
        .settings
        .lock()
        .map_err(|_| "Settings lock was poisoned")?
        .clone();
    let mut imported: Settings = serde_json::from_value(value)
        .map_err(|error| format!("Could not read the backup: {error}"))?;
    imported.welcome_dismissed = current.welcome_dismissed;
    // A model that is not on this PC would leave dictation with nothing to run.
    if !model_is_installed(&state.models_path, &imported.selected_model) {
        imported.selected_model = current.selected_model;
    }
    Ok(imported)
}

#[tauri::command]
fn get_models() -> Vec<Model> {
    model_catalog()
}

/// Oriserve's Whisper Hindi2Hinglish Apex (a Large v3 Turbo fine-tune) as a
/// whisper.cpp GGML q8_0 file. Same model as VocaMac's Voca Hinglish.
const VOCA_HINGLISH: &str = "voca-hinglish";

/// Models that run on whisper.cpp from a single GGML `.bin`.
fn is_whisper_model(id: &str) -> bool {
    id.starts_with("whisper-") || id.starts_with("distil-whisper-") || id == VOCA_HINGLISH
}

fn model_path(models_path: &Path, id: &str) -> PathBuf {
    if is_whisper_model(id) {
        models_path.join(format!("{id}.bin"))
    } else {
        models_path.join(id)
    }
}

/// How a catalog model is fetched. Archives unpack into `models/{id}/` with the
/// filenames expected by the matching `transcribe-rs` loader.
#[derive(Clone, Copy)]
enum ModelPackage {
    /// Single whisper.cpp GGML `.bin` written as `models/{id}.bin`.
    GgmlBin { url: &'static str },
    /// Official `.tar.gz` whose contents become `models/{id}/`.
    TarGz { url: &'static str },
    /// Flat files downloaded into `models/{id}/`.
    Files {
        files: &'static [(&'static str, &'static str)],
    },
}

fn model_package(id: &str) -> Option<ModelPackage> {
    match id {
        "whisper-tiny" => Some(ModelPackage::GgmlBin {
            url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-tiny.bin",
        }),
        "whisper-base" => Some(ModelPackage::GgmlBin {
            url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.bin",
        }),
        "whisper-small" => Some(ModelPackage::GgmlBin {
            url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-small.bin",
        }),
        "whisper-medium" => Some(ModelPackage::GgmlBin {
            url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-medium.bin",
        }),
        "whisper-large-v3" => Some(ModelPackage::GgmlBin {
            url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-large-v3.bin",
        }),
        "whisper-large-v3-turbo" => Some(ModelPackage::GgmlBin {
            url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-large-v3-turbo.bin",
        }),
        "distil-whisper-large-v3" => Some(ModelPackage::GgmlBin {
            url: "https://huggingface.co/distil-whisper/distil-large-v3-ggml/resolve/main/ggml-distil-large-v3.bin",
        }),
        // A third-party conversion, pinned to a commit so the file cannot
        // change under the catalog.
        VOCA_HINGLISH => Some(ModelPackage::GgmlBin {
            url: "https://huggingface.co/Marquestra/Whisper-Hindi2Hinglish-Apex-GGML/resolve/d1de3ff618856e5675c47d3158ca820506fb4d9e/ggml-apex-hinglish-q8_0.bin",
        }),
        "parakeet-tdt-0.6b-v3" => Some(ModelPackage::TarGz {
            url: "https://blob.handy.computer/parakeet-v3-int8.tar.gz",
        }),
        "moonshine-tiny" => Some(ModelPackage::Files {
            files: &[
                (
                    "encoder_model.onnx",
                    "https://huggingface.co/onnx-community/moonshine-tiny-ONNX/resolve/main/onnx/encoder_model.onnx",
                ),
                (
                    "decoder_model_merged.onnx",
                    "https://huggingface.co/onnx-community/moonshine-tiny-ONNX/resolve/main/onnx/decoder_model_merged.onnx",
                ),
                (
                    "tokenizer.json",
                    "https://huggingface.co/onnx-community/moonshine-tiny-ONNX/resolve/main/tokenizer.json",
                ),
            ],
        }),
        // istupakov's export, pinned. Its repo has no preprocessor; the
        // generic 128-bin NeMo one is byte-identical to the file in the
        // Canary 180M archive, so it comes from the Parakeet repo.
        "canary-1b-v2" => Some(ModelPackage::Files {
            files: &[
                (
                    "encoder-model.int8.onnx",
                    "https://huggingface.co/istupakov/canary-1b-v2-onnx/resolve/5ebc1520cef7b6b318b3526ad17adbfe00bc1bfc/encoder-model.int8.onnx",
                ),
                (
                    "decoder-model.int8.onnx",
                    "https://huggingface.co/istupakov/canary-1b-v2-onnx/resolve/5ebc1520cef7b6b318b3526ad17adbfe00bc1bfc/decoder-model.int8.onnx",
                ),
                (
                    "vocab.txt",
                    "https://huggingface.co/istupakov/canary-1b-v2-onnx/resolve/5ebc1520cef7b6b318b3526ad17adbfe00bc1bfc/vocab.txt",
                ),
                (
                    "nemo128.onnx",
                    "https://huggingface.co/istupakov/parakeet-tdt-0.6b-v3-onnx/resolve/8f23f0c03c8761650bdb5b40aaf3e40d2c15f1ce/nemo128.onnx",
                ),
            ],
        }),
        // The int4 export transcribe-rs documents, pinned.
        "cohere-transcribe" => Some(ModelPackage::Files {
            files: &[
                (
                    "cohere-encoder.int4.onnx",
                    "https://huggingface.co/cstr/cohere-transcribe-onnx-int4/resolve/2c870f89692c61348481884eb6e1222ec3ba25c9/cohere-encoder.int4.onnx",
                ),
                (
                    "cohere-encoder.int4.onnx.data",
                    "https://huggingface.co/cstr/cohere-transcribe-onnx-int4/resolve/2c870f89692c61348481884eb6e1222ec3ba25c9/cohere-encoder.int4.onnx.data",
                ),
                (
                    "cohere-decoder.int4.onnx",
                    "https://huggingface.co/cstr/cohere-transcribe-onnx-int4/resolve/2c870f89692c61348481884eb6e1222ec3ba25c9/cohere-decoder.int4.onnx",
                ),
                (
                    "cohere-decoder.int4.onnx.data",
                    "https://huggingface.co/cstr/cohere-transcribe-onnx-int4/resolve/2c870f89692c61348481884eb6e1222ec3ba25c9/cohere-decoder.int4.onnx.data",
                ),
                (
                    "tokens.txt",
                    "https://huggingface.co/cstr/cohere-transcribe-onnx-int4/resolve/2c870f89692c61348481884eb6e1222ec3ba25c9/tokens.txt",
                ),
            ],
        }),
        "moonshine-base" => Some(ModelPackage::TarGz {
            url: "https://blob.handy.computer/moonshine-base.tar.gz",
        }),
        "sensevoice-small" => Some(ModelPackage::TarGz {
            url: "https://blob.handy.computer/sense-voice-int8.tar.gz",
        }),
        "gigaam-v3" => Some(ModelPackage::TarGz {
            url: "https://blob.handy.computer/giga-am-v3-int8.tar.gz",
        }),
        "canary-180m" => Some(ModelPackage::TarGz {
            url: "https://blob.handy.computer/canary-180m-flash.tar.gz",
        }),
        _ => None,
    }
}

fn model_is_installed(models_path: &Path, id: &str) -> bool {
    let path = model_path(models_path, id);
    match id {
        id if is_whisper_model(id) => path.is_file(),
        "parakeet-tdt-0.6b-v3" => {
            path.join("encoder-model.int8.onnx").is_file() && path.join("vocab.txt").is_file()
        }
        "moonshine-tiny" | "moonshine-base" => {
            path.join("encoder_model.onnx").is_file()
                && path.join("decoder_model_merged.onnx").is_file()
                && path.join("tokenizer.json").is_file()
        }
        "sensevoice-small" => {
            path.join("model.int8.onnx").is_file() && path.join("tokens.txt").is_file()
        }
        "gigaam-v3" => {
            (path.join("model.int8.onnx").is_file() || path.join("model.onnx").is_file())
                && path.join("vocab.txt").is_file()
        }
        "canary-180m" => {
            path.join("encoder-model.int8.onnx").is_file() && path.join("vocab.txt").is_file()
        }
        "canary-1b-v2" => [
            "encoder-model.int8.onnx",
            "decoder-model.int8.onnx",
            "nemo128.onnx",
            "vocab.txt",
        ]
        .iter()
        .all(|file| path.join(file).is_file()),
        "cohere-transcribe" => [
            "cohere-encoder.int4.onnx",
            "cohere-encoder.int4.onnx.data",
            "cohere-decoder.int4.onnx",
            "cohere-decoder.int4.onnx.data",
            "tokens.txt",
        ]
        .iter()
        .all(|file| path.join(file).is_file()),
        _ => path.exists(),
    }
}

fn path_bytes(path: &Path) -> u64 {
    if path.is_file() {
        return fs::metadata(path).map(|meta| meta.len()).unwrap_or(0);
    }
    if !path.is_dir() {
        return 0;
    }
    let mut total = 0_u64;
    let mut stack = vec![path.to_path_buf()];
    while let Some(current) = stack.pop() {
        let Ok(entries) = fs::read_dir(&current) else {
            continue;
        };
        for entry in entries.flatten() {
            let child = entry.path();
            if child.is_dir() {
                stack.push(child);
            } else if let Ok(meta) = entry.metadata() {
                total = total.saturating_add(meta.len());
            }
        }
    }
    total
}

fn model_bytes_on_disk(models_path: &Path, id: &str) -> u64 {
    if !model_is_installed(models_path, id) {
        return 0;
    }
    path_bytes(&model_path(models_path, id))
}

fn installation_status(state: &AppState, id: &str) -> ModelInstallStatus {
    if let Ok(downloads) = state.downloads.lock() {
        if let Some(status) = downloads.get(id) {
            let mut status = status.clone();
            if status.installed && status.bytes_on_disk == 0 {
                status.bytes_on_disk = model_bytes_on_disk(&state.models_path, id);
            }
            return status;
        }
    }
    ModelInstallStatus {
        installed: model_is_installed(&state.models_path, id),
        downloadable: model_package(id).is_some(),
        downloading: false,
        progress: 0,
        message: None,
        bytes_on_disk: model_bytes_on_disk(&state.models_path, id),
    }
}

fn set_download_status(state: &AppState, model_id: &str, status: ModelInstallStatus) {
    if let Ok(mut downloads) = state.downloads.lock() {
        downloads.insert(model_id.to_string(), status);
    }
}

fn mark_progress(state: &AppState, model_id: &str, progress: u8, message: impl Into<String>) {
    set_download_status(
        state,
        model_id,
        ModelInstallStatus {
            installed: false,
            downloadable: true,
            downloading: true,
            progress,
            message: Some(message.into()),
            bytes_on_disk: 0,
        },
    );
}

fn url_host(url: &str) -> &str {
    url.split_once("://")
        .map(|(_, rest)| rest)
        .unwrap_or(url)
        .split('/')
        .next()
        .filter(|host| !host.is_empty())
        .unwrap_or(url)
}

/// Sibling staging file. `Path::with_extension("partial")` is a no-op when
/// the destination already ends in `.partial`, which made TarGz downloads
/// delete the archive they had just written and then fail rename with
/// os error 2.
fn download_staging_path(destination: &Path) -> PathBuf {
    let name = destination
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("download");
    destination.with_file_name(format!("{name}.part"))
}

async fn download_url_to_file(
    app: &AppHandle,
    state: &AppState,
    model_id: &str,
    url: &str,
    destination: &Path,
    progress_start: u8,
    progress_end: u8,
) -> Result<(), String> {
    use futures_util::StreamExt;
    use tokio::io::AsyncWriteExt;

    logbuf::debug_and_emit(app, format!("Download {model_id} from {}", url_host(url)));
    if let Some(parent) = destination.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .map_err(|error| format!("Could not create download directory: {error}"))?;
    }
    let temporary = download_staging_path(destination);
    let result = async {
        let response = reqwest::get(url)
            .await
            .map_err(|error| format!("Could not start download: {error}"))?;
        logbuf::debug_and_emit(
            app,
            format!("Download {model_id} HTTP {}", response.status()),
        );
        let response = response
            .error_for_status()
            .map_err(|error| format!("Model download failed: {error}"))?;
        let total = response.content_length();
        let mut stream = response.bytes_stream();
        let mut file = tokio::fs::File::create(&temporary)
            .await
            .map_err(|error| format!("Could not create model file: {error}"))?;
        let mut downloaded = 0_u64;
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|error| format!("Model download interrupted: {error}"))?;
            file.write_all(&chunk)
                .await
                .map_err(|error| format!("Could not write model file: {error}"))?;
            downloaded += chunk.len() as u64;
            let fraction = total
                .map(|length| downloaded as f64 / length.max(1) as f64)
                .unwrap_or(0.0)
                .clamp(0.0, 1.0);
            let progress = progress_start
                + ((progress_end.saturating_sub(progress_start) as f64) * fraction).round() as u8;
            mark_progress(
                state,
                model_id,
                progress.min(99),
                format!("Downloading… {}%", progress.min(99)),
            );
        }
        file.flush()
            .await
            .map_err(|error| format!("Could not finalize model file: {error}"))?;
        drop(file);
        if destination.exists() {
            tokio::fs::remove_file(destination).await.ok();
        }
        tokio::fs::rename(&temporary, destination)
            .await
            .map_err(|error| format!("Could not install model: {error}"))?;
        Ok::<(), String>(())
    }
    .await;
    if result.is_err() {
        let _ = tokio::fs::remove_file(&temporary).await;
    }
    result
}

fn unpack_tar_gz_into(archive_path: &Path, destination: &Path) -> Result<(), String> {
    use flate2::read::GzDecoder;

    if destination.exists() {
        if destination.is_dir() {
            fs::remove_dir_all(destination)
                .map_err(|error| format!("Could not clear model directory: {error}"))?;
        } else {
            fs::remove_file(destination)
                .map_err(|error| format!("Could not clear model path: {error}"))?;
        }
    }
    fs::create_dir_all(destination)
        .map_err(|error| format!("Could not create model directory: {error}"))?;

    let staging = destination.with_extension("extracting");
    if staging.exists() {
        fs::remove_dir_all(&staging).ok();
    }
    fs::create_dir_all(&staging)
        .map_err(|error| format!("Could not create extract directory: {error}"))?;

    let file = fs::File::open(archive_path)
        .map_err(|error| format!("Could not open model archive: {error}"))?;
    let mut archive = tar::Archive::new(GzDecoder::new(file));
    archive
        .unpack(&staging)
        .map_err(|error| format!("Could not unpack model archive: {error}"))?;

    let children = walkdir_shallow(&staging)?;
    let payload = if children.len() == 1 && children[0].is_dir() {
        children[0].clone()
    } else {
        staging.clone()
    };

    // Ignore macOS AppleDouble junk that some CDN archives still ship.
    for entry in walkdir_shallow(&payload)? {
        let Some(name) = entry.file_name() else {
            continue;
        };
        let name = name.to_string_lossy();
        if name.starts_with("._") || name == ".DS_Store" {
            let _ = if entry.is_dir() {
                fs::remove_dir_all(&entry)
            } else {
                fs::remove_file(&entry)
            };
        }
    }

    for entry in walkdir_shallow(&payload)? {
        let Some(name) = entry.file_name() else {
            continue;
        };
        let name_text = name.to_string_lossy();
        if name_text.starts_with("._") || name_text == ".DS_Store" {
            continue;
        }
        let target = destination.join(name);
        fs::rename(&entry, &target).map_err(|error| {
            format!(
                "Could not place model file {}: {error}",
                name.to_string_lossy()
            )
        })?;
    }
    let _ = fs::remove_dir_all(&staging);
    Ok(())
}

fn walkdir_shallow(path: &Path) -> Result<Vec<PathBuf>, String> {
    let mut children = Vec::new();
    for entry in
        fs::read_dir(path).map_err(|error| format!("Could not read extract directory: {error}"))?
    {
        let entry = entry.map_err(|error| format!("Could not read extract entry: {error}"))?;
        children.push(entry.path());
    }
    children.sort();
    Ok(children)
}

#[tauri::command]
fn get_model_statuses(state: State<'_, AppState>) -> HashMap<String, ModelInstallStatus> {
    model_catalog()
        .into_iter()
        .map(|model| (model.id.to_string(), installation_status(&state, model.id)))
        .collect()
}

#[tauri::command]
async fn download_model(
    model_id: String,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let package = model_package(&model_id)
        .ok_or_else(|| "In-app download is not available for this model.".to_string())?;
    if model_is_installed(&state.models_path, &model_id) {
        return Ok(());
    }
    {
        let mut downloads = state
            .downloads
            .lock()
            .map_err(|_| "Download lock was poisoned")?;
        if downloads
            .get(&model_id)
            .is_some_and(|status| status.downloading)
        {
            return Err("This model is already downloading".into());
        }
        downloads.insert(
            model_id.clone(),
            ModelInstallStatus {
                installed: false,
                downloadable: true,
                downloading: true,
                progress: 0,
                message: Some("Connecting…".into()),
                bytes_on_disk: 0,
            },
        );
    }

    let result = async {
        match package {
            ModelPackage::GgmlBin { url } => {
                let destination = model_path(&state.models_path, &model_id);
                download_url_to_file(&app, &state, &model_id, url, &destination, 0, 99).await?;
            }
            ModelPackage::TarGz { url } => {
                let archive = state.models_path.join(format!("{model_id}.tar.gz"));
                download_url_to_file(&app, &state, &model_id, url, &archive, 0, 85).await?;
                mark_progress(&state, &model_id, 90, "Unpacking…");
                let models_path = state.models_path.clone();
                let model_id_for_blocking = model_id.clone();
                tokio::task::spawn_blocking(move || {
                    unpack_tar_gz_into(&archive, &models_path.join(&model_id_for_blocking))?;
                    let _ = fs::remove_file(&archive);
                    Ok::<(), String>(())
                })
                .await
                .map_err(|error| format!("Unpack task failed: {error}"))??;
            }
            ModelPackage::Files { files } => {
                let destination = model_path(&state.models_path, &model_id);
                tokio::fs::create_dir_all(&destination)
                    .await
                    .map_err(|error| format!("Could not create model directory: {error}"))?;
                let count = files.len().max(1) as u8;
                for (index, (filename, url)) in files.iter().enumerate() {
                    let start = (index as u8).saturating_mul(99 / count);
                    let end = ((index as u8 + 1).saturating_mul(99 / count)).min(99);
                    let file_path = destination.join(filename);
                    download_url_to_file(&app, &state, &model_id, url, &file_path, start, end)
                        .await?;
                }
            }
        }
        if !model_is_installed(&state.models_path, &model_id) {
            return Err(
                "Download finished but the expected model files are missing. Try Remove, then Download again."
                    .into(),
            );
        }
        // Skip Silence's voice detector comes with the first model, as on
        // VocaMac; without it trimming falls back to loudness.
        if let Err(error) = vad::download(&state.models_path).await {
            logbuf::warn(error);
        }
        Ok::<(), String>(())
    }
    .await;

    let status = match &result {
        Ok(()) => {
            logbuf::info_and_emit(&app, format!("Downloaded {model_id}"));
            ModelInstallStatus {
                installed: true,
                downloadable: true,
                downloading: false,
                progress: 100,
                message: Some("Installed".into()),
                bytes_on_disk: model_bytes_on_disk(&state.models_path, &model_id),
            }
        }
        Err(error) => {
            logbuf::error_and_emit(&app, format!("Download {model_id} failed: {error}"));
            ModelInstallStatus {
                installed: model_is_installed(&state.models_path, &model_id),
                downloadable: true,
                downloading: false,
                progress: 0,
                message: Some(error.clone()),
                bytes_on_disk: model_bytes_on_disk(&state.models_path, &model_id),
            }
        }
    };
    set_download_status(&state, &model_id, status);
    result
}

#[tauri::command]
fn delete_model(model_id: String, state: State<'_, AppState>) -> Result<(), String> {
    // A kept model holds its files open, and Windows will not delete those.
    ONNX_MODELS.unload();
    let path = model_path(&state.models_path, &model_id);
    if path.is_dir() {
        fs::remove_dir_all(&path)
    } else if path.exists() {
        fs::remove_file(&path)
    } else {
        return Ok(());
    }
    .map_err(|error| format!("Could not remove model: {error}"))?;
    // Clean leftover archive and staging files for this model.
    for leftover in [
        format!("{model_id}.tar.gz"),
        format!("{model_id}.tar.gz.part"),
        format!("{model_id}.tar.gz.partial"),
    ] {
        let _ = fs::remove_file(state.models_path.join(leftover));
    }
    state
        .downloads
        .lock()
        .map_err(|_| "Download lock was poisoned")?
        .remove(&model_id);
    Ok(())
}

/// Converts the take to the 16 kHz the models expect with a band-limited FFT
/// resampler (rubato, as Handy uses). Plain interpolation let everything
/// between 8 kHz and the mic's Nyquist fold back into the speech band.
fn resample_to_16khz(samples: &[f32], source_rate: u32) -> Vec<f32> {
    if source_rate == 16_000 || samples.is_empty() {
        return samples.to_vec();
    }
    let expected = (samples.len() as u64 * 16_000 / source_rate as u64) as usize;
    match fft_resample(samples, source_rate as usize, 16_000, expected) {
        Ok(output) => output,
        Err(error) => {
            logbuf::warn(format!("Resampler failed ({error}); using interpolation."));
            interpolate_to_16khz(samples, source_rate)
        }
    }
}

/// Resamples a whole recording: every chunk, then the filter's tail, with
/// its start-up delay dropped so the output lines up with the input.
fn fft_resample(
    samples: &[f32],
    from: usize,
    to: usize,
    expected: usize,
) -> Result<Vec<f32>, String> {
    use rubato::{FftFixedIn, Resampler};
    const CHUNK: usize = 1024;
    let mut resampler =
        FftFixedIn::<f32>::new(from, to, CHUNK, 2, 1).map_err(|error| error.to_string())?;
    let delay = resampler.output_delay();
    let mut output = Vec::with_capacity(expected + delay + CHUNK);
    let mut chunks = samples.chunks_exact(CHUNK);
    for chunk in &mut chunks {
        let resampled = resampler
            .process(&[chunk], None)
            .map_err(|error| error.to_string())?;
        output.extend_from_slice(&resampled[0]);
    }
    let rest = chunks.remainder();
    if !rest.is_empty() {
        let resampled = resampler
            .process_partial(Some(&[rest]), None)
            .map_err(|error| error.to_string())?;
        output.extend_from_slice(&resampled[0]);
    }
    while output.len() < expected + delay {
        let resampled = resampler
            .process_partial::<&[f32]>(None, None)
            .map_err(|error| error.to_string())?;
        if resampled[0].is_empty() {
            break;
        }
        output.extend_from_slice(&resampled[0]);
    }
    output.drain(..delay.min(output.len()));
    output.resize(expected, 0.0);
    Ok(output)
}

/// The old linear interpolation, kept only as a fallback.
fn interpolate_to_16khz(samples: &[f32], source_rate: u32) -> Vec<f32> {
    let output_length = (samples.len() as u64 * 16_000 / source_rate as u64) as usize;
    (0..output_length)
        .map(|index| {
            let position = index as f64 * source_rate as f64 / 16_000.0;
            let left = position.floor() as usize;
            let right = (left + 1).min(samples.len() - 1);
            let fraction = (position - left as f64) as f32;
            samples[left] * (1.0 - fraction) + samples[right] * fraction
        })
        .collect()
}

/// Held while an ONNX model loads. transcribe-rs reads its process-wide
/// accelerator when a session is built, so setting it and loading happen
/// together; decoding runs outside the lock, so a history retry and a live
/// take only wait for each other's load, never a whole decode.
static ONNX_LOAD: Mutex<()> = Mutex::new(());

fn load_onnx<M>(
    accelerator: transcribe_rs::accel::OrtAccelerator,
    load: impl FnOnce() -> Result<M, transcribe_rs::TranscribeError>,
) -> Result<M, transcribe_rs::TranscribeError> {
    let _load = ONNX_LOAD
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    transcribe_rs::accel::set_ort_accelerator(accelerator);
    load()
}

/// Languages Canary 180M Flash transcribes.
const CANARY_FLASH_LANGUAGES: &[&str] = &["en", "de", "es", "fr"];
/// Languages Canary 1B v2 transcribes.
const CANARY_V2_LANGUAGES: &[&str] = &[
    "bg", "hr", "cs", "da", "nl", "en", "et", "fi", "fr", "de", "el", "hu", "it", "lv", "lt", "mt",
    "pl", "pt", "ro", "sk", "sl", "es", "sv", "ru", "uk",
];
/// Languages Cohere Transcribe transcribes.
const COHERE_LANGUAGES: &[&str] = &[
    "en", "de", "fr", "it", "es", "pt", "el", "nl", "pl", "ar", "vi", "zh", "ja", "ko",
];

/// The language an encoder-decoder model is told. Canary and Cohere cannot
/// detect one, and without one they assume English: German speech came out
/// translated, badly. Languages the model does not know (and Auto-detect)
/// stay English.
fn prompt_language(language: Option<&str>, supported: &[&str]) -> Option<String> {
    language
        .filter(|code| supported.contains(code))
        .map(str::to_owned)
}

/// A loaded ONNX model, kept between takes in `ONNX_MODELS`.
enum OnnxModel {
    Parakeet(transcribe_rs::onnx::parakeet::ParakeetModel),
    Moonshine(transcribe_rs::onnx::moonshine::MoonshineModel),
    SenseVoice(transcribe_rs::onnx::sense_voice::SenseVoiceModel),
    GigaAM(transcribe_rs::onnx::gigaam::GigaAMModel),
    Canary(transcribe_rs::onnx::canary::CanaryModel),
    Cohere(transcribe_rs::onnx::cohere::CohereModel),
}

/// The ONNX model kept between takes. Loading one took seconds on every
/// take, after the user stopped speaking; now it happens once, and at the
/// hotkey press (`preload_selected_model`) rather than after the take.
static ONNX_MODELS: model_slot::ModelSlot<OnnxModel> = model_slot::ModelSlot::new();

fn onnx_key(model_id: &str, accelerator: transcribe_rs::accel::OrtAccelerator) -> String {
    format!("{model_id}|{accelerator:?}")
}

fn load_onnx_model(
    model_id: &str,
    models_path: &Path,
    accelerator: transcribe_rs::accel::OrtAccelerator,
) -> Result<OnnxModel, String> {
    use transcribe_rs::onnx::{
        canary::CanaryModel,
        cohere::CohereModel,
        gigaam::GigaAMModel,
        moonshine::{MoonshineModel, MoonshineVariant},
        parakeet::ParakeetModel,
        sense_voice::SenseVoiceModel,
        Quantization,
    };

    let model_path = models_path.join(model_id);
    if !model_path.is_dir() {
        return Err(format!(
            "Model is not installed: {}. Expected its ONNX model directory at {}.",
            model_id,
            model_path.display()
        ));
    }
    let started = std::time::Instant::now();
    let model = match model_id {
        "parakeet-tdt-0.6b-v3" => load_onnx(accelerator, || {
            ParakeetModel::load(&model_path, &Quantization::Int8)
        })
        .map(OnnxModel::Parakeet)
        .map_err(|error| format!("Could not load Parakeet: {error}"))?,
        "moonshine-tiny" | "moonshine-base" => {
            let variant = if model_id == "moonshine-tiny" {
                MoonshineVariant::Tiny
            } else {
                MoonshineVariant::Base
            };
            load_onnx(accelerator, || {
                MoonshineModel::load(&model_path, variant, &Quantization::default())
            })
            .map(OnnxModel::Moonshine)
            .map_err(|error| format!("Could not load Moonshine: {error}"))?
        }
        "sensevoice-small" => load_onnx(accelerator, || {
            SenseVoiceModel::load(&model_path, &Quantization::Int8)
        })
        .map(OnnxModel::SenseVoice)
        .map_err(|error| format!("Could not load SenseVoice: {error}"))?,
        "gigaam-v3" => load_onnx(accelerator, || {
            GigaAMModel::load(&model_path, &Quantization::Int8)
        })
        .map(OnnxModel::GigaAM)
        .map_err(|error| format!("Could not load GigaAM: {error}"))?,
        "canary-180m" | "canary-1b-v2" => load_onnx(accelerator, || {
            CanaryModel::load(&model_path, &Quantization::Int8)
        })
        .map(OnnxModel::Canary)
        .map_err(|error| format!("Could not load Canary: {error}"))?,
        "cohere-transcribe" => load_onnx(accelerator, || {
            CohereModel::load(&model_path, &Quantization::Int4)
        })
        .map(OnnxModel::Cohere)
        .map_err(|error| format!("Could not load Cohere Transcribe: {error}"))?,
        _ => return Err(format!("The {} adapter is not available yet.", model_id)),
    };
    logbuf::debug(format!(
        "Loaded {model_id} ({accelerator:?}) in {} ms.",
        started.elapsed().as_millis()
    ));
    Ok(model)
}

fn decode_onnx(
    model: &mut OnnxModel,
    model_id: &str,
    pcm: &[f32],
    language: Option<&str>,
) -> Result<String, String> {
    use transcribe_rs::onnx::{
        canary::CanaryParams, cohere::CohereParams, parakeet::ParakeetParams,
        sense_voice::SenseVoiceParams,
    };

    let text = match model {
        OnnxModel::Parakeet(model) => decode_parakeet(|lead_in| {
            let result = if lead_in {
                model.transcribe_with(pcm, &ParakeetParams::default())
            } else {
                model.transcribe_raw(pcm, &transcribe_rs::TranscribeOptions::default())
            };
            result
                .map(|result| result.text)
                .map_err(|error| format!("Parakeet transcription failed: {error}"))
        })?,
        OnnxModel::Moonshine(model) => decode_in_windows(pcm, |window| {
            model
                .transcribe(window, &transcribe_rs::TranscribeOptions::default())
                .map(|result| result.text)
                .map_err(|error| format!("Moonshine transcription failed: {error}"))
        })?,
        OnnxModel::SenseVoice(model) => {
            model
                .transcribe_with(
                    pcm,
                    &SenseVoiceParams {
                        language: language.map(str::to_owned),
                        ..Default::default()
                    },
                )
                .map_err(|error| format!("SenseVoice transcription failed: {error}"))?
                .text
        }
        OnnxModel::GigaAM(model) => decode_in_windows(pcm, |window| {
            model
                .transcribe(window, &transcribe_rs::TranscribeOptions::default())
                .map(|result| result.text)
                .map_err(|error| format!("GigaAM transcription failed: {error}"))
        })?,
        OnnxModel::Canary(model) => {
            let supported = if model_id == "canary-1b-v2" {
                CANARY_V2_LANGUAGES
            } else {
                CANARY_FLASH_LANGUAGES
            };
            let params = CanaryParams {
                language: prompt_language(language, supported),
                ..CanaryParams::default()
            };
            decode_in_windows(pcm, |window| {
                model
                    .transcribe_with(window, &params)
                    .map(|result| result.text)
                    .map_err(|error| format!("Canary transcription failed: {error}"))
            })?
        }
        OnnxModel::Cohere(model) => {
            let params = CohereParams {
                language: prompt_language(language, COHERE_LANGUAGES),
                ..CohereParams::default()
            };
            // It decodes a take in one pass with a 512-token budget, so long
            // takes go in windows like Canary's.
            decode_in_windows(pcm, |window| {
                model
                    .transcribe_with(window, &params)
                    .map(|result| result.text)
                    .map_err(|error| format!("Cohere transcription failed: {error}"))
            })?
        }
    };
    Ok(text.trim().to_string())
}

/// Decodes with the kept model, loading it first when it is not kept. A
/// model goes back only after a good decode, so a failure never sticks.
fn transcribe_onnx(
    model_id: &str,
    models_path: &Path,
    pcm: &[f32],
    language: Option<&str>,
    accelerator: transcribe_rs::accel::OrtAccelerator,
) -> Result<String, String> {
    let key = onnx_key(model_id, accelerator);
    let mut lease = ONNX_MODELS.take(&key, || load_onnx_model(model_id, models_path, accelerator))?;
    let text = decode_onnx(&mut lease.model, model_id, pcm, language);
    if text.is_ok() {
        ONNX_MODELS.put(lease);
    } else {
        ONNX_MODELS.release(lease);
    }
    text
}

/// Parakeet's decode rule. `decode(true)` is transcribe-rs's usual path,
/// which puts 250 ms of digital zeros before the take; `decode(false)`
/// leaves them out. When speech starts right away, the zeros can tip the
/// int8 model into returning nothing for the whole take, while the same
/// audio without them often decodes. Leaving them out is not better
/// everywhere, so the usual path runs first and only an empty result gets
/// a second pass.
fn decode_parakeet(
    mut decode: impl FnMut(bool) -> Result<String, String>,
) -> Result<String, String> {
    let text = decode(true)?;
    if !text.trim().is_empty() {
        return Ok(text);
    }
    logbuf::debug("Parakeet returned nothing; decoding again without its lead-in.");
    decode(false)
}

/// Decodes a take in `chunking` windows and joins the text. Canary and
/// Moonshine drop or repeat text on long takes, Moonshine rejects takes over
/// 64 s, and GigaAM's encoder rejects takes over 200 s.
///
/// Moonshine also returns nothing for a window that opens on a second or
/// more of pause (a pause too long for a cut to reach past, or any take with
/// Skip Silence off). A window that decodes to nothing is decoded once more
/// with its long pauses shortened and every sound kept; it had no text to
/// lose.
fn decode_in_windows(
    pcm: &[f32],
    mut decode: impl FnMut(&[f32]) -> Result<String, String>,
) -> Result<String, String> {
    let ranges = chunking::windows(pcm);
    if ranges.len() > 1 {
        logbuf::debug(format!("Decoding the take in {} windows.", ranges.len()));
    }
    let mut parts = Vec::with_capacity(ranges.len());
    for range in ranges {
        let window = &pcm[range];
        let mut text = decode(window)?;
        if text.trim().is_empty() {
            if let Some(shortened) = chunking::without_long_pauses(window) {
                logbuf::debug(
                    "A window decoded to nothing; decoding it again with its pauses shortened.",
                );
                text = decode(&shortened)?;
            }
        }
        let text = text.trim();
        if !text.is_empty() {
            parts.push(text.to_owned());
        }
    }
    Ok(parts.join(" "))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PttPressedAction {
    Start,
    Ignore,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MicTestGate {
    Allow,
    ClearStaleThenAllow,
    RefuseLiveSession,
}

fn ptt_pressed_action(recording_flag: bool, capture_live: bool) -> PttPressedAction {
    if recording_flag || capture_live {
        PttPressedAction::Ignore
    } else {
        PttPressedAction::Start
    }
}

fn mic_test_gate(recording_flag: bool, capture_live: bool) -> MicTestGate {
    if capture_live {
        MicTestGate::RefuseLiveSession
    } else if recording_flag {
        MicTestGate::ClearStaleThenAllow
    } else {
        MicTestGate::Allow
    }
}

fn is_stale_stop_error(error: &str) -> bool {
    error.contains("No recording is in progress")
        || error.contains("No microphone audio was captured")
}

/// How long the microphone stays open after a take (Handy uses 30 s). The
/// next take in that time starts recording at once.
const WARM_MICROPHONE_MS: u128 = 30_000;

/// Whether a microphone kept open after a take can record the next one: the
/// same device by name (the Windows default resolved to its current
/// device), and still delivering audio (a stream that went quiet across
/// sleep or an unplugged mic is replaced).
fn reuse_warm_stream(kept_device: &str, requested_device: &str, last_audio_ms: u128, now_ms: u128) -> bool {
    kept_device == requested_device && now_ms.saturating_sub(last_audio_ms) <= 500
}

/// A failed Start must not flip an already-open stream from meter to dictation
/// (or the other way). Only a successful open owns `meter_only`.
fn meter_only_after_start(stream_open: bool, requested_meter: bool, current_meter: bool) -> bool {
    if stream_open {
        current_meter
    } else {
        requested_meter
    }
}

fn recording_after_start_attempt(start_ok: bool) -> bool {
    start_ok
}

fn recording_after_stop_attempt() -> bool {
    false
}

/// take_recording drops the stream before it can Err. Auto-stop must
/// still clear the session flag so Mic Test and Ready stay honest.
#[cfg_attr(not(windows), allow(dead_code))]
fn clear_recording_after_capture_drop(app: &AppHandle) {
    if let Some(state) = app.try_state::<AppState>() {
        set_recording_flag(&state, recording_after_stop_attempt());
        state.session_opening.store(false, Ordering::SeqCst);
        state.release_during_open.store(false, Ordering::SeqCst);
    }
    let _ = app.emit("recording-changed", false);
    release_session_chrome(app);
}

fn session_is_live(_recording_flag: bool, capture_live: bool) -> bool {
    capture_live
}

fn safety_timeout_for(max_recording_seconds: f32) -> std::time::Duration {
    std::time::Duration::from_secs_f32((max_recording_seconds + 5.0).clamp(8.0, 360.0))
}

fn set_recording_flag(state: &AppState, value: bool) {
    *state.recording.lock().unwrap_or_else(|e| e.into_inner()) = value;
}

/// Begins microphone capture. The session flag is true only after WASAPI
/// actually opens. `inject` is false for Test Dictation so silence/max
/// auto-stop will not type into the front app. `silence_auto_stop` is on for
/// toggled takes; a held key or the hands-free shortcut ends the others.
fn begin_voice_session(
    app: &AppHandle,
    inject: bool,
    trigger: Trigger,
    silence_auto_stop: bool,
) -> Result<(), String> {
    let state = app.state::<AppState>();
    if *state
        .dictation_paused
        .lock()
        .map_err(|_| "Pause lock was poisoned")?
    {
        return Err("Dictation is paused while a watched app is running.".into());
    }
    let already = *state
        .recording
        .lock()
        .map_err(|_| "Recording lock was poisoned")?;
    if already && state.recorder.capture_live() {
        return Ok(());
    }
    set_recording_flag(&state, false);
    *state
        .inject_on_auto_stop
        .lock()
        .map_err(|_| "Inject lock was poisoned")? = inject;
    *state
        .session_trigger
        .lock()
        .map_err(|_| "Session lock was poisoned")? = trigger;
    let settings = state
        .settings
        .lock()
        .map_err(|_| "Settings lock was poisoned")?
        .clone();
    logbuf::debug_and_emit(
        app,
        format!(
            "Start {} ({})",
            if inject {
                "dictation"
            } else {
                "test dictation"
            },
            settings.selected_model
        ),
    );
    if !model_is_installed(&state.models_path, &settings.selected_model) {
        set_recording_flag(&state, recording_after_start_attempt(false));
        state.session_opening.store(false, Ordering::SeqCst);
        let error = format!(
            "No speech model is installed. Open Models and download {} first.",
            settings.selected_model
        );
        logbuf::error_and_emit(app, error.clone());
        return Err(error);
    }
    preload_selected_model(&state, &settings);
    state.session_opening.store(true, Ordering::SeqCst);
    state.release_during_open.store(false, Ordering::SeqCst);
    let session = state.session_id.fetch_add(1, Ordering::SeqCst) + 1;
    if let Err(error) = state.recorder.start(
        settings.silence_seconds,
        settings.max_recording_seconds,
        settings.input_device.clone(),
        silence_auto_stop,
        session,
    ) {
        state.session_opening.store(false, Ordering::SeqCst);
        set_recording_flag(&state, recording_after_start_attempt(false));
        let _ = app.emit("recording-changed", false);
        logbuf::error_and_emit(app, format!("Start dictation failed: {error}"));
        if inject {
            overlay::show(app, overlay::Phase::Error(error.clone()), overlay_on(&settings));
        }
        return Err(error);
    }
    set_recording_flag(&state, recording_after_start_attempt(true));
    state.session_opening.store(false, Ordering::SeqCst);
    if settings.escape_cancels {
        hook::set_cancel_armed(Some(session));
    }
    if settings.mute_other_audio {
        state.ducker.mute_others();
    }
    if state.release_during_open.swap(false, Ordering::SeqCst) {
        finish_voice_session(app);
        return Ok(());
    }
    sounds::play_if_enabled(&settings.sound_theme, true);
    let _ = app.emit("recording-changed", true);
    set_tray_phase(app, TrayPhase::Listening);
    if inject {
        let hint = if settings.escape_cancels {
            "Esc to cancel".to_string()
        } else {
            String::new()
        };
        overlay::show(app, overlay::Phase::Listening { hint }, overlay_on(&settings));
        if settings.live_preview && overlay_on(&settings) {
            live_preview::start(app.clone(), session);
        }
    }
    Ok(())
}

/// Begins microphone capture. The UI should call `stop_and_transcribe` after
/// push-to-talk is released (or when toggle mode is stopped). Runs off the
/// WebView IPC thread so WASAPI open cannot freeze the window.
#[tauri::command]
async fn start_recording(app: AppHandle, no_inject: Option<bool>) -> Result<(), String> {
    let inject = !no_inject.unwrap_or(false);
    tauri::async_runtime::spawn_blocking(move || {
        let toggle = manual_silence_auto_stop(&app);
        begin_voice_session(&app, inject, Trigger::Manual, toggle)
    })
    .await
    .map_err(|error| format!("Start dictation was cancelled: {error}"))?
}

/// Takes started from the tray or window follow the activation style.
fn manual_silence_auto_stop(app: &AppHandle) -> bool {
    app.state::<AppState>()
        .settings
        .lock()
        .map(|settings| settings.activation_mode == "toggle")
        .unwrap_or(false)
}

fn language_code(language: &str) -> Option<&'static str> {
    Some(match language {
        "English" => "en",
        "Spanish" => "es",
        "French" => "fr",
        "German" => "de",
        "Italian" => "it",
        "Portuguese" => "pt",
        "Dutch" => "nl",
        "Russian" => "ru",
        "Japanese" => "ja",
        "Chinese" => "zh",
        "Korean" => "ko",
        "Arabic" => "ar",
        "Hindi" => "hi",
        "Turkish" => "tr",
        "Polish" => "pl",
        "Ukrainian" => "uk",
        "Swedish" => "sv",
        "Norwegian" => "no",
        "Danish" => "da",
        "Finnish" => "fi",
        "Czech" => "cs",
        "Greek" => "el",
        "Hebrew" => "he",
        "Indonesian" => "id",
        "Vietnamese" => "vi",
        "Thai" => "th",
        "Romanian" => "ro",
        "Hungarian" => "hu",
        "Bulgarian" => "bg",
        "Croatian" => "hr",
        "Estonian" => "et",
        "Latvian" => "lv",
        "Lithuanian" => "lt",
        "Maltese" => "mt",
        "Slovak" => "sk",
        "Slovenian" => "sl",
        "Catalan" => "ca",
        _ => return None,
    })
}

/// The language the text rules may assume: the chosen one, or English for
/// an English-only model. `None` lets them judge the text. Voca Hinglish
/// decodes as English but writes Hindi in Latin letters; English rules
/// would "correct" those words, so its text counts as Hindi.
fn text_language(settings: &Settings) -> Option<&'static str> {
    if settings.selected_model == VOCA_HINGLISH {
        return Some("hi");
    }
    language_code(&settings.language).or_else(|| {
        model_catalog()
            .iter()
            .find(|model| model.id == settings.selected_model)
            .filter(|model| model.languages == "English")
            .map(|_| "en")
    })
}

/// Shown when a take decodes to no words (silence, or only noise markers).
const NO_SPEECH: &str = "No speech was recognized. Nothing was typed.";

/// A finished take: the text to type, its history entry, and how long the
/// speech was.
struct Take {
    text: String,
    history_id: Option<u128>,
    speech_ms: u64,
}

fn transcribe_samples(state: &AppState, samples: Vec<f32>, sample_rate: u32) -> Result<Take, String> {
    let settings = state
        .settings
        .lock()
        .map_err(|_| "Settings lock was poisoned")?
        .clone();
    let pcm = resample_to_16khz(&samples, sample_rate);
    logbuf::debug(format!(
        "Transcribe {} ({} samples at {} Hz, lang {})",
        settings.selected_model,
        pcm.len(),
        sample_rate,
        settings.language
    ));
    if pcm.len() < 4_000 {
        logbuf::warn("Recording is too short.");
        return Err("Recording is too short. Hold the hotkey and speak for a moment.".into());
    }
    let speech_ms = pcm.len() as u64 * 1000 / 16_000;
    // The audio is on disk before the model runs, so a crash or a failed
    // decode never loses what was said.
    let history_id = if settings.history_enabled {
        match state
            .history
            .begin(&pcm, &settings.selected_model, settings.history_keep_audio)
        {
            Ok(id) => Some(id),
            Err(error) => {
                logbuf::warn(error);
                None
            }
        }
    } else {
        None
    };
    let result = recognize_and_format(state, &settings, &pcm);
    if let Some(id) = history_id {
        let saved = match &result {
            // Failed in History; `complete_take` reports it once it knows
            // the take was not cancelled.
            Ok(text) if text.trim().is_empty() => state.history.finish(
                id,
                "",
                &settings.selected_model,
                history::STATUS_FAILED,
                Some(NO_SPEECH.into()),
            ),
            Ok(text) => state.history.finish(
                id,
                text,
                &settings.selected_model,
                history::STATUS_OK,
                None,
            ),
            Err(error) => state.history.finish(
                id,
                "",
                &settings.selected_model,
                history::STATUS_FAILED,
                Some(error.clone()),
            ),
        };
        if let Err(error) = saved {
            logbuf::warn(error);
        }
    }
    result.map(|text| Take {
        text,
        history_id,
        speech_ms,
    })
}

/// Silence trim, the model, then the text rules.
fn recognize_and_format(state: &AppState, settings: &Settings, pcm: &[f32]) -> Result<String, String> {
    let audio = if settings.skip_silence {
        let decision = vad::decide(&state.models_path, pcm).unwrap_or_else(|error| {
            logbuf::debug(format!("{error}; trimming silence by loudness instead."));
            silence::decide(pcm)
        });
        match decision {
            silence::Decision::NoSpeech => {
                logbuf::debug("No speech in the recording; skipped the model.");
                return Ok(String::new());
            }
            silence::Decision::Trim(ranges) => {
                let trimmed = silence::apply(&ranges, pcm);
                logbuf::debug(format!(
                    "Skipped silence: {} of {} samples go to the model.",
                    trimmed.len(),
                    pcm.len()
                ));
                trimmed
            }
            silence::Decision::Keep => pcm.to_vec(),
        }
    } else {
        pcm.to_vec()
    };
    let raw = recognize(state, settings, audio)?;
    let text = format_transcript(settings, &raw);
    logbuf::info(format!(
        "Transcribed {} chars with {}",
        text.chars().count(),
        settings.selected_model
    ));
    Ok(text)
}

/// ONNX models that run on DirectML when Windows has a hardware GPU. The
/// catalog's "CPU · DirectML" label comes from this list.
///
/// Canary stays on CPU. Its int8 decoder runs once per output token, and on
/// DirectML every step crosses between CPU and GPU, so a 180M model decoded
/// slower than on CPU and sometimes returned nothing. An empty result is not
/// an error, so the CPU fallback never caught it.
fn onnx_uses_directml(model_id: &str) -> bool {
    matches!(model_id, "parakeet-tdt-0.6b-v3" | "sensevoice-small")
}

/// Set once DirectML has failed a take that CPU then decoded, so later takes
/// in this run go straight to CPU instead of failing on the GPU first.
static DIRECTML_FAILED: AtomicBool = AtomicBool::new(false);

/// `transcribe_onnx` on DirectML when the model and the PC support it, and
/// on CPU when they do not or DirectML fails.
fn transcribe_onnx_accelerated(
    model_id: &str,
    models_path: &Path,
    pcm: &[f32],
    language: Option<&str>,
) -> Result<String, String> {
    use transcribe_rs::accel::OrtAccelerator;

    let directml = onnx_tries_directml(model_id, models_path);
    let (result, directml_broken) = decode_with_cpu_fallback(directml, |gpu| {
        let accelerator = if gpu {
            OrtAccelerator::DirectMl
        } else {
            OrtAccelerator::CpuOnly
        };
        let started = std::time::Instant::now();
        let result = transcribe_onnx(model_id, models_path, pcm, language, accelerator);
        logbuf::debug(format!(
            "{model_id} on {}: {} ms for {:.1} s of audio (includes a load if it was not kept).",
            if gpu { "DirectML" } else { "CPU" },
            started.elapsed().as_millis(),
            pcm.len() as f32 / 16_000.0
        ));
        result
    });
    if directml_broken {
        DIRECTML_FAILED.store(true, Ordering::Relaxed);
        logbuf::warn(format!(
            "DirectML could not decode {model_id}; using CPU until VocaWin restarts."
        ));
    }
    result
}

/// Whether a take of `model_id` tries DirectML first.
fn onnx_tries_directml(model_id: &str, models_path: &Path) -> bool {
    cfg!(windows)
        && onnx_uses_directml(model_id)
        && model_is_installed(models_path, model_id)
        && !DIRECTML_FAILED.load(Ordering::Relaxed)
        && gpu::detect_gpu().available
}

/// Starts loading the selected model when the hotkey goes down, so it loads
/// while the user speaks instead of after (Handy does the same). A take
/// that ends first waits for this load rather than starting its own.
fn preload_selected_model(state: &AppState, settings: &Settings) {
    let model_id = settings.selected_model.clone();
    if is_whisper_model(&model_id) {
        let gpu = gpu::detect_gpu();
        state.whisper_cache.preload(
            state.models_path.join(format!("{model_id}.bin")),
            cfg!(vocawin_whisper_vulkan) && gpu.available,
            gpu.name.clone(),
        );
        return;
    }
    let models_path = state.models_path.clone();
    let _ = std::thread::Builder::new()
        .name("vocawin-preload".into())
        .spawn(move || {
            use transcribe_rs::accel::OrtAccelerator;
            let accelerator = if onnx_tries_directml(&model_id, &models_path) {
                OrtAccelerator::DirectMl
            } else {
                OrtAccelerator::CpuOnly
            };
            let key = onnx_key(&model_id, accelerator);
            if let Err(error) =
                ONNX_MODELS.preload(&key, || load_onnx_model(&model_id, &models_path, accelerator))
            {
                logbuf::debug(format!("Preload of {model_id} failed: {error}"));
            }
        });
}

/// Decodes on the GPU first when `gpu` is set, then on CPU if that fails.
/// Also says whether DirectML is to blame: only when CPU decodes the take
/// the GPU could not. When CPU fails too, the model or the audio is the
/// problem, and the CPU error is returned.
fn decode_with_cpu_fallback(
    gpu: bool,
    mut decode: impl FnMut(bool) -> Result<String, String>,
) -> (Result<String, String>, bool) {
    if !gpu {
        return (decode(false), false);
    }
    let gpu_error = match decode(true) {
        Ok(text) => return (Ok(text), false),
        Err(error) => error,
    };
    logbuf::debug(format!("DirectML decode failed; trying CPU: {gpu_error}"));
    let result = decode(false);
    let directml_broken = result.is_ok();
    (result, directml_broken)
}

fn recognize(state: &AppState, settings: &Settings, pcm: Vec<f32>) -> Result<String, String> {
    let language = decoder_language(settings);
    if !is_whisper_model(&settings.selected_model) {
        return transcribe_onnx_accelerated(
            &settings.selected_model,
            &state.models_path,
            &pcm,
            language,
        );
    }
    let model_path = state
        .models_path
        .join(format!("{}.bin", settings.selected_model));
    if !model_path.exists() {
        return Err(format!(
            "Model is not installed: {}. Put its whisper.cpp GGML .bin file at {}.",
            settings.selected_model,
            model_path.display()
        ));
    }
    let gpu = gpu::detect_gpu();
    let use_gpu = cfg!(vocawin_whisper_vulkan) && gpu.available;
    let text = state.whisper_cache.transcribe(
        model_path,
        pcm,
        language.map(str::to_string),
        use_gpu,
        gpu.name.clone(),
        true,
        vocabulary::whisper_prompt(&settings.custom_vocabulary),
    )?;
    Ok(if settings.selected_model == VOCA_HINGLISH {
        hinglish::clean(&text)
    } else {
        text
    })
}

/// The language the decoder is told. Voca Hinglish writes romanized Hindi
/// only when decoded as English; asked for Hindi, or left to detect, it
/// falls back to Devanagari or translates. So it ignores the setting.
fn decoder_language(settings: &Settings) -> Option<&'static str> {
    if settings.selected_model == VOCA_HINGLISH {
        Some("en")
    } else {
        language_code(&settings.language)
    }
}

/// The text rules every take gets (see `pipeline`).
fn format_transcript(settings: &Settings, raw: &str) -> String {
    let vocabulary = vocabulary::terms(&settings.custom_vocabulary);
    pipeline::process(
        raw,
        &pipeline::TextOptions {
            language: text_language(settings),
            cleanup: settings.cleanup_level != "none",
            vocabulary: &vocabulary,
            replacements: &settings.replacements,
            snippets: &settings.snippets,
            numbers: settings.numbers_as_digits,
            symbols: settings.numbers_as_digits && settings.number_symbols,
            emoji: settings.spoken_emoji,
            auto_capitalize: settings.auto_capitalize,
            trailing_space: settings.append_trailing_space,
        },
    )
}

/// Stop capture and clear the session flag even when stop() fails. Returns
/// the audio when there is any.
fn stop_capture(state: &AppState) -> Result<Option<(Vec<f32>, u32)>, String> {
    let stopped = state.recorder.stop();
    set_recording_flag(state, recording_after_stop_attempt());
    state.session_opening.store(false, Ordering::SeqCst);
    state.release_during_open.store(false, Ordering::SeqCst);
    match stopped {
        Ok((samples, sample_rate)) if !samples.is_empty() => {
            // From here Escape cancels the transcription, not the recording.
            state.processing.store(true, Ordering::SeqCst);
            Ok(Some((samples, sample_rate)))
        }
        Ok(_) => Ok(None),
        Err(error) if is_stale_stop_error(&error) => Ok(None),
        Err(error) => Err(error),
    }
}

/// Stop and discard: auto-pause and Escape.
fn abandon_voice_session(state: &AppState) {
    // Stop the microphone first: waiting for a preview pass must not keep
    // it recording.
    let _ = state.recorder.stop();
    live_preview::stop();
    set_recording_flag(state, recording_after_stop_attempt());
    state.session_opening.store(false, Ordering::SeqCst);
    state.release_during_open.store(false, Ordering::SeqCst);
    state.ducker.restore();
}

fn finish_voice_session(handle: &AppHandle) {
    let state = handle.state::<AppState>();
    let settings = state.settings.lock().map(|s| s.clone()).unwrap_or_default();
    let inject = session_injects(&state);
    let session = state.session_id.load(Ordering::SeqCst);
    let captured = stop_capture(&state);
    let _ = handle.emit("recording-changed", false);
    let result = match captured {
        Ok(Some((samples, sample_rate))) => {
            sounds::play_if_enabled(&settings.sound_theme, false);
            complete_take(handle, samples, sample_rate, true, session)
        }
        Ok(None) => {
            release_session_chrome(handle);
            return;
        }
        Err(error) => {
            release_session_chrome(handle);
            Err(error)
        }
    };
    match result {
        Ok(text) if !text.is_empty() => {
            let event = if inject {
                "dictation-finished"
            } else {
                "test-dictation-finished"
            };
            let _ = handle.emit(event, text);
        }
        Ok(_) => {}
        Err(error) => {
            sounds::play_error_if_enabled(&settings.sound_theme);
            logbuf::error_and_emit(handle, format!("Dictation error: {error}"));
            let _ = handle.emit("dictation-error", error);
        }
    }
}

#[tauri::command]
async fn stop_and_transcribe(app: AppHandle) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<AppState>();
        let sound = state
            .settings
            .lock()
            .map(|settings| settings.sound_theme.clone())
            .unwrap_or_else(|_| "voca".into());
        let session = state.session_id.load(Ordering::SeqCst);
        let captured = stop_capture(&state);
        let _ = app.emit("recording-changed", false);
        let result = match captured {
            Ok(Some((samples, sample_rate))) => {
                sounds::play_if_enabled(&sound, false);
                complete_take(&app, samples, sample_rate, false, session)
            }
            Ok(None) => {
                release_session_chrome(&app);
                Ok(String::new())
            }
            Err(error) => {
                release_session_chrome(&app);
                Err(error)
            }
        };
        if result.is_err() {
            sounds::play_error_if_enabled(&sound);
        }
        result
    })
    .await
    .map_err(|error| format!("Stop dictation was cancelled: {error}"))?
}

/// Escape: throw take `session` away. While recording the audio is
/// discarded; while transcribing, the result is kept in History but not
/// typed. An Escape for an older take (handled after a new one started) is
/// ignored, so it can never discard the take after it.
fn cancel_voice_session(app: &AppHandle, session: u64) {
    let state = app.state::<AppState>();
    if !is_current_session(state.session_id.load(Ordering::SeqCst), session) {
        logbuf::debug("Escape for an earlier take ignored.");
        return;
    }
    if !cancellations(&state).cancel(session) {
        logbuf::debug("Escape came after the take was typed.");
        return;
    }
    hook::disarm_cancel_for(session);
    let settings = state.settings.lock().map(|s| s.clone()).unwrap_or_default();
    let recording = *state.recording.lock().unwrap_or_else(|e| e.into_inner())
        || state.session_opening.load(Ordering::SeqCst)
        || state.recorder.capture_live();
    if recording {
        abandon_voice_session(&state);
        let _ = app.emit("recording-changed", false);
        logbuf::info_and_emit(app, "Dictation cancelled.");
        apply_ready_or_parked_tray(app);
    } else {
        // Not settled yet (checked above), so completion will see the mark.
        logbuf::info_and_emit(app, "Transcription cancelled; nothing will be typed.");
    }
    overlay::show(app, overlay::Phase::Cancelled, overlay_on(&settings));
    let _ = app.emit("dictation-cancelled", ());
}

/// Cancelled takes and takes that are past the point of cancelling. Both
/// lists are bounded; ids are never reused.
#[derive(Default)]
struct Cancellations {
    cancelled: Vec<u64>,
    settled: Vec<u64>,
}

impl Cancellations {
    /// Escape for `session`. False when the take already committed to typing.
    fn cancel(&mut self, session: u64) -> bool {
        if self.settled.contains(&session) {
            return false;
        }
        remember(&mut self.cancelled, session);
        true
    }

    /// The take is about to type (or not): was it cancelled? From here on
    /// a later Escape is refused.
    fn settle(&mut self, session: u64) -> bool {
        remember(&mut self.settled, session);
        match self.cancelled.iter().position(|id| *id == session) {
            Some(index) => {
                self.cancelled.remove(index);
                true
            }
            None => false,
        }
    }
}

fn remember(ids: &mut Vec<u64>, session: u64) {
    if !ids.contains(&session) {
        ids.push(session);
    }
    let excess = ids.len().saturating_sub(16);
    ids.drain(..excess);
}

fn cancellations(state: &AppState) -> std::sync::MutexGuard<'_, Cancellations> {
    state
        .cancellations
        .lock()
        .unwrap_or_else(|e| e.into_inner())
}

fn is_current_session(current: u64, cancelled: u64) -> bool {
    cancelled != 0 && cancelled == current
}

/// Type the last dictation again. Waits for the shortcut's modifiers to come
/// up first, or Ctrl+Alt would turn the text into shortcuts.
fn paste_last(app: &AppHandle) {
    let state = app.state::<AppState>();
    let settings = state.settings.lock().map(|s| s.clone()).unwrap_or_default();
    let remembered = state
        .last_dictation
        .lock()
        .map(|last| last.clone())
        .unwrap_or_default();
    let text = if remembered.trim().is_empty() {
        state
            .history
            .latest_text()
            .map(|text| {
                if settings.append_trailing_space {
                    output::append_trailing_space(&text)
                } else {
                    text
                }
            })
            .unwrap_or_default()
    } else {
        remembered
    };
    if text.trim().is_empty() {
        overlay::show(
            app,
            overlay::Phase::Notice("Nothing to paste yet".into()),
            overlay_on(&settings),
        );
        return;
    }
    let app = app.clone();
    std::thread::spawn(move || {
        output::wait_for_modifiers_released(std::time::Duration::from_millis(1_500));
        let state = app.state::<AppState>();
        if let Err(error) = inject_transcript(&state, &text) {
            overlay::show(&app, overlay::Phase::Error(error), overlay_on(&settings));
        }
    });
}

/// The pill shown for a few seconds after launch, so people can tell VocaWin
/// started even when its tray icon is tucked into the overflow.
fn show_ready_pill(app: &AppHandle) {
    let state = app.state::<AppState>();
    let settings = state.settings.lock().map(|s| s.clone()).unwrap_or_default();
    if !settings.ready_pill {
        return;
    }
    let paused = *state
        .dictation_paused
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    let (title, detail) = if !model_is_installed(&state.models_path, &settings.selected_model) {
        (
            "VocaWin is running".to_string(),
            "Download a speech model to start dictating".to_string(),
        )
    } else if paused {
        (
            "VocaWin is paused".to_string(),
            "A watched app is running".to_string(),
        )
    } else {
        let key = hotkey_display(&settings.hotkey);
        let action = if settings.activation_mode == "toggle" {
            format!("Tap {key} to dictate")
        } else {
            format!("Hold {key} to dictate")
        };
        ("VocaWin is ready".to_string(), action)
    };
    overlay::show(app, overlay::Phase::Ready { title, detail }, true);
}

/// "Right Alt" rather than the preset label's "(Option)" or "Custom:".
fn hotkey_display(spec: &str) -> String {
    let name = hotkey::display_name(spec);
    let name = name.strip_prefix("Custom: ").unwrap_or(&name);
    name.replace(" (Option)", "")
}

#[tauri::command]
fn system_summary() -> serde_json::Value {
    let gpu = gpu::detect_gpu();
    serde_json::json!({
        "platform": "Windows 10/11",
        "runtime": "Rust + Tauri",
        "privacy": "Audio and transcription remain on this device",
        "gpuBackends": gpu_backends_summary(),
        "whisperAcceleration": whisper_acceleration(),
        "gpu": gpu
    })
}

#[tauri::command]
fn get_gpu_status() -> gpu::GpuStatus {
    gpu::detect_gpu()
}

#[tauri::command]
fn get_log_lines() -> Vec<logbuf::LogLine> {
    logbuf::snapshot()
}

#[tauri::command]
fn get_debug_report(state: State<'_, AppState>) -> machine::DebugReport {
    let debug_logging = state
        .settings
        .lock()
        .map(|settings| settings.debug_logging)
        .unwrap_or(false);
    machine::debug_report(
        gpu::detect_gpu(),
        debug_logging,
        &logbuf::snapshot_text(debug_logging),
    )
}

#[tauri::command]
fn clear_log_lines() {
    logbuf::clear();
}

#[tauri::command]
fn dismiss_welcome(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    let mut settings = state
        .settings
        .lock()
        .map_err(|_| "Settings lock was poisoned")?
        .clone();
    settings.welcome_dismissed = true;
    persist_settings(&state.settings_path, &settings)?;
    *state
        .settings
        .lock()
        .map_err(|_| "Settings lock was poisoned")? = settings.clone();
    let _ = app.emit("settings-changed", settings);
    Ok(())
}

#[tauri::command]
fn selected_model_installed(state: State<'_, AppState>) -> bool {
    state
        .settings
        .lock()
        .map(|settings| model_is_installed(&state.models_path, &settings.selected_model))
        .unwrap_or(false)
}

#[tauri::command]
async fn start_mic_test(app: AppHandle) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<AppState>();
        let flag = *state
            .recording
            .lock()
            .map_err(|_| "Recording lock was poisoned")?;
        let live = state.recorder.capture_live();
        match mic_test_gate(flag, live) {
            MicTestGate::RefuseLiveSession => {
                let _ = app.emit("recording-changed", true);
                return Err("Stop dictation before running Mic Test.".into());
            }
            MicTestGate::ClearStaleThenAllow => {
                match state.recorder.stop() {
                    Ok(_) => {}
                    Err(error) if is_stale_stop_error(&error) => {}
                    Err(_) => {}
                }
                set_recording_flag(&state, false);
                let _ = app.emit("recording-changed", false);
            }
            MicTestGate::Allow => {}
        }
        let device = state
            .settings
            .lock()
            .map(|settings| settings.input_device.clone())
            .unwrap_or_default();
        let result = state.recorder.start_meter(device);
        if result.is_ok() {
            logbuf::debug_and_emit(&app, "Mic test started.");
        }
        result
    })
    .await
    .map_err(|error| format!("Mic Test was cancelled: {error}"))?
}

#[tauri::command]
async fn stop_mic_test(app: AppHandle) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<AppState>();
        state.recorder.stop()?;
        logbuf::debug_and_emit(&app, "Mic test stopped.");
        Ok(())
    })
    .await
    .map_err(|error| format!("Mic Test stop was cancelled: {error}"))?
}

#[tauri::command]
fn get_mic_level(state: State<'_, AppState>) -> f32 {
    state.recorder.level()
}

#[tauri::command]
fn get_hotkey_presets() -> Vec<serde_json::Value> {
    hotkey::PRESETS
        .iter()
        .map(|(id, label)| serde_json::json!({ "id": id, "label": label }))
        .collect()
}

#[tauri::command]
fn list_input_devices() -> Result<Vec<devices::InputDevice>, String> {
    devices::list_input_devices()
}

#[tauri::command]
fn recommend_model() -> hardware::ModelRecommendation {
    hardware::recommend_starting_model()
}

fn park_kind(reason: &ParkReason) -> &'static str {
    match reason {
        ParkReason::None => "",
        ParkReason::IdleTimeout => "idle",
        ParkReason::AutoPause(_) => "autopause",
    }
}

fn park_detail(reason: &ParkReason, idle_seconds: u32) -> String {
    match reason {
        ParkReason::None => String::new(),
        ParkReason::IdleTimeout => {
            let minutes = idle_seconds / 60;
            if minutes >= 2 && idle_seconds % 60 == 0 {
                format!("Unloaded to save RAM after {minutes} minutes idle.")
            } else {
                format!("Unloaded to save RAM after {idle_seconds} seconds idle.")
            }
        }
        ParkReason::AutoPause(app) => {
            format!("Paused because {app} launched.")
        }
    }
}

fn runtime_status_value(state: &AppState) -> serde_json::Value {
    let settings = state.settings.lock().map(|s| s.clone()).unwrap_or_default();
    let flag = *state.recording.lock().unwrap_or_else(|e| e.into_inner());
    let live = state.recorder.capture_live();
    if flag && !live {
        set_recording_flag(state, false);
    }
    let recording = session_is_live(flag, live);
    let paused = *state
        .dictation_paused
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    let park = state
        .park_reason
        .lock()
        .map(|reason| reason.clone())
        .unwrap_or_default();
    let model_loaded = state.whisper_cache.is_loaded() || ONNX_MODELS.is_loaded();
    let gpu = gpu::detect_gpu();
    let status = if recording {
        "Recording"
    } else if matches!(park, ParkReason::AutoPause(_)) || paused {
        "Paused"
    } else if matches!(park, ParkReason::IdleTimeout) {
        "Unloaded"
    } else {
        "Ready"
    };
    serde_json::json!({
        "status": status,
        "recording": recording,
        "paused": paused || matches!(park, ParkReason::AutoPause(_)),
        "modelLoaded": model_loaded,
        "parkKind": park_kind(&park),
        "parkDetail": park_detail(&park, settings.idle_unload_seconds),
        "hotkey": settings.hotkey,
        "inputDevice": if settings.input_device.is_empty() { "Default microphone".into() } else { settings.input_device },
        "gpuName": gpu.name,
        "gpuBackend": gpu.backend,
        "gpuDetail": gpu.detail,
        "gpuDiscrete": gpu.discrete,
        "gpuVramMb": gpu.vram_mb,
    })
}

fn emit_runtime(app: &AppHandle) {
    if let Some(state) = app.try_state::<AppState>() {
        let _ = app.emit("runtime-status", runtime_status_value(&state));
    }
}

#[tauri::command]
fn get_runtime_status(state: State<'_, AppState>) -> serde_json::Value {
    runtime_status_value(&state)
}

#[tauri::command]
fn list_running_apps() -> Vec<autopause::RunningApp> {
    autopause::list_running_apps()
}

fn inject_transcript(state: &AppState, text: &str) -> Result<(), String> {
    let options = state
        .settings
        .lock()
        .map(|settings| inject_options(&settings))
        .unwrap_or_default();
    logbuf::debug(format!(
        "Inject {} chars (copy_to_clipboard={}, paste_everywhere={})",
        text.chars().count(),
        options.copy_to_clipboard,
        options.paste_everywhere
    ));
    match output::inject(text, &options) {
        Ok(()) => Ok(()),
        Err(error) => {
            logbuf::error(format!("Inject failed: {error}"));
            Err(error)
        }
    }
}

fn inject_options(settings: &Settings) -> output::InjectOptions {
    output::InjectOptions {
        copy_to_clipboard: settings.copy_to_clipboard,
        paste_everywhere: settings.insertion_mode == "paste",
        paste_apps: autopause::parse_app_list(&settings.paste_apps),
    }
}

/// On Windows this is the final platform boundary: the recognizer gives us
/// text, then the injector enters it at the focused application. It is kept
/// separate from recognition so every engine gets identical insertion behavior.
#[tauri::command]
fn inject_text(text: String, state: State<'_, AppState>) -> Result<(), String> {
    if text.trim().is_empty() {
        return Ok(());
    }
    inject_transcript(&*state, &text)?;
    let pending = state
        .pending_window_take
        .lock()
        .ok()
        .and_then(|mut pending| pending.take());
    if let Some((dictated, speech_ms)) = pending.filter(|(dictated, _)| *dictated == text) {
        record_stats(&state, &dictated, speech_ms);
    }
    Ok(())
}

/// Stats count a dictation only once its text reached the app.
fn record_stats(state: &AppState, text: &str, speech_ms: u64) {
    if let Err(error) = stats::record(&state.stats_path, text, speech_ms) {
        logbuf::warn(error);
    }
}

/// What the overlay page should show when it (re)loads.
#[tauri::command]
fn get_overlay_phase() -> overlay::Payload {
    overlay::current()
}

#[tauri::command]
fn set_overlay_width(width: f64, app: AppHandle) {
    overlay::set_width(&app, width);
}

#[tauri::command]
fn dismiss_overlay(app: AppHandle) {
    overlay::dismiss(&app);
}

#[tauri::command]
fn copy_text(text: String) -> Result<(), String> {
    output::copy_to_clipboard(&text)
}

#[tauri::command]
fn preview_sound(theme: String, start: bool) -> Result<(), String> {
    sounds::preview_theme(&theme, start)
}

fn allowed_external_url(url: &str) -> bool {
    const ALLOWED: &[&str] = &[
        "https://vocawin.com",
        "https://vocahq.com",
        "https://vocalinux.com",
        "https://vocamac.com",
        "https://vocaphone.vocahq.com",
        "https://vocagateway.vocahq.com",
        "https://discord.gg/t6muquAJbm",
        "https://x.com/vocahq",
        "https://github.com/VocaHQ/vocawin",
        "https://github.com/VocaHQ/vocawin/issues/new/choose",
        "mailto:hello@vocahq.com",
    ];
    ALLOWED.contains(&url)
}

#[tauri::command]
fn open_external(url: String) -> Result<(), String> {
    if !allowed_external_url(&url) {
        return Err("That link is not on the VocaWin allow list.".into());
    }
    #[cfg(windows)]
    {
        std::process::Command::new("cmd")
            .args(["/C", "start", "", &url])
            .spawn()
            .map_err(|error| format!("Could not open link: {error}"))?;
        return Ok(());
    }
    #[cfg(not(windows))]
    {
        let opener = if url.starts_with("mailto:") {
            "xdg-open"
        } else {
            "xdg-open"
        };
        let _ = std::process::Command::new(opener).arg(&url).spawn();
        Ok(())
    }
}

pub fn run() {
    let start_minimized = std::env::args().any(|arg| arg == "--start-minimized");
    let mut builder = tauri::Builder::default();
    builder = builder.plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
        if let Some(window) = app.get_webview_window("main") {
            let _ = window.show();
            let _ = window.set_focus();
        }
    }));
    builder
        .plugin(
            tauri_plugin_autostart::Builder::new()
                .args(["--start-minimized"])
                .build(),
        )
        .setup(move |app| {
            let app_data = app.path().app_data_dir()?;
            let settings_path = app_data.join("settings.json");
            let history = history::HistoryStore::new(
                app_data.join("history.json"),
                app_data.join("history-audio"),
            );
            let stats_path = app_data.join("stats.json");
            let models_path = app_data.join("models");
            fs::create_dir_all(&models_path)?;
            let mut settings = load_settings(&settings_path);
            if fallback_selected_model_if_needed(&mut settings, &models_path) {
                if let Err(error) = persist_settings(&settings_path, &settings) {
                    eprintln!("VocaWin could not persist selected model fallback: {error}");
                }
            }
            let handle = app.handle().clone();
            let whisper_cache = whisper_cache::WhisperCache::new(handle.clone());
            whisper_cache
                .configure_idle(settings.idle_unload_enabled, settings.idle_unload_seconds);
            logbuf::set_debug_enabled(settings.debug_logging);
            if let Err(error) = history.recover_pending() {
                logbuf::warn(error);
            }
            if let Err(error) = history.prune(settings.history_retention_days) {
                logbuf::warn(error);
            }
            app.manage(AppState {
                settings: Mutex::new(settings.clone()),
                settings_path,
                history,
                stats_path,
                models_path,
                downloads: Mutex::new(HashMap::new()),
                recorder: AudioRecorder::new(handle.clone()),
                recording: Mutex::new(false),
                session_opening: AtomicBool::new(false),
                release_during_open: AtomicBool::new(false),
                inject_on_auto_stop: Mutex::new(true),
                registered_hotkey: Mutex::new(settings.hotkey.clone()),
                dictation_paused: Mutex::new(false),
                park_reason: Mutex::new(ParkReason::None),
                saw_model_loaded: Mutex::new(false),
                whisper_cache,
                session_trigger: Mutex::new(Trigger::default()),
                processing: AtomicBool::new(false),
                session_id: AtomicU64::new(0),
                cancellations: Mutex::new(Cancellations::default()),
                last_dictation: Mutex::new(String::new()),
                pending_window_take: Mutex::new(None),
                ducker: ducking::Ducker::new(),
            });
            if let Err(error) = apply_launch_at_login(&handle, settings.launch_at_login) {
                eprintln!("VocaWin launch-at-login registration failed: {error}");
                logbuf::error(format!("Launch-at-login registration failed: {error}"));
            }
            hook::start(handle.clone());
            if let Err(error) = register_dictation_hotkey(&handle, &settings.hotkey) {
                eprintln!("VocaWin hotkey registration failed: {error}");
                logbuf::error(format!("Hotkey registration failed: {error}"));
            }
            setup_tray(app)?;
            start_auto_pause_watcher(handle.clone());
            power::start_sleep_wake_watcher(handle.clone(), |app| {
                let state = app.state::<AppState>();
                let paused = *state
                    .dictation_paused
                    .lock()
                    .unwrap_or_else(|e| e.into_inner());
                if paused {
                    return;
                }
                let hotkey = state
                    .registered_hotkey
                    .lock()
                    .map(|value| value.clone())
                    .unwrap_or_else(|_| "AltRight".into());
                if let Err(error) = register_dictation_hotkey(&app, &hotkey) {
                    eprintln!("VocaWin hotkey re-register after wake failed: {error}");
                    logbuf::error(format!("Hotkey re-register after wake failed: {error}"));
                } else {
                    logbuf::info("Hotkey re-registered after wake.");
                    emit_runtime(&app);
                }
            });
            if let Some(window) = app.get_webview_window("main") {
                let window_for_close = window.clone();
                window.on_window_event(move |event| {
                    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                        api.prevent_close();
                        let _ = window_for_close.hide();
                    }
                });
                if start_minimized {
                    let _ = window.hide();
                }
            }
            if let Err(error) = overlay::create(&handle) {
                logbuf::warn(format!("Could not create the status overlay: {error}"));
            } else {
                let ready = handle.clone();
                std::thread::spawn(move || {
                    // Give the overlay page a moment to load before it shows.
                    std::thread::sleep(std::time::Duration::from_millis(700));
                    show_ready_pill(&ready);
                });
            }
            let gpu = gpu::detect_gpu();
            logbuf::debug(format!("GPU: {} ({})", gpu.name, gpu.backend));
            logbuf::info("VocaWin ready.");
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_settings,
            save_settings,
            get_history,
            clear_history,
            get_models,
            get_model_statuses,
            download_model,
            delete_model,
            system_summary,
            get_gpu_status,
            get_log_lines,
            get_debug_report,
            clear_log_lines,
            dismiss_welcome,
            selected_model_installed,
            start_mic_test,
            stop_mic_test,
            get_mic_level,
            get_hotkey_presets,
            pause_hotkey_listener,
            resume_hotkey_listener,
            start_hotkey_capture,
            stop_hotkey_capture,
            list_input_devices,
            recommend_model,
            get_runtime_status,
            list_running_apps,
            start_recording,
            stop_and_transcribe,
            preview_sound,
            inject_text,
            copy_text,
            open_external,
            delete_history_entry,
            retry_history_entry,
            play_history_audio,
            stop_history_audio,
            get_stats,
            reset_stats,
            preview_text,
            export_settings,
            parse_settings_backup,
            get_overlay_phase,
            set_overlay_width,
            dismiss_overlay
        ])
        .run(tauri::generate_context!())
        .expect("error while running VocaWin");
}

fn register_dictation_hotkey(app: &AppHandle, hotkey_spec: &str) -> Result<(), String> {
    let binding = hotkey::parse_hotkey(hotkey_spec)?;
    hook::set_binding(binding);
    logbuf::debug(format!("Hotkey bound to {hotkey_spec}"));
    if let Some(state) = app.try_state::<AppState>() {
        if let Ok(settings) = state.settings.lock() {
            hook::set_safety_timeout(safety_timeout_for(settings.max_recording_seconds));
            apply_extra_shortcuts(&settings);
        }
    }
    Ok(())
}

/// Hands-free, paste-last, and the mouse button. Invalid values were
/// rejected by `save_settings`; anything unparsable is simply left unbound.
fn apply_extra_shortcuts(settings: &Settings) {
    let mut actions = Vec::new();
    for (spec, event) in [
        (&settings.hands_free_hotkey, hook::HookEvent::HandsFree),
        (&settings.paste_last_hotkey, hook::HookEvent::PasteLast),
    ] {
        if spec.trim().is_empty() {
            continue;
        }
        match hotkey::parse_hotkey(spec) {
            Ok(parsed) => actions.push((parsed, event)),
            Err(error) => logbuf::warn(format!("Shortcut {spec} ignored: {error}")),
        }
    }
    hook::set_action_bindings(actions);
    hook::set_mouse_button(hook::MouseButton::parse(&settings.mouse_button));
}

/// A shortcut for hands-free or paste-last: a non-empty value must parse, may
/// not be a lone modifier (those are hold keys), and may not repeat another.
fn validate_extra_shortcut(value: &str, label: &str, taken: &[&str]) -> Result<String, String> {
    if value.trim().is_empty() {
        return Ok(String::new());
    }
    let canonical = hotkey::canonicalize(value)?;
    if let hotkey::HotkeySpec::Lone { vk } = hotkey::parse_hotkey(&canonical)? {
        if hotkey::is_modifier_vk(vk) {
            return Err(format!(
                "{label} needs a function key or a combo such as Ctrl+Alt+V, not a lone modifier."
            ));
        }
    }
    for other in taken {
        if !other.trim().is_empty()
            && hotkey::parse_hotkey(other).ok() == hotkey::parse_hotkey(&canonical).ok()
        {
            return Err(format!("{label} is already used by another VocaWin shortcut."));
        }
    }
    Ok(canonical)
}

#[tauri::command]
fn pause_hotkey_listener() {
    // Mac stops its tap while Record is open so the capture UI can see keys.
    hook::set_capture_paused(true);
}

#[tauri::command]
fn resume_hotkey_listener() {
    hook::set_capture_paused(false);
}

/// Record a shortcut in the keyboard hook, which sees keys an IME or the
/// webview would take (Ctrl+Space). The result arrives as
/// `hotkey-captured`. False when the hook is not running, and the page
/// records keys itself.
#[tauri::command]
fn start_hotkey_capture() -> bool {
    hook::begin_capture()
}

#[tauri::command]
fn stop_hotkey_capture() {
    hook::end_capture();
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PressDecision {
    Start,
    Stop,
    Ignore,
}

/// What a press of the hotkey, the mouse button, or the hands-free shortcut
/// does. The hands-free shortcut stops whatever is running; in toggle mode a
/// second hotkey or mouse press ends the take (but not a hands-free one); a
/// held key's typematic repeats do nothing.
fn press_decision(live: bool, toggle_mode: bool, pressed: Trigger, running: Trigger) -> PressDecision {
    if !live {
        return PressDecision::Start;
    }
    match pressed {
        Trigger::HandsFree => PressDecision::Stop,
        Trigger::Toggle | Trigger::Mouse if toggle_mode && running != Trigger::HandsFree => {
            PressDecision::Stop
        }
        _ => PressDecision::Ignore,
    }
}

/// A key-up ends only the push-to-talk hold that started the take.
fn release_ends_session(toggle_mode: bool, released: Trigger, running: Trigger) -> bool {
    !toggle_mode && released == running
}

#[cfg_attr(not(windows), allow(dead_code))]
pub(crate) fn on_hotkey_event(handle: &AppHandle, event: hook::HookEvent) {
    let state = handle.state::<AppState>();
    let paused = *state
        .dictation_paused
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    let settings = state.settings.lock().map(|s| s.clone()).unwrap_or_default();
    let toggle = settings.activation_mode == "toggle";
    let hotkey_trigger = if toggle { Trigger::Toggle } else { Trigger::Hold };
    match event {
        hook::HookEvent::Pressed if !paused => press(handle, &settings, hotkey_trigger),
        hook::HookEvent::MouseDown if !paused => press(handle, &settings, Trigger::Mouse),
        hook::HookEvent::HandsFree if !paused => press(handle, &settings, Trigger::HandsFree),
        hook::HookEvent::Released => release(handle, toggle, hotkey_trigger),
        hook::HookEvent::MouseUp => release(handle, toggle, Trigger::Mouse),
        hook::HookEvent::Cancel(session) => cancel_voice_session(handle, session),
        hook::HookEvent::PasteLast if !paused => paste_last(handle),
        _ => {}
    }
}

fn press(handle: &AppHandle, settings: &Settings, trigger: Trigger) {
    let state = handle.state::<AppState>();
    let flag = *state.recording.lock().unwrap_or_else(|e| e.into_inner());
    let live = ptt_pressed_action(flag, state.recorder.capture_live()) == PttPressedAction::Ignore;
    let running = *state
        .session_trigger
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    let toggle = settings.activation_mode == "toggle";
    match press_decision(live, toggle, trigger, running) {
        PressDecision::Ignore => {}
        PressDecision::Stop => {
            logbuf::debug_and_emit(handle, "Shortcut pressed again; stopping.");
            finish_voice_session(handle);
        }
        PressDecision::Start => {
            logbuf::debug_and_emit(handle, "Hotkey pressed.");
            let silence_auto_stop = match trigger {
                Trigger::Toggle => true,
                Trigger::Mouse => toggle,
                _ => false,
            };
            if let Err(error) = begin_voice_session(handle, true, trigger, silence_auto_stop) {
                if error.contains("No speech model is installed") {
                    sounds::play_error_if_enabled(&settings.sound_theme);
                    logbuf::error_and_emit(handle, error.clone());
                    let _ = handle.emit("dictation-error", error.clone());
                    overlay::show(
                        handle,
                        overlay::Phase::Error("Download a speech model first".into()),
                        overlay_on(settings),
                    );
                }
            }
        }
    }
}

fn release(handle: &AppHandle, toggle: bool, trigger: Trigger) {
    let state = handle.state::<AppState>();
    let running = *state
        .session_trigger
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    if !release_ends_session(toggle, trigger, running) {
        return;
    }
    logbuf::debug_and_emit(handle, "Hotkey released.");
    let recording = *state.recording.lock().unwrap_or_else(|e| e.into_inner());
    if recording {
        finish_voice_session(handle);
    } else if state.session_opening.load(Ordering::SeqCst) {
        state.release_during_open.store(true, Ordering::SeqCst);
    }
}

fn start_auto_pause_watcher(app: AppHandle) {
    std::thread::Builder::new()
        .name("vocawin-autopause".into())
        .spawn(move || loop {
            std::thread::sleep(std::time::Duration::from_secs(2));
            let state = app.state::<AppState>();
            let settings = match state.settings.lock() {
                Ok(settings) => settings.clone(),
                Err(_) => continue,
            };
            let watched = autopause::parse_app_list(&settings.auto_pause_apps);
            let hit = if watched.is_empty() {
                None
            } else {
                autopause::matching_process_name(&watched)
            };
            let should_pause = hit.is_some();
            let mut paused = match state.dictation_paused.lock() {
                Ok(guard) => guard,
                Err(_) => continue,
            };
            let mut tray_dirty = false;
            if should_pause && !*paused {
                *paused = true;
                drop(paused);
                hook::set_dictation_paused(true);
                hook::clear_held_vk();
                hook::set_cancel_armed(None);
                abandon_voice_session(&state);
                overlay::hide(&app);
                let _ = app.emit("recording-changed", false);
                state.whisper_cache.unload();
                // A watched app is running: close the microphone too.
                state.recorder.release_microphone();
                ONNX_MODELS.unload();
                let app_name = hit.clone().unwrap_or_else(|| "a watched app".into());
                if let Ok(mut park) = state.park_reason.lock() {
                    *park = ParkReason::AutoPause(app_name.clone());
                }
                logbuf::info_and_emit(&app, format!("Paused while {app_name} is running."));
                tray_dirty = true;
            } else if !should_pause && *paused {
                *paused = false;
                drop(paused);
                hook::set_dictation_paused(false);
                if let Ok(mut park) = state.park_reason.lock() {
                    if matches!(*park, ParkReason::AutoPause(_)) {
                        *park = ParkReason::None;
                    }
                }
                if let Err(error) = register_dictation_hotkey(&app, &settings.hotkey) {
                    eprintln!("VocaWin hotkey restore after auto-pause failed: {error}");
                    logbuf::error_and_emit(
                        &app,
                        format!("Hotkey restore after auto-pause failed: {error}"),
                    );
                } else {
                    logbuf::info_and_emit(&app, "Auto-pause cleared. Hotkey restored.");
                }
                tray_dirty = true;
            } else {
                drop(paused);
            }

            if settings.idle_unload_enabled {
                ONNX_MODELS.unload_if_idle(std::time::Duration::from_secs(
                    settings.idle_unload_seconds.max(30) as u64,
                ));
            }
            let loaded = state.whisper_cache.is_loaded() || ONNX_MODELS.is_loaded();
            let mut prev_loaded = match state.saw_model_loaded.lock() {
                Ok(guard) => guard,
                Err(_) => continue,
            };
            let was_loaded = *prev_loaded;
            *prev_loaded = loaded;
            drop(prev_loaded);
            if !should_pause && settings.idle_unload_enabled {
                if was_loaded && !loaded {
                    if let Ok(mut park) = state.park_reason.lock() {
                        if *park == ParkReason::None {
                            *park = ParkReason::IdleTimeout;
                            logbuf::info_and_emit(&app, "Unloaded the speech model after idle.");
                            tray_dirty = true;
                        }
                    }
                } else if loaded {
                    if let Ok(mut park) = state.park_reason.lock() {
                        if *park == ParkReason::IdleTimeout {
                            *park = ParkReason::None;
                            tray_dirty = true;
                        }
                    }
                }
            }

            if tray_dirty {
                emit_runtime(&app);
                apply_ready_or_parked_tray(&app);
            }
        })
        .ok();
}

fn setup_tray(app: &tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};

    let icon = match tauri::image::Image::from_bytes(include_bytes!("../icons/tray-idle.png")) {
        Ok(icon) => icon,
        Err(_) => app
            .default_window_icon()
            .ok_or("VocaWin is missing a tray icon")?
            .clone(),
    };
    let menu = build_tray_menu(app.handle())?;

    TrayIconBuilder::with_id("main")
        .icon(icon)
        .tooltip("VocaWin")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "start_voice" => {
                if let Err(error) = tray_start_voice(app) {
                    logbuf::error_and_emit(app, format!("Start Voice Typing failed: {error}"));
                }
            }
            "stop_voice" => {
                if let Err(error) = tray_stop_voice(app) {
                    logbuf::error_and_emit(app, format!("Stop Voice Typing failed: {error}"));
                }
            }
            "start_on_login" => {
                if let Err(error) = tray_toggle_login(app) {
                    logbuf::error_and_emit(app, format!("Start on Login failed: {error}"));
                }
            }
            "settings" => {
                show_main_window(app);
                let _ = app.emit("navigate", "settings");
            }
            "debug" => {
                show_main_window(app);
                let _ = app.emit("navigate", "debug");
            }
            "about" => {
                show_main_window(app);
                let _ = app.emit("navigate", "about");
            }
            "quit" => {
                app.exit(0);
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                let app = tray.app_handle();
                if let Some(window) = app.get_webview_window("main") {
                    if window.is_visible().unwrap_or(false) {
                        let _ = window.hide();
                    } else {
                        let _ = window.show();
                        let _ = window.set_focus();
                    }
                }
            }
        })
        .build(app)?;
    Ok(())
}

fn build_tray_menu(app: &AppHandle) -> Result<tauri::menu::Menu<tauri::Wry>, String> {
    use tauri::menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem};

    let state = app.try_state::<AppState>();
    let recording = state
        .as_ref()
        .and_then(|s| s.recording.lock().ok().map(|v| *v))
        .unwrap_or(false);
    let launch = state
        .as_ref()
        .and_then(|s| {
            s.settings
                .lock()
                .ok()
                .map(|settings| settings.launch_at_login)
        })
        .unwrap_or(false);

    let start = MenuItem::with_id(
        app,
        "start_voice",
        "Start Voice Typing",
        !recording,
        None::<&str>,
    )
    .map_err(|error| error.to_string())?;
    let stop = MenuItem::with_id(
        app,
        "stop_voice",
        "Stop Voice Typing",
        recording,
        None::<&str>,
    )
    .map_err(|error| error.to_string())?;
    let login = CheckMenuItem::with_id(
        app,
        "start_on_login",
        "Start on Login",
        true,
        launch,
        None::<&str>,
    )
    .map_err(|error| error.to_string())?;
    let settings = MenuItem::with_id(app, "settings", "Settings", true, None::<&str>)
        .map_err(|error| error.to_string())?;
    let debug = MenuItem::with_id(app, "debug", "Debug", true, None::<&str>)
        .map_err(|error| error.to_string())?;
    let about = MenuItem::with_id(app, "about", "About", true, None::<&str>)
        .map_err(|error| error.to_string())?;
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)
        .map_err(|error| error.to_string())?;
    let sep1 = PredefinedMenuItem::separator(app).map_err(|error| error.to_string())?;
    let sep2 = PredefinedMenuItem::separator(app).map_err(|error| error.to_string())?;
    let sep3 = PredefinedMenuItem::separator(app).map_err(|error| error.to_string())?;

    Menu::with_items(
        app,
        &[
            &start, &stop, &sep1, &login, &sep2, &settings, &debug, &about, &sep3, &quit,
        ],
    )
    .map_err(|error| error.to_string())
}

fn refresh_tray_menu(app: &AppHandle) -> Result<(), String> {
    let menu = build_tray_menu(app)?;
    if let Some(tray) = app.tray_by_id("main") {
        tray.set_menu(Some(menu))
            .map_err(|error| error.to_string())?;
    }
    Ok(())
}

fn show_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.set_focus();
    }
}

fn tray_start_voice(app: &AppHandle) -> Result<(), String> {
    let toggle = manual_silence_auto_stop(app);
    begin_voice_session(app, true, Trigger::Manual, toggle)
}

fn tray_stop_voice(app: &AppHandle) -> Result<(), String> {
    let state = app.state::<AppState>();
    if !*state
        .recording
        .lock()
        .map_err(|_| "Recording lock was poisoned")?
    {
        return Ok(());
    }
    finish_voice_session(app);
    Ok(())
}

fn tray_toggle_login(app: &AppHandle) -> Result<(), String> {
    let state = app.state::<AppState>();
    let mut settings = state
        .settings
        .lock()
        .map_err(|_| "Settings lock was poisoned")?
        .clone();
    let previous_launch = settings.launch_at_login;
    settings.launch_at_login = !settings.launch_at_login;
    persist_settings(&state.settings_path, &settings)?;
    if let Err(error) = apply_launch_at_login(app, settings.launch_at_login) {
        settings.launch_at_login = previous_launch;
        persist_settings(&state.settings_path, &settings)?;
        *state
            .settings
            .lock()
            .map_err(|_| "Settings lock was poisoned")? = settings;
        let _ = refresh_tray_menu(app);
        return Err(error);
    }
    *state
        .settings
        .lock()
        .map_err(|_| "Settings lock was poisoned")? = settings.clone();
    let _ = app.emit("settings-changed", settings);
    refresh_tray_menu(app)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 50 s of tone with a 0.4 s gap near each 16.7 s split, so it decodes
    /// as three windows.
    fn three_window_take() -> Vec<f32> {
        let tone = |seconds: f32| -> Vec<f32> {
            (0..(seconds * 16_000.0) as usize)
                .map(|i| (i as f32 * 0.07).sin() * 0.3)
                .collect()
        };
        let mut pcm = tone(16.5);
        pcm.extend(vec![0.0; 6_400]);
        pcm.extend(tone(16.5));
        pcm.extend(vec![0.0; 6_400]);
        pcm.extend(tone(16.2));
        pcm
    }

    #[test]
    fn short_takes_are_decoded_in_one_call() {
        let pcm = vec![0.1; 16_000 * 5];
        let mut calls = Vec::new();
        let text = decode_in_windows(&pcm, |window| {
            calls.push(window.len());
            Ok(" hello ".into())
        })
        .unwrap();
        assert_eq!(calls, vec![pcm.len()]);
        assert_eq!(text, "hello");
    }

    #[test]
    fn window_texts_join_in_order_without_empty_ones() {
        let pcm = three_window_take();
        let mut index = 0;
        let text = decode_in_windows(&pcm, |_| {
            index += 1;
            Ok(match index {
                1 => " first part ".into(),
                2 => "   ".into(),
                _ => "third part".into(),
            })
        })
        .unwrap();
        assert_eq!(index, 3);
        assert_eq!(text, "first part third part");
    }

    #[test]
    fn an_empty_window_is_decoded_again_with_its_pauses_shortened() {
        // 0.8 s of pause, a click, 1 s more pause, then speech. The stub,
        // like Moonshine, returns nothing for audio that opens on half a
        // second or more of pause.
        let opens_on_pause =
            |window: &[f32]| window.iter().position(|s| s.abs() >= 0.1) >= Some(8_000);
        let mut pcm = vec![0.0; 8_000 + 4_800];
        pcm.extend(vec![1.0; 480]);
        pcm.extend(vec![0.0; 16_000]);
        pcm.extend((0..16_000 * 3).map(|i| (i as f32 * 0.07).sin() * 0.3));
        let mut decoded = Vec::new();
        let text = decode_in_windows(&pcm, |window| {
            decoded.push(window.to_vec());
            Ok(if opens_on_pause(window) {
                String::new()
            } else {
                "speech".into()
            })
        })
        .unwrap();
        assert_eq!(text, "speech");
        assert_eq!(decoded.len(), 2);
        assert_eq!(decoded[0], pcm);
        // The retry keeps the click and all of the speech.
        let loud = |audio: &[f32]| audio.iter().filter(|s| s.abs() >= 0.1).count();
        assert_eq!(loud(&decoded[1]), loud(&pcm));
        assert!(decoded[1].len() < pcm.len());

        // Text on the first pass: no second one.
        let mut calls = 0;
        decode_in_windows(&pcm, |_| {
            calls += 1;
            Ok("speech".into())
        })
        .unwrap();
        assert_eq!(calls, 1);
    }

    #[test]
    fn a_failed_window_fails_the_take() {
        let pcm = three_window_take();
        let mut calls = 0;
        let result = decode_in_windows(&pcm, |_| {
            calls += 1;
            if calls == 2 {
                Err("Canary transcription failed: boom".into())
            } else {
                Ok("text".into())
            }
        });
        assert_eq!(result, Err("Canary transcription failed: boom".into()));
        assert_eq!(calls, 2);
    }

    #[test]
    fn parakeet_text_from_the_usual_decode_is_kept() {
        let mut calls = Vec::new();
        let text = decode_parakeet(|lead_in| {
            calls.push(lead_in);
            Ok("I have a dream.".into())
        });
        assert_eq!(text, Ok("I have a dream.".into()));
        assert_eq!(calls, vec![true]);
    }

    #[test]
    fn an_empty_parakeet_decode_retries_without_the_lead_in() {
        let mut calls = Vec::new();
        let text = decode_parakeet(|lead_in| {
            calls.push(lead_in);
            Ok(if lead_in {
                "  ".into()
            } else {
                "Everything that money can buy.".into()
            })
        });
        assert_eq!(text, Ok("Everything that money can buy.".into()));
        assert_eq!(calls, vec![true, false]);

        // Empty both ways stays empty; the caller decides what that means.
        assert_eq!(decode_parakeet(|_| Ok(String::new())), Ok(String::new()));
    }

    #[test]
    fn a_failed_parakeet_decode_is_not_retried() {
        let mut calls = 0;
        let text = decode_parakeet(|_| {
            calls += 1;
            Err("Parakeet transcription failed: boom".into())
        });
        assert_eq!(text, Err("Parakeet transcription failed: boom".into()));
        assert_eq!(calls, 1);
    }
    #[test]
    fn catalog_has_unique_ids_and_voca_engines() {
        let catalog = model_catalog();
        let mut ids: Vec<_> = catalog.iter().map(|m| m.id).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), catalog.len());
        assert!(catalog.iter().any(|m| m.engine == "whisper.cpp"));
        assert!(catalog.iter().any(|m| m.engine == "ONNX Runtime"));
        assert!(catalog.iter().all(|m| model_package(m.id).is_some()));
        assert!(!catalog
            .iter()
            .any(|m| m.id.contains("vosk") || m.id.contains("ctc")));
    }

    #[test]
    fn voca_hinglish_is_a_whisper_model_pinned_to_english() {
        assert!(is_whisper_model(VOCA_HINGLISH));
        assert!(matches!(
            model_package(VOCA_HINGLISH),
            Some(ModelPackage::GgmlBin { .. })
        ));
        assert_eq!(
            model_path(Path::new("models"), VOCA_HINGLISH),
            Path::new("models").join("voca-hinglish.bin")
        );
        for language in ["Hindi", "Auto-detect", "English", "German"] {
            let settings = Settings {
                selected_model: VOCA_HINGLISH.into(),
                language: language.into(),
                ..Settings::default()
            };
            assert_eq!(decoder_language(&settings), Some("en"), "{language}");
            assert_eq!(text_language(&settings), Some("hi"), "{language}");
        }
        let whisper = Settings {
            selected_model: "whisper-small".into(),
            language: "Hindi".into(),
            ..Settings::default()
        };
        assert_eq!(decoder_language(&whisper), Some("hi"));
    }

    #[test]
    fn a_model_loads_with_the_accelerator_it_asked_for() {
        use transcribe_rs::accel::{get_ort_accelerator, OrtAccelerator};
        let threads: Vec<_> = (0..8)
            .map(|index| {
                std::thread::spawn(move || {
                    let wanted = if index % 2 == 0 {
                        OrtAccelerator::DirectMl
                    } else {
                        OrtAccelerator::CpuOnly
                    };
                    for _ in 0..50 {
                        let seen = load_onnx(wanted, || {
                            std::thread::yield_now();
                            Ok::<_, transcribe_rs::TranscribeError>(get_ort_accelerator())
                        })
                        .unwrap();
                        assert_eq!(seen, wanted);
                    }
                })
            })
            .collect();
        for thread in threads {
            thread.join().unwrap();
        }
    }

    #[test]
    fn directml_success_needs_no_cpu_pass() {
        let mut calls = Vec::new();
        let (result, broken) = decode_with_cpu_fallback(true, |gpu| {
            calls.push(gpu);
            Ok("text".into())
        });
        assert_eq!(result, Ok("text".into()));
        assert!(!broken);
        assert_eq!(calls, vec![true]);
    }

    #[test]
    fn directml_is_blamed_only_when_cpu_then_decodes() {
        let (result, broken) = decode_with_cpu_fallback(true, |gpu| {
            if gpu {
                Err("DirectML device lost".into())
            } else {
                Ok("text".into())
            }
        });
        assert_eq!(result, Ok("text".into()));
        assert!(broken);

        // A corrupt model fails on both: not DirectML's fault.
        let mut calls = Vec::new();
        let (result, broken) = decode_with_cpu_fallback(true, |gpu| {
            calls.push(gpu);
            Err(format!("Could not load Parakeet ({gpu})"))
        });
        assert_eq!(result, Err("Could not load Parakeet (false)".into()));
        assert!(!broken);
        assert_eq!(calls, vec![true, false]);
    }

    #[test]
    fn cpu_only_decodes_once() {
        let mut calls = Vec::new();
        let (result, broken) = decode_with_cpu_fallback(false, |gpu| {
            calls.push(gpu);
            Err("bad audio".into())
        });
        assert_eq!(result, Err("bad audio".into()));
        assert!(!broken);
        assert_eq!(calls, vec![false]);
    }

    #[test]
    fn encoder_decoder_models_are_told_the_chosen_language() {
        let flash = CANARY_FLASH_LANGUAGES;
        assert_eq!(prompt_language(Some("de"), flash).as_deref(), Some("de"));
        assert_eq!(prompt_language(Some("it"), flash), None);
        assert_eq!(prompt_language(None, flash), None);
        assert_eq!(
            prompt_language(Some("it"), CANARY_V2_LANGUAGES).as_deref(),
            Some("it")
        );
        assert_eq!(
            prompt_language(Some("ja"), COHERE_LANGUAGES).as_deref(),
            Some("ja")
        );
        assert_eq!(prompt_language(Some("hi"), COHERE_LANGUAGES), None);
    }

    #[test]
    fn every_canary_v2_language_can_be_chosen() {
        let chosen: Vec<_> = [
            "English", "Spanish", "French", "German", "Italian", "Portuguese", "Dutch", "Russian",
            "Polish", "Ukrainian", "Swedish", "Danish", "Finnish", "Czech", "Greek", "Romanian",
            "Hungarian", "Bulgarian", "Croatian", "Estonian", "Latvian", "Lithuanian", "Maltese",
            "Slovak", "Slovenian",
        ]
        .iter()
        .filter_map(|name| language_code(name))
        .collect();
        assert_eq!(chosen.len(), CANARY_V2_LANGUAGES.len());
        assert!(CANARY_V2_LANGUAGES.iter().all(|code| chosen.contains(code)));
    }

    #[test]
    fn new_onnx_models_download_what_their_loader_opens() {
        for (id, needed) in [
            (
                "canary-1b-v2",
                &["encoder-model.int8.onnx", "decoder-model.int8.onnx", "nemo128.onnx", "vocab.txt"][..],
            ),
            (
                "cohere-transcribe",
                &[
                    "cohere-encoder.int4.onnx",
                    "cohere-encoder.int4.onnx.data",
                    "cohere-decoder.int4.onnx",
                    "cohere-decoder.int4.onnx.data",
                    "tokens.txt",
                ][..],
            ),
        ] {
            let Some(ModelPackage::Files { files }) = model_package(id) else {
                panic!("{id} should download flat files");
            };
            let names: Vec<_> = files.iter().map(|(name, _)| *name).collect();
            assert_eq!(names.len(), needed.len(), "{id}");
            assert!(needed.iter().all(|file| names.contains(file)), "{id}");
            assert!(files.iter().all(|(name, url)| url.ends_with(name) && !url.contains("/main/")), "{id}");
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join(id);
            fs::create_dir_all(&path).unwrap();
            for file in needed {
                assert!(!model_is_installed(dir.path(), id), "{id} before {file}");
                fs::write(path.join(file), b"x").unwrap();
            }
            assert!(model_is_installed(dir.path(), id), "{id}");
        }
    }

    #[test]
    fn onnx_directml_label_matches_the_models_that_use_it() {
        let onnx: Vec<_> = model_catalog()
            .into_iter()
            .filter(|model| model.engine == "ONNX Runtime")
            .collect();
        assert!(!onnx.is_empty());
        for model in onnx {
            let expected = if cfg!(windows) && onnx_uses_directml(model.id) {
                "CPU · DirectML"
            } else {
                "CPU"
            };
            assert_eq!(model.acceleration, expected, "{}", model.id);
        }
        assert!(onnx_uses_directml("parakeet-tdt-0.6b-v3"));
        assert!(!onnx_uses_directml("canary-180m"));
        assert!(!onnx_uses_directml("moonshine-base"));
        assert!(!onnx_uses_directml("whisper-tiny"));
    }

    #[test]
    fn whisper_acceleration_matches_vulkan_cfg() {
        let expected = if cfg!(vocawin_whisper_vulkan) {
            "CPU · Vulkan"
        } else {
            "CPU"
        };
        assert_eq!(whisper_acceleration(), expected);
        assert!(model_catalog()
            .iter()
            .filter(|model| model.engine == "whisper.cpp")
            .all(|model| model.acceleration == expected));
        let backends = gpu_backends_summary();
        assert!(backends.contains(&"CPU fallback"));
        if cfg!(vocawin_whisper_vulkan) {
            assert!(backends.iter().any(|b| b.contains("Vulkan")));
        } else {
            assert!(!backends.iter().any(|b| b.contains("Vulkan")));
        }
    }

    fn tone(hz: f32, rate: u32, seconds: f32) -> Vec<f32> {
        (0..(rate as f32 * seconds) as usize)
            .map(|i| 0.5 * (2.0 * std::f32::consts::PI * hz * i as f32 / rate as f32).sin())
            .collect()
    }

    fn rms(samples: &[f32]) -> f32 {
        (samples.iter().map(|s| s * s).sum::<f32>() / samples.len() as f32).sqrt()
    }

    #[test]
    fn resampling_preserves_duration() {
        for rate in [48_000, 44_100, 22_050, 8_000] {
            let input = vec![0.5; rate as usize];
            let output = resample_to_16khz(&input, rate);
            assert_eq!(output.len(), 16_000, "{rate}");
            // Away from the edges, a constant stays constant.
            assert!(
                output[800..15_200].iter().all(|sample| (*sample - 0.5).abs() < 1e-3),
                "{rate}"
            );
        }
        assert_eq!(resample_to_16khz(&[0.25; 100], 16_000), vec![0.25; 100]);
    }

    #[test]
    fn resampling_keeps_speech_and_drops_what_would_alias() {
        // 1 kHz is speech band and passes; 12 kHz is above 16 kHz's Nyquist
        // and must vanish, where interpolation folded it down to 4 kHz.
        let speech = resample_to_16khz(&tone(1_000.0, 48_000, 1.0), 48_000);
        assert!((rms(&speech[1_000..15_000]) - 0.354).abs() < 0.01);
        let high = resample_to_16khz(&tone(12_000.0, 48_000, 1.0), 48_000);
        assert!(rms(&high[1_000..15_000]) < 0.01, "{}", rms(&high[1_000..15_000]));
        let folded = interpolate_to_16khz(&tone(12_000.0, 48_000, 1.0), 48_000);
        assert!(rms(&folded[1_000..15_000]) > 0.1);
    }

    fn click_position(len: usize, click: usize, rate: u32) -> (usize, usize) {
        let mut input = vec![0.0; len];
        input[click] = 1.0;
        let output = resample_to_16khz(&input, rate);
        let peak = output
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.abs().total_cmp(&b.1.abs()))
            .unwrap()
            .0;
        (peak, output.len())
    }

    #[test]
    fn resampling_keeps_timing() {
        // A click at 0.5 s lands at 0.5 s: the filter delay is removed.
        let (peak, _) = click_position(48_000, 24_000, 48_000);
        assert!((7_998..=8_002).contains(&peak), "{peak}");
    }

    #[test]
    fn resampling_keeps_the_end_of_the_take() {
        // The last partial chunk and the flushed filter tail: a click 30 ms
        // before the end is still there, on time, at 48 and 44.1 kHz.
        let (peak, len) = click_position(48_000, 46_560, 48_000);
        assert_eq!(len, 16_000);
        assert!((15_518..=15_522).contains(&peak), "{peak}");
        let (peak, len) = click_position(44_100, 42_777, 44_100);
        assert_eq!(len, 16_000);
        assert!((15_518..=15_522).contains(&peak), "{peak}");
        // An input that is a whole number of 1024-sample chunks has no
        // partial chunk; its end must survive the flush too.
        let (peak, len) = click_position(1024 * 48, 1024 * 48 - 1_440, 48_000);
        assert_eq!(len, 1024 * 48 / 3);
        assert!((len - 482..=len - 478).contains(&peak), "{peak} of {len}");
    }

    #[test]
    fn a_warm_microphone_is_reused_only_while_it_still_delivers_audio() {
        // Names are the devices' real names, the default resolved first.
        assert!(reuse_warm_stream("USB Mic", "USB Mic", 1_000, 1_500));
        assert!(!reuse_warm_stream("USB Mic", "Realtek Mic", 1_000, 1_200), "default changed");
        assert!(!reuse_warm_stream("USB Mic", "USB Mic", 1_000, 2_000), "stream went quiet");
    }

    #[test]
    fn a_snapshot_is_the_tail_of_the_take() {
        assert_eq!(tail_start(480_000, 48_000, 10.0), 0);
        assert_eq!(tail_start(960_000, 48_000, 10.0), 480_000);
        assert_eq!(tail_start(100, 16_000, 10.0), 0);
        assert!(!Settings::default().live_preview);
    }

    #[test]
    fn path_bytes_counts_files_and_nested_dirs() {
        let directory = tempfile::tempdir().unwrap();
        let file = directory.path().join("one.bin");
        fs::write(&file, vec![0u8; 40]).unwrap();
        assert_eq!(path_bytes(&file), 40);
        let nested = directory.path().join("nested");
        fs::create_dir_all(&nested).unwrap();
        fs::write(nested.join("a"), vec![0u8; 10]).unwrap();
        fs::write(nested.join("b"), vec![0u8; 15]).unwrap();
        assert_eq!(path_bytes(directory.path()), 65);
    }

    #[test]
    fn download_staging_path_never_equals_a_partial_tar_dest() {
        let dest = PathBuf::from("models/gigaam-v3.tar.gz.partial");
        assert_eq!(dest.with_extension("partial"), dest);
        let staging = download_staging_path(&dest);
        assert_ne!(staging, dest);
        assert_eq!(
            staging.file_name().and_then(|name| name.to_str()),
            Some("gigaam-v3.tar.gz.partial.part")
        );
        assert_eq!(
            download_staging_path(Path::new("whisper-tiny.bin"))
                .file_name()
                .and_then(|name| name.to_str()),
            Some("whisper-tiny.bin.part")
        );
        assert_eq!(
            download_staging_path(Path::new("moonshine-tiny/encoder_model.onnx"))
                .file_name()
                .and_then(|name| name.to_str()),
            Some("encoder_model.onnx.part")
        );
        assert_eq!(
            url_host("https://blob.handy.computer/giga-am-v3-int8.tar.gz"),
            "blob.handy.computer"
        );
    }

    #[test]
    fn settings_round_trip_to_disk() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("VocaWin/settings.json");
        let settings = Settings {
            hotkey: "Ctrl+Shift+V".into(),
            ..Settings::default()
        };
        persist_settings(&path, &settings).unwrap();
        let loaded = load_settings(&path);
        assert_eq!(loaded.hotkey, "Ctrl+Shift+V");
        assert_eq!(loaded.sound_theme, "voca");
        assert!(loaded.sound_effects);
        assert!(loaded.custom_vocabulary.is_empty());
    }

    #[test]
    fn custom_vocabulary_round_trips_like_phone() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("VocaWin/settings.json");
        let settings = Settings {
            custom_vocabulary: "Claude Code\nTailscale, VocaPhone".into(),
            ..Settings::default()
        };
        persist_settings(&path, &settings).unwrap();
        assert_eq!(
            load_settings(&path).custom_vocabulary,
            "Claude Code\nTailscale, VocaPhone"
        );
        assert_eq!(
            vocabulary::whisper_prompt(&settings.custom_vocabulary),
            "Claude Code, Tailscale, VocaPhone."
        );
    }

    #[test]
    fn old_settings_json_without_theme_still_loads() {
        let directory = tempfile::tempdir().unwrap();
        let on_path = directory.path().join("on.json");
        fs::write(
            &on_path,
            r#"{"hotkey":"AltRight","activationMode":"pushToTalk","language":"Auto-detect","silenceSeconds":1.5,"maxRecordingSeconds":60.0,"launchAtLogin":false,"soundEffects":true,"appendTrailingSpace":true,"autoCapitalize":true,"selectedModel":"whisper-tiny"}"#,
        )
        .unwrap();
        let on = load_settings(&on_path);
        assert_eq!(on.sound_theme, "voca");
        assert!(on.sound_effects);
        assert!(on.history_enabled);
        assert!(!on.debug_logging);

        let off_path = directory.path().join("off.json");
        fs::write(
            &off_path,
            r#"{"hotkey":"AltRight","activationMode":"pushToTalk","language":"Auto-detect","silenceSeconds":1.5,"maxRecordingSeconds":60.0,"launchAtLogin":false,"soundEffects":false,"appendTrailingSpace":true,"autoCapitalize":true,"selectedModel":"whisper-tiny"}"#,
        )
        .unwrap();
        let off = load_settings(&off_path);
        assert_eq!(off.sound_theme, "off");
        assert!(!off.sound_effects);

        let fifth_path = directory.path().join("fifth.json");
        fs::write(
            &fifth_path,
            r#"{"hotkey":"AltRight","activationMode":"pushToTalk","language":"Auto-detect","silenceSeconds":1.5,"maxRecordingSeconds":60.0,"launchAtLogin":false,"soundEffects":true,"soundTheme":"fifth","appendTrailingSpace":true,"autoCapitalize":true,"selectedModel":"whisper-tiny"}"#,
        )
        .unwrap();
        let fifth = load_settings(&fifth_path);
        assert_eq!(fifth.sound_theme, "voca");
        assert!(fifth.sound_effects);
    }

    #[test]
    fn default_sound_theme_is_voca() {
        assert_eq!(Settings::default().sound_theme, "voca");
        assert!(Settings::default().sound_effects);
    }

    #[test]
    fn default_selected_model_is_whisper_tiny() {
        assert_eq!(Settings::default().selected_model, "whisper-tiny");
    }

    #[test]
    fn history_stays_on_and_debug_logs_stay_off() {
        let settings = Settings::default();
        assert!(settings.history_enabled);
        assert!(!settings.debug_logging);
    }

    #[test]
    fn copy_to_clipboard_stays_off_by_default() {
        assert!(!Settings::default().copy_to_clipboard);
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("legacy.json");
        fs::write(
            &path,
            r#"{"hotkey":"AltRight","activationMode":"pushToTalk","language":"Auto-detect","silenceSeconds":1.5,"maxRecordingSeconds":60.0,"launchAtLogin":false,"soundEffects":true,"appendTrailingSpace":true,"autoCapitalize":true,"selectedModel":"whisper-tiny"}"#,
        )
        .unwrap();
        assert!(!load_settings(&path).copy_to_clipboard);
    }

    #[test]
    fn idle_never_is_the_default_keep_in_ram() {
        assert!(!Settings::default().idle_unload_enabled);
    }

    #[test]
    fn park_copy_names_idle_and_app() {
        assert!(park_detail(&ParkReason::IdleTimeout, 300).contains("5 minutes"));
        assert!(park_detail(&ParkReason::AutoPause("obs64.exe".into()), 60).contains("obs64.exe"));
        assert_eq!(park_kind(&ParkReason::IdleTimeout), "idle");
        assert_eq!(park_kind(&ParkReason::AutoPause("x".into())), "autopause");
    }

    #[test]
    fn about_links_are_allowlisted() {
        assert!(allowed_external_url("https://vocawin.com"));
        assert!(allowed_external_url(
            "https://github.com/VocaHQ/vocawin/issues/new/choose"
        ));
        assert!(allowed_external_url("https://x.com/vocahq"));
        assert!(allowed_external_url("https://discord.gg/t6muquAJbm"));
        assert!(!allowed_external_url("https://x.com/jatinkrmalik"));
        assert!(!allowed_external_url("https://example.com"));
    }

    #[test]
    fn about_uses_brand_kit_not_simple_icons() {
        let src = include_str!("../../src/main.ts");
        assert!(src.contains("web/assets/brand/vocahq/voca-mark.svg"));
        assert!(src.contains("web/assets/brand/promo/cards/platform/linux.svg"));
        assert!(src.contains("web/assets/brand/promo/cards/platform/apple.svg"));
        assert!(src.contains("web/assets/brand/promo/cards/platform/android.svg"));
        assert!(src.contains("web/assets/brand/vocagateway/vocagateway-1u.svg"));
        assert!(src.contains("web/assets/brand/vocahq/social/discord.svg"));
        assert!(src.contains("web/assets/brand/vocahq/social/github.svg"));
        assert!(src.contains("web/assets/brand/vocahq/social/x.svg"));
        assert!(src.contains("web/assets/brand/vocahq/social/mail.svg"));
        assert!(!src.contains("web/assets/icons/"));
        assert!(!src.contains("./assets/social/"));
        assert!(src.contains("themeBrandSvg(hqMarkRaw)"));
        assert!(src.contains("themeBrandSvg(gatewayMarkRaw, { dropPlate: true })"));
        assert!(src.contains("fill=\"#0[Bb]1[Aa]15\""));
        assert!(src.contains("fill=\"#0[Ff]6[Bb]57\""));
        assert!(src.contains("'fill=\"currentColor\"'"));
        let hq = include_str!("../../web/assets/brand/vocahq/voca-mark.svg");
        let gateway = include_str!("../../web/assets/brand/vocagateway/vocagateway-1u.svg");
        assert!(
            hq.contains("#0B1A15"),
            "HQ mark still has baked ink for About to recolor"
        );
        assert!(
            gateway.contains("#0F6B57") && gateway.contains(r#"width="1024""#),
            "Gateway 1U still has the baked plate for dropPlate"
        );
    }

    #[test]
    fn dictation_page_is_hotkey_first_without_start_cta() {
        let src = include_str!("../../src/main.ts");
        assert!(src.contains("recording && !testListening"));
        assert!(src.contains("Stop &amp; type"));
        assert!(src.contains("dictation-bento"));
        assert!(!src.contains(">Start dictation<"));
        assert!(src.contains("Practice via sidebar Test"));
    }

    #[test]
    fn test_dictation_marks_practice_before_capture_and_never_injects() {
        let src = include_str!("../../src/main.ts");
        let start = src
            .find("async function testDictation()")
            .expect("testDictation");
        let rest = &src[start..];
        let end = rest[1..]
            .find("\nasync function ")
            .map(|idx| start + 1 + idx)
            .unwrap_or(src.len());
        let body = &src[start..end];
        let mark = body
            .find("testListening = true")
            .expect("practice take must set testListening");
        let invoke = body
            .find("start_recording")
            .expect("practice take must start capture");
        assert!(
            mark < invoke,
            "testListening must be set before start_recording so Stop & type stays hidden"
        );
        assert!(!body.contains("inject_text"));
        assert!(body.contains("noInject: true"));

        let toggle_start = src
            .find("async function toggleRecording()")
            .expect("toggleRecording");
        let toggle_rest = &src[toggle_start..];
        let toggle_end = toggle_rest[1..]
            .find("\nasync function ")
            .map(|idx| toggle_start + 1 + idx)
            .unwrap_or(src.len());
        let toggle = &src[toggle_start..toggle_end];
        assert!(
            toggle.contains("if (testListening) return;"),
            "Stop & type must not inject a practice take"
        );
    }

    #[test]
    fn hotkey_repeat_while_live_is_silent() {
        let src = include_str!("lib.rs");
        let needle = format!("{}{}", "Hotkey press ", "ignored");
        assert!(
            !src.contains(&needle),
            "typematic Ignore must not write a log line"
        );
    }

    #[test]
    fn tray_stop_does_not_inject_on_its_own() {
        let src = include_str!("lib.rs");
        let start = src.find("fn tray_stop_voice").expect("tray_stop_voice");
        let rest = &src[start..];
        let end = rest[1..]
            .find("\nfn ")
            .map(|idx| start + 1 + idx)
            .unwrap_or(src.len());
        let body = &src[start..end];
        assert!(body.contains("finish_voice_session"));
        assert!(!body.contains("inject_transcript"));
    }

    #[test]
    fn audio_reply_timeout_returns_a_clear_error() {
        assert!(AUDIO_REPLY_TIMEOUT >= std::time::Duration::from_secs(2));
        let (_tx, rx) = std::sync::mpsc::channel::<()>();
        let error =
            recv_audio_reply(rx, std::time::Duration::from_millis(15), "start").unwrap_err();
        assert!(error.contains("timed out"), "{error}");
        assert!(error.contains("start"), "{error}");
    }

    #[test]
    fn disable_autostart_when_shortcut_is_missing_is_ok() {
        assert!(autostart_disable_error_is_missing(
            "The system cannot find the file specified. (os error 2)"
        ));
        assert!(autostart_disable_error_is_missing(
            std::io::Error::from_raw_os_error(2)
        ));
        assert!(!autostart_disable_error_is_missing(
            "Access is denied. (os error 5)"
        ));
    }

    #[test]
    fn load_falls_back_to_whisper_tiny_when_selected_is_missing() {
        let directory = tempfile::tempdir().unwrap();
        let models = directory.path().join("models");
        fs::create_dir_all(&models).unwrap();
        fs::write(models.join("whisper-tiny.bin"), b"tiny").unwrap();

        let mut settings = Settings {
            selected_model: "gigaam-v3".into(),
            ..Settings::default()
        };
        assert!(fallback_selected_model_if_needed(&mut settings, &models));
        assert_eq!(settings.selected_model, "whisper-tiny");

        let path = directory.path().join("settings.json");
        persist_settings(&path, &settings).unwrap();
        assert_eq!(load_settings(&path).selected_model, "whisper-tiny");
    }

    #[test]
    fn fallback_keeps_installed_whisper_base() {
        let directory = tempfile::tempdir().unwrap();
        let models = directory.path().join("models");
        fs::create_dir_all(&models).unwrap();
        fs::write(models.join("whisper-tiny.bin"), b"tiny").unwrap();
        fs::write(models.join("whisper-base.bin"), b"base").unwrap();
        let mut settings = Settings {
            selected_model: "whisper-base".into(),
            ..Settings::default()
        };
        assert!(!fallback_selected_model_if_needed(&mut settings, &models));
        assert_eq!(settings.selected_model, "whisper-base");
    }

    #[test]
    fn fallback_skips_when_tiny_is_not_installed() {
        let directory = tempfile::tempdir().unwrap();
        let models = directory.path().join("models");
        fs::create_dir_all(&models).unwrap();
        let mut settings = Settings {
            selected_model: "gigaam-v3".into(),
            ..Settings::default()
        };
        assert!(!fallback_selected_model_if_needed(&mut settings, &models));
        assert_eq!(settings.selected_model, "gigaam-v3");
    }

    #[test]
    fn app_state_is_send_and_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<AppState>();
    }

    #[test]
    fn tar_gz_flat_archive_lands_in_catalog_directory() {
        use flate2::write::GzEncoder;
        use flate2::Compression;

        let directory = tempfile::tempdir().unwrap();
        let archive = directory.path().join("flat.tar.gz");
        {
            let file = fs::File::create(&archive).unwrap();
            let encoder = GzEncoder::new(file, Compression::default());
            let mut builder = tar::Builder::new(encoder);
            let mut header = tar::Header::new_gnu();
            header.set_size(4);
            header.set_mode(0o644);
            header.set_cksum();
            builder
                .append_data(&mut header, "model.int8.onnx", &b"onnx"[..])
                .unwrap();
            let mut header = tar::Header::new_gnu();
            header.set_size(5);
            header.set_mode(0o644);
            header.set_cksum();
            builder
                .append_data(&mut header, "vocab.txt", &b"vocab"[..])
                .unwrap();
            builder.into_inner().unwrap().finish().unwrap();
        }
        let destination = directory.path().join("gigaam-v3");
        unpack_tar_gz_into(&archive, &destination).unwrap();
        assert_eq!(
            fs::read_to_string(destination.join("model.int8.onnx")).unwrap(),
            "onnx"
        );
        assert_eq!(
            fs::read_to_string(destination.join("vocab.txt")).unwrap(),
            "vocab"
        );
    }

    #[test]
    fn tar_gz_nested_archive_renames_to_catalog_id() {
        use flate2::write::GzEncoder;
        use flate2::Compression;

        let directory = tempfile::tempdir().unwrap();
        let archive = directory.path().join("nested.tar.gz");
        {
            let file = fs::File::create(&archive).unwrap();
            let encoder = GzEncoder::new(file, Compression::default());
            let mut builder = tar::Builder::new(encoder);
            let mut header = tar::Header::new_gnu();
            header.set_size(3);
            header.set_mode(0o644);
            header.set_cksum();
            builder
                .append_data(
                    &mut header,
                    "moonshine-base/encoder_model.onnx",
                    &b"enc"[..],
                )
                .unwrap();
            let mut header = tar::Header::new_gnu();
            header.set_size(3);
            header.set_mode(0o644);
            header.set_cksum();
            builder
                .append_data(
                    &mut header,
                    "moonshine-base/decoder_model_merged.onnx",
                    &b"dec"[..],
                )
                .unwrap();
            let mut header = tar::Header::new_gnu();
            header.set_size(4);
            header.set_mode(0o644);
            header.set_cksum();
            builder
                .append_data(&mut header, "moonshine-base/tokenizer.json", &b"toks"[..])
                .unwrap();
            builder.into_inner().unwrap().finish().unwrap();
        }
        let destination = directory.path().join("moonshine-base");
        unpack_tar_gz_into(&archive, &destination).unwrap();
        assert!(destination.join("encoder_model.onnx").is_file());
        assert!(destination.join("tokenizer.json").is_file());
        assert!(!destination.join("moonshine-base").exists());
    }

    #[test]
    fn stale_recording_flag_does_not_block_mic_test() {
        assert_eq!(mic_test_gate(true, false), MicTestGate::ClearStaleThenAllow);
        assert_eq!(mic_test_gate(false, false), MicTestGate::Allow);
        assert_eq!(mic_test_gate(true, true), MicTestGate::RefuseLiveSession);
        assert!(!session_is_live(true, false));
    }

    #[test]
    fn failed_start_leaves_recording_false() {
        assert!(!recording_after_start_attempt(false));
        assert!(recording_after_start_attempt(true));
    }

    #[test]
    fn leftover_test_dictation_session_clears_the_flag() {
        assert!(!recording_after_stop_attempt());
    }

    #[test]
    fn auto_stop_take_error_clears_recording() {
        assert!(!recording_after_stop_attempt());
        assert!(is_stale_stop_error("No microphone audio was captured"));
        assert!(is_stale_stop_error("No recording is in progress"));
    }

    #[test]
    fn ptt_press_while_recording_is_noop() {
        assert_eq!(ptt_pressed_action(true, false), PttPressedAction::Ignore);
        assert_eq!(ptt_pressed_action(false, true), PttPressedAction::Ignore);
        assert_eq!(ptt_pressed_action(false, false), PttPressedAction::Start);
    }

    #[test]
    fn stop_without_stream_leaves_recording_false() {
        assert!(!recording_after_stop_attempt());
        assert!(is_stale_stop_error("No recording is in progress"));
        assert!(is_stale_stop_error("No microphone audio was captured"));
        assert_eq!(mic_test_gate(true, false), MicTestGate::ClearStaleThenAllow);
    }

    #[test]
    fn start_while_meter_running_does_not_clobber_meter_only() {
        assert!(meter_only_after_start(true, false, true));
        assert!(!meter_only_after_start(false, false, true));
        assert!(meter_only_after_start(false, true, false));
    }

    #[test]
    fn safety_timeout_is_just_past_max_recording() {
        assert_eq!(
            safety_timeout_for(60.0),
            std::time::Duration::from_secs_f32(65.0)
        );
    }

    #[test]
    fn a_press_starts_when_idle() {
        for trigger in [Trigger::Hold, Trigger::Toggle, Trigger::HandsFree, Trigger::Mouse] {
            assert_eq!(press_decision(false, false, trigger, Trigger::Hold), PressDecision::Start);
        }
    }

    #[test]
    fn toggle_mode_second_tap_stops_but_hold_repeats_do_not() {
        // Push-to-talk: typematic repeats of the held key change nothing.
        assert_eq!(press_decision(true, false, Trigger::Hold, Trigger::Hold), PressDecision::Ignore);
        assert_eq!(press_decision(true, false, Trigger::Mouse, Trigger::Mouse), PressDecision::Ignore);
        // Toggle: a second tap of the hotkey or mouse button ends the take.
        assert_eq!(press_decision(true, true, Trigger::Toggle, Trigger::Toggle), PressDecision::Stop);
        assert_eq!(press_decision(true, true, Trigger::Mouse, Trigger::Toggle), PressDecision::Stop);
        // ...but never a hands-free take, which has its own shortcut.
        assert_eq!(press_decision(true, true, Trigger::Toggle, Trigger::HandsFree), PressDecision::Ignore);
        // The hands-free shortcut stops whatever is running.
        assert_eq!(press_decision(true, false, Trigger::HandsFree, Trigger::Hold), PressDecision::Stop);
    }

    #[test]
    fn a_release_only_ends_the_hold_that_started_the_take() {
        assert!(release_ends_session(false, Trigger::Hold, Trigger::Hold));
        assert!(release_ends_session(false, Trigger::Mouse, Trigger::Mouse));
        assert!(!release_ends_session(false, Trigger::Hold, Trigger::HandsFree));
        assert!(!release_ends_session(false, Trigger::Mouse, Trigger::Hold));
        assert!(!release_ends_session(true, Trigger::Toggle, Trigger::Toggle));
    }

    #[test]
    fn extra_shortcuts_reject_modifiers_and_duplicates() {
        assert_eq!(validate_extra_shortcut("", "Hands-free", &["AltRight"]).unwrap(), "");
        assert_eq!(validate_extra_shortcut("f9", "Hands-free", &["AltRight"]).unwrap(), "F9");
        assert_eq!(
            validate_extra_shortcut("ctrl+alt+v", "Paste", &["AltRight"]).unwrap(),
            "Ctrl+Alt+V"
        );
        assert!(validate_extra_shortcut("ControlRight", "Hands-free", &["AltRight"])
            .unwrap_err()
            .contains("lone modifier"));
        assert!(validate_extra_shortcut("F9", "Paste", &["AltRight", "F9"])
            .unwrap_err()
            .contains("already used"));
    }

    #[test]
    fn settings_from_an_older_version_get_parity_defaults() {
        let old = r#"{"hotkey":"AltRight","activationMode":"pushToTalk","language":"English",
            "silenceSeconds":1.5,"launchAtLogin":false,"selectedModel":"whisper-tiny"}"#;
        let settings: Settings = serde_json::from_str(old).unwrap();
        assert_eq!(settings.cleanup_level, "medium");
        assert!(settings.escape_cancels);
        assert!(settings.skip_silence);
        assert!(settings.ready_pill);
        assert_eq!(settings.overlay_style, "minimal");
        assert_eq!(settings.history_retention_days, 30);
        assert!(!settings.numbers_as_digits && !settings.spoken_emoji && !settings.mute_other_audio);
        assert!(settings.hands_free_hotkey.is_empty() && settings.paste_last_hotkey.is_empty());
        assert_eq!(settings.insertion_mode, "type");
        assert!(inject_options(&settings).paste_apps.is_empty());
        // A file missing a field once required still loads the rest.
        let partial: Settings = serde_json::from_str(r#"{"language":"German"}"#).unwrap();
        assert_eq!(partial.language, "German");
        assert_eq!(partial.hotkey, "AltRight");
    }

    #[test]
    fn extra_settings_are_checked_and_tidied() {
        let mut settings = Settings {
            replacements: vec![
                dictionary::Replacement {
                    heard: "  get hub ".into(),
                    replacement: " GitHub ".into(),
                },
                dictionary::Replacement {
                    heard: " ".into(),
                    replacement: "nothing".into(),
                },
            ],
            snippets: vec![dictionary::Snippet {
                trigger: "".into(),
                expansion: "dropped".into(),
            }],
            hands_free_hotkey: "f8".into(),
            ..Settings::default()
        };
        normalize_extra_settings(&mut settings).unwrap();
        assert_eq!(settings.replacements.len(), 1);
        assert_eq!(settings.replacements[0].heard, "get hub");
        assert_eq!(settings.replacements[0].replacement, "GitHub");
        assert!(settings.snippets.is_empty());
        assert_eq!(settings.hands_free_hotkey, "F8");

        let mut bad = Settings {
            history_retention_days: 3,
            ..Settings::default()
        };
        assert!(normalize_extra_settings(&mut bad).is_err());
        let mut bad = Settings {
            mouse_button: "left".into(),
            ..Settings::default()
        };
        assert!(normalize_extra_settings(&mut bad).is_err());
    }

    #[test]
    fn english_only_models_let_the_text_rules_assume_english() {
        let mut settings = Settings {
            language: "Auto-detect".into(),
            selected_model: "moonshine-base".into(),
            ..Settings::default()
        };
        assert_eq!(text_language(&settings), Some("en"));
        settings.selected_model = "whisper-base".into();
        assert_eq!(text_language(&settings), None);
        settings.language = "German".into();
        assert_eq!(text_language(&settings), Some("de"));
    }

    #[test]
    fn transcripts_get_the_dictionary_and_spoken_forms() {
        let settings = Settings {
            numbers_as_digits: true,
            spoken_emoji: true,
            replacements: vec![dictionary::Replacement {
                heard: "get hub".into(),
                replacement: "GitHub".into(),
            }],
            ..Settings::default()
        };
        assert_eq!(
            format_transcript(&settings, "um push twenty three commits to get hub, party emoji"),
            "Push 23 commits to GitHub, 🎉 "
        );
    }

    #[test]
    fn cancellations_of_overlapping_takes_are_all_kept() {
        let mut cancellations = Cancellations::default();
        assert!(cancellations.cancel(4));
        assert!(cancellations.cancel(5));
        assert!(cancellations.cancel(5));
        assert!(cancellations.settle(4));
        assert!(cancellations.settle(5));
        assert!(!cancellations.settle(6));
        for session in 7..40 {
            cancellations.cancel(session);
        }
        assert_eq!(cancellations.cancelled.len(), 16);
    }

    #[test]
    fn escape_after_a_take_commits_is_refused_not_half_applied() {
        let mut cancellations = Cancellations::default();
        // Completion decided first: the text types, and Escape says so.
        assert!(!cancellations.settle(8));
        assert!(!cancellations.cancel(8));
        // Escape first: completion sees it and does not type.
        assert!(cancellations.cancel(9));
        assert!(cancellations.settle(9));
    }

    #[test]
    fn escape_only_cancels_the_take_it_was_pressed_during() {
        assert!(is_current_session(4, 4));
        // Take 5 started before Escape for take 4 was handled: leave it.
        assert!(!is_current_session(5, 4));
        assert!(!is_current_session(0, 0));
    }

    #[test]
    fn the_ready_pill_names_the_key_plainly() {
        assert_eq!(hotkey_display("AltRight"), "Right Alt");
        assert_eq!(hotkey_display("Ctrl+Shift+Space"), "Ctrl+Shift+Space");
        assert_eq!(hotkey_display("Ctrl+Alt+K"), "Ctrl+Alt+K");
    }
}
