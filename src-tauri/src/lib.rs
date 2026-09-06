//! VocaWin's platform shell. Recognition engines are deliberately behind a small
//! catalog/adapter boundary so model downloads never require a cloud account.

mod autopause;
mod devices;
mod gpu;
mod hardware;
mod hook;
mod hotkey;
mod logbuf;
mod machine;
mod output;
mod power;
mod sounds;
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
    sync::atomic::{AtomicBool, Ordering},
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
/// the Windows target enables `whisper-rs/vulkan`.
fn whisper_acceleration() -> &'static str {
    if cfg!(vocawin_whisper_vulkan) {
        "CPU · Vulkan"
    } else {
        "CPU"
    }
}

fn onnx_acceleration(directml: bool) -> &'static str {
    if directml && cfg!(windows) {
        "CPU · DirectML"
    } else {
        "CPU"
    }
}

fn gpu_backends_summary() -> Vec<&'static str> {
    let mut backends = Vec::new();
    if cfg!(vocawin_whisper_vulkan) {
        backends.push("Vulkan (whisper.cpp)");
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
            id: "parakeet-tdt-0.6b-v3",
            name: "Parakeet TDT 0.6B v3",
            engine: "ONNX Runtime",
            size: "478 MB",
            languages: "25 languages",
            acceleration: onnx_acceleration(true),
            description: "High-speed multilingual dictation.",
        },
        Model {
            id: "moonshine-tiny",
            name: "Moonshine Tiny",
            engine: "ONNX Runtime",
            size: "145 MB",
            languages: "English",
            acceleration: onnx_acceleration(false),
            description: "Low-memory, quick English notes.",
        },
        Model {
            id: "moonshine-base",
            name: "Moonshine Base",
            engine: "ONNX Runtime",
            size: "190 MB",
            languages: "English",
            acceleration: onnx_acceleration(false),
            description: "Compact English model.",
        },
        Model {
            id: "sensevoice-small",
            name: "SenseVoice Small",
            engine: "ONNX Runtime",
            size: "240 MB",
            languages: "Chinese · Japanese · Korean · Cantonese · English",
            acceleration: onnx_acceleration(true),
            description: "East Asian language specialist.",
        },
        Model {
            id: "gigaam-v3",
            name: "GigaAM v3",
            engine: "ONNX Runtime",
            size: "225 MB",
            languages: "Russian",
            acceleration: onnx_acceleration(false),
            description: "Russian recognition with punctuation.",
        },
        Model {
            id: "canary-180m",
            name: "Canary 180M Flash",
            engine: "ONNX Runtime",
            size: "150 MB",
            languages: "English · Spanish · German · French",
            acceleration: onnx_acceleration(true),
            description: "Fast four-language transcription.",
        },
    ]
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
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
}

fn default_true() -> bool {
    true
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
        }
    }
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
    samples: &Arc<Mutex<Vec<f32>>>,
    last_voice_ms: &Arc<Mutex<u128>>,
    heard_speech: &Arc<Mutex<bool>>,
    peak_level: &Arc<Mutex<f32>>,
    store_samples: bool,
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
    if store_samples {
        samples.lock().unwrap().push(mono);
    }
}

#[cfg(windows)]
fn open_input_stream(
    samples: Arc<Mutex<Vec<f32>>>,
    last_voice_ms: Arc<Mutex<u128>>,
    heard_speech: Arc<Mutex<bool>>,
    peak_level: Arc<Mutex<f32>>,
    store_samples: bool,
    device_name: &str,
) -> Result<(cpal::Stream, u32), String> {
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
            device.build_input_stream(
                &config,
                move |data: &[f32], _| {
                    for frame in data.chunks(channels) {
                        let mono = frame.iter().sum::<f32>() / frame.len() as f32;
                        note_audio_sample(
                            mono,
                            &samples,
                            &last_voice_ms,
                            &heard_speech,
                            &peak_level,
                            store_samples,
                        );
                    }
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
            device.build_input_stream(
                &config,
                move |data: &[i16], _| {
                    for frame in data.chunks(channels) {
                        let mono = frame
                            .iter()
                            .map(|sample| *sample as f32 / i16::MAX as f32)
                            .sum::<f32>()
                            / frame.len() as f32;
                        note_audio_sample(
                            mono,
                            &samples,
                            &last_voice_ms,
                            &heard_speech,
                            &peak_level,
                            store_samples,
                        );
                    }
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
            device.build_input_stream(
                &config,
                move |data: &[u16], _| {
                    for frame in data.chunks(channels) {
                        let mono = frame
                            .iter()
                            .map(|sample| (*sample as f32 / u16::MAX as f32) * 2.0 - 1.0)
                            .sum::<f32>()
                            / frame.len() as f32;
                        note_audio_sample(
                            mono,
                            &samples,
                            &last_voice_ms,
                            &heard_speech,
                            &peak_level,
                            store_samples,
                        );
                    }
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
    Ok((stream, sample_rate))
}

#[cfg(windows)]
fn now_ms() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0)
}

#[cfg(windows)]
fn audio_thread_main(commands: mpsc::Receiver<AudioCommand>, app: AppHandle) {
    let samples = Arc::new(Mutex::new(Vec::<f32>::new()));
    let last_voice_ms = Arc::new(Mutex::new(now_ms()));
    let heard_speech = Arc::new(Mutex::new(false));
    let peak_level = Arc::new(Mutex::new(0.0_f32));
    let mut stream: Option<cpal::Stream> = None;
    let mut sample_rate: Option<u32> = None;
    let mut started_ms: Option<u128> = None;
    let mut silence_seconds = 1.5_f32;
    let mut max_seconds = 60.0_f32;
    let mut meter_only = false;
    let mut silence_auto_stop = false;

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
                        reply,
                    } => {
                        meter_only = meter_only_after_start(stream.is_some(), meter, meter_only);
                        let result = if stream.is_some() {
                            Err("A recording is already in progress".into())
                        } else {
                            silence_seconds = silence.clamp(0.3, 10.0);
                            max_seconds = max.clamp(3.0, 300.0);
                            silence_auto_stop = enable_silence;
                            match open_input_stream(
                                Arc::clone(&samples),
                                Arc::clone(&last_voice_ms),
                                Arc::clone(&heard_speech),
                                Arc::clone(&peak_level),
                                !meter,
                                &device_name,
                            ) {
                                Ok((next_stream, rate)) => {
                                    sample_rate = Some(rate);
                                    stream = Some(next_stream);
                                    started_ms = Some(now_ms());
                                    *last_voice_ms.lock().unwrap() = now_ms();
                                    *heard_speech.lock().unwrap() = false;
                                    logbuf::debug("Microphone opened.");
                                    Ok(())
                                }
                                Err(error) => {
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
                            stream.take();
                            sample_rate.take();
                            started_ms = None;
                            if let Ok(mut buffer) = samples.lock() {
                                buffer.clear();
                            }
                            Ok((Vec::new(), 16_000))
                        } else {
                            take_recording(&mut stream, &mut sample_rate, &mut started_ms, &samples)
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
                    AudioCommand::IsLive { reply } => {
                        let _ = reply.send(stream.is_some() && !meter_only);
                    }
                }
                false
            }
            Err(mpsc::RecvTimeoutError::Timeout) => true,
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        };

        if timed_out && stream.is_some() && !meter_only {
            let started = started_ms.unwrap_or_else(now_ms);
            let elapsed = (now_ms().saturating_sub(started)) as f32 / 1000.0;
            let last_voice = *last_voice_ms.lock().unwrap_or_else(|e| e.into_inner());
            let quiet_for = (now_ms().saturating_sub(last_voice)) as f32 / 1000.0;
            let heard = *heard_speech.lock().unwrap_or_else(|e| e.into_inner());
            let silence_hit = silence_auto_stop && heard && quiet_for >= silence_seconds;
            let max_hit = elapsed >= max_seconds;
            if silence_hit || max_hit {
                match take_recording(&mut stream, &mut sample_rate, &mut started_ms, &samples) {
                    Ok((pcm, rate)) => {
                        let app_for_finish = app.clone();
                        std::thread::spawn(move || {
                            finish_captured_audio(&app_for_finish, pcm, rate);
                        });
                    }
                    Err(_) => {
                        // take_recording already dropped the stream.
                        clear_recording_after_capture_drop(&app);
                    }
                }
            }
        }
    }
}

#[cfg(windows)]
fn take_recording(
    stream: &mut Option<cpal::Stream>,
    sample_rate: &mut Option<u32>,
    started_ms: &mut Option<u128>,
    samples: &Arc<Mutex<Vec<f32>>>,
) -> Result<(Vec<f32>, u32), String> {
    if stream.take().is_none() {
        return Err("No recording is in progress".into());
    }
    *started_ms = None;
    let rate = sample_rate.take().unwrap_or(16_000);
    match samples.lock() {
        Ok(mut buffer) => {
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
    ) -> Result<(), String> {
        let (reply, response) = mpsc::channel();
        self.commands
            .send(AudioCommand::Start {
                silence_seconds,
                max_seconds,
                device_name,
                silence_auto_stop,
                meter_only: false,
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

    fn capture_live(&self) -> bool {
        let (reply, response) = mpsc::channel();
        if self.commands.send(AudioCommand::IsLive { reply }).is_err() {
            return false;
        }
        response
            .recv_timeout(std::time::Duration::from_millis(200))
            .unwrap_or(false)
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
    fn start(&self, _: f32, _: f32, _: String, _: bool) -> Result<(), String> {
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

#[cfg(windows)]
fn finish_captured_audio(app: &AppHandle, samples: Vec<f32>, sample_rate: u32) {
    let state = app.state::<AppState>();
    {
        let mut flag = state.recording.lock().unwrap_or_else(|e| e.into_inner());
        *flag = false;
    }
    let _ = app.emit("recording-changed", false);
    set_tray_phase(app, TrayPhase::Processing);
    let sound = state
        .settings
        .lock()
        .map(|settings| settings.sound_theme.clone())
        .unwrap_or_else(|_| "voca".into());
    sounds::play_if_enabled(&sound, false);
    match transcribe_samples(&state, samples, sample_rate) {
        Ok(text) if !text.is_empty() => {
            let inject = *state
                .inject_on_auto_stop
                .lock()
                .unwrap_or_else(|e| e.into_inner());
            if inject {
                let _ = inject_transcript(&*state, &text);
                let _ = app.emit("dictation-finished", text);
            } else {
                let _ = app.emit("test-dictation-finished", text);
            }
        }
        Ok(_) => {
            let inject = *state
                .inject_on_auto_stop
                .lock()
                .unwrap_or_else(|e| e.into_inner());
            if inject {
                let _ = app.emit("dictation-finished", String::new());
            } else {
                let _ = app.emit("test-dictation-finished", String::new());
            }
        }
        Err(error) => {
            sounds::play_error_if_enabled(&sound);
            logbuf::error_and_emit(app, format!("Dictation error: {error}"));
            let _ = app.emit("dictation-error", error);
        }
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

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct HistoryEntry {
    id: u128,
    text: String,
    model_id: String,
    created_at_ms: u128,
}

struct AppState {
    settings: Mutex<Settings>,
    settings_path: PathBuf,
    history_path: PathBuf,
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

fn load_history(path: &Path) -> Vec<HistoryEntry> {
    fs::read_to_string(path)
        .ok()
        .and_then(|contents| serde_json::from_str(&contents).ok())
        .unwrap_or_default()
}

fn append_history(path: &Path, text: String, model_id: String) -> Result<(), String> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|error| format!("Could not timestamp transcription: {error}"))?
        .as_millis();
    let mut entries = load_history(path);
    entries.insert(
        0,
        HistoryEntry {
            id: now,
            text,
            model_id,
            created_at_ms: now,
        },
    );
    entries.truncate(100);
    let parent = path
        .parent()
        .ok_or("History path has no parent directory")?;
    fs::create_dir_all(parent)
        .map_err(|error| format!("Could not create history directory: {error}"))?;
    fs::write(
        path,
        serde_json::to_vec_pretty(&entries)
            .map_err(|error| format!("Could not save history: {error}"))?,
    )
    .map_err(|error| format!("Could not save history: {error}"))
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
    // Windows does not replace an existing file during rename, so remove the
    // previous version only after the complete temporary file was written.
    if path.exists() {
        fs::remove_file(path).map_err(|error| format!("Could not replace settings: {error}"))?;
    }
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
    persist_settings(&state.settings_path, &settings)?;
    // Disk is the source of truth after persist. Refresh AppState before
    // autostart/hotkey so a later side-effect error cannot leave start_recording
    // and the UI on different selected models.
    *state
        .settings
        .lock()
        .map_err(|_| "Settings lock was poisoned")? = settings.clone();
    apply_launch_at_login(&app, settings.launch_at_login)?;
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
    Ok(())
}

/// auto-launch's Windows disable() always calls RegDeleteValue. A machine that
/// never enabled launch-at-login has no Run value, and Windows returns
/// ERROR_FILE_NOT_FOUND ("The system cannot find the file specified. (os error 2)").
/// That is already-disabled, not a failed settings save.
fn autostart_disable_error_is_missing(error: impl std::fmt::Display) -> bool {
    let text = error.to_string().to_ascii_lowercase();
    text.contains("os error 2")
        || text.contains("cannot find the file specified")
        || text.contains("no such file or directory")
}

fn apply_launch_at_login(app: &AppHandle, enabled: bool) -> Result<(), String> {
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
fn get_history(state: State<'_, AppState>) -> Vec<HistoryEntry> {
    load_history(&state.history_path)
}

#[tauri::command]
fn clear_history(state: State<'_, AppState>) -> Result<(), String> {
    if state.history_path.exists() {
        fs::remove_file(&state.history_path)
            .map_err(|error| format!("Could not clear history: {error}"))?;
    }
    Ok(())
}

#[tauri::command]
fn get_models() -> Vec<Model> {
    model_catalog()
}

fn model_path(models_path: &Path, id: &str) -> PathBuf {
    if id.starts_with("whisper-") || id.starts_with("distil-whisper-") {
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
        id if id.starts_with("whisper-") || id.starts_with("distil-whisper-") => path.is_file(),
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

fn resample_to_16khz(samples: &[f32], source_rate: u32) -> Vec<f32> {
    if source_rate == 16_000 {
        return samples.to_vec();
    }
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

fn transcribe_onnx(
    model_id: &str,
    models_path: &std::path::Path,
    pcm: &[f32],
    language: Option<&str>,
) -> Result<String, String> {
    use transcribe_rs::onnx::{
        canary::{CanaryModel, CanaryParams},
        gigaam::GigaAMModel,
        moonshine::{MoonshineModel, MoonshineVariant},
        parakeet::{ParakeetModel, ParakeetParams},
        sense_voice::{SenseVoiceModel, SenseVoiceParams},
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
    let result = match model_id {
        "parakeet-tdt-0.6b-v3" => {
            let mut model = ParakeetModel::load(&model_path, &Quantization::Int8)
                .map_err(|error| format!("Could not load Parakeet: {error}"))?;
            model
                .transcribe_with(pcm, &ParakeetParams::default())
                .map_err(|error| format!("Parakeet transcription failed: {error}"))?
        }
        "moonshine-tiny" | "moonshine-base" => {
            let variant = if model_id == "moonshine-tiny" {
                MoonshineVariant::Tiny
            } else {
                MoonshineVariant::Base
            };
            let mut model = MoonshineModel::load(&model_path, variant, &Quantization::default())
                .map_err(|error| format!("Could not load Moonshine: {error}"))?;
            // Moonshine's public adapter takes a WAV path. The PCM is already
            // mono/16 kHz, so writing this short temporary file is lossless.
            let wav_path = models_path.join(".vocawin-recording.wav");
            write_pcm_wav(&wav_path, pcm)?;
            let output = model
                .transcribe_file(&wav_path, &transcribe_rs::TranscribeOptions::default())
                .map_err(|error| format!("Moonshine transcription failed: {error}"));
            let _ = fs::remove_file(wav_path);
            output?
        }
        "sensevoice-small" => {
            let mut model = SenseVoiceModel::load(&model_path, &Quantization::Int8)
                .map_err(|error| format!("Could not load SenseVoice: {error}"))?;
            model
                .transcribe_with(
                    pcm,
                    &SenseVoiceParams {
                        language: language.map(str::to_owned),
                        ..Default::default()
                    },
                )
                .map_err(|error| format!("SenseVoice transcription failed: {error}"))?
        }
        "gigaam-v3" => {
            let mut model = GigaAMModel::load(&model_path, &Quantization::Int8)
                .map_err(|error| format!("Could not load GigaAM: {error}"))?;
            let wav_path = models_path.join(".vocawin-recording.wav");
            write_pcm_wav(&wav_path, pcm)?;
            let output = model
                .transcribe_file(&wav_path, &transcribe_rs::TranscribeOptions::default())
                .map_err(|error| format!("GigaAM transcription failed: {error}"));
            let _ = fs::remove_file(wav_path);
            output?
        }
        "canary-180m" => {
            let mut model = CanaryModel::load(&model_path, &Quantization::Int8)
                .map_err(|error| format!("Could not load Canary: {error}"))?;
            model
                .transcribe_with(pcm, &CanaryParams::default())
                .map_err(|error| format!("Canary transcription failed: {error}"))?
        }
        _ => return Err(format!("The {} adapter is not available yet.", model_id)),
    };
    Ok(result.text.trim().to_string())
}

fn write_pcm_wav(path: &std::path::Path, pcm: &[f32]) -> Result<(), String> {
    let specification = hound::WavSpec {
        channels: 1,
        sample_rate: 16_000,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut writer = hound::WavWriter::create(path, specification)
        .map_err(|error| format!("Could not prepare audio for transcription: {error}"))?;
    for sample in pcm {
        writer
            .write_sample((sample.clamp(-1.0, 1.0) * i16::MAX as f32) as i16)
            .map_err(|error| format!("Could not write audio for transcription: {error}"))?;
    }
    writer
        .finalize()
        .map_err(|error| format!("Could not finalize audio for transcription: {error}"))
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
    apply_ready_or_parked_tray(app);
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
/// auto-stop will not type into the front app.
fn begin_voice_session(app: &AppHandle, inject: bool) -> Result<(), String> {
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
    state.session_opening.store(true, Ordering::SeqCst);
    state.release_during_open.store(false, Ordering::SeqCst);
    if let Err(error) = state.recorder.start(
        settings.silence_seconds,
        settings.max_recording_seconds,
        settings.input_device.clone(),
        settings.activation_mode == "toggle",
    ) {
        state.session_opening.store(false, Ordering::SeqCst);
        set_recording_flag(&state, recording_after_start_attempt(false));
        let _ = app.emit("recording-changed", false);
        logbuf::error_and_emit(app, format!("Start dictation failed: {error}"));
        return Err(error);
    }
    set_recording_flag(&state, recording_after_start_attempt(true));
    state.session_opening.store(false, Ordering::SeqCst);
    if state.release_during_open.swap(false, Ordering::SeqCst) {
        finish_voice_session(app);
        return Ok(());
    }
    sounds::play_if_enabled(&settings.sound_theme, true);
    let _ = app.emit("recording-changed", true);
    set_tray_phase(app, TrayPhase::Listening);
    Ok(())
}

/// Begins microphone capture. The UI should call `stop_and_transcribe` after
/// push-to-talk is released (or when toggle mode is stopped). Runs off the
/// WebView IPC thread so WASAPI open cannot freeze the window.
#[tauri::command]
async fn start_recording(app: AppHandle, no_inject: Option<bool>) -> Result<(), String> {
    let inject = !no_inject.unwrap_or(false);
    tauri::async_runtime::spawn_blocking(move || begin_voice_session(&app, inject))
        .await
        .map_err(|error| format!("Start dictation was cancelled: {error}"))?
}

fn transcribe_samples(
    state: &AppState,
    samples: Vec<f32>,
    sample_rate: u32,
) -> Result<String, String> {
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
    let language_code = match settings.language.as_str() {
        "English" => Some("en"),
        "Spanish" => Some("es"),
        "French" => Some("fr"),
        "German" => Some("de"),
        "Italian" => Some("it"),
        "Portuguese" => Some("pt"),
        "Dutch" => Some("nl"),
        "Russian" => Some("ru"),
        "Japanese" => Some("ja"),
        "Chinese" => Some("zh"),
        "Korean" => Some("ko"),
        "Arabic" => Some("ar"),
        "Hindi" => Some("hi"),
        "Turkish" => Some("tr"),
        "Polish" => Some("pl"),
        "Ukrainian" => Some("uk"),
        "Swedish" => Some("sv"),
        "Norwegian" => Some("no"),
        "Danish" => Some("da"),
        "Finnish" => Some("fi"),
        "Czech" => Some("cs"),
        "Greek" => Some("el"),
        "Hebrew" => Some("he"),
        "Indonesian" => Some("id"),
        "Vietnamese" => Some("vi"),
        "Thai" => Some("th"),
        "Romanian" => Some("ro"),
        "Hungarian" => Some("hu"),
        "Catalan" => Some("ca"),
        _ => None,
    };
    let raw = if !settings.selected_model.starts_with("whisper-")
        && !settings.selected_model.starts_with("distil-whisper-")
    {
        transcribe_onnx(
            &settings.selected_model,
            &state.models_path,
            &pcm,
            language_code,
        )?
    } else {
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
        state.whisper_cache.transcribe(
            model_path,
            pcm,
            language_code.map(str::to_string),
            use_gpu,
            gpu.device_index,
            true,
            vocabulary::whisper_prompt(&settings.custom_vocabulary),
        )?
    };
    let text = output::apply_output_polish(
        &raw,
        settings.auto_capitalize,
        settings.append_trailing_space,
    );
    if !text.trim().is_empty() && settings.history_enabled {
        append_history(
            &state.history_path,
            text.trim().to_string(),
            settings.selected_model.clone(),
        )?;
    }
    logbuf::info(format!(
        "Transcribed {} chars with {}",
        text.chars().count(),
        settings.selected_model
    ));
    Ok(text)
}

/// Stop capture and clear the session flag even when stop() fails.
/// Transcribe only when there is real PCM.
fn end_voice_session(state: &AppState) -> Result<Option<String>, String> {
    let stopped = state.recorder.stop();
    set_recording_flag(state, recording_after_stop_attempt());
    state.session_opening.store(false, Ordering::SeqCst);
    state.release_during_open.store(false, Ordering::SeqCst);
    match stopped {
        Ok((samples, sample_rate)) if !samples.is_empty() => {
            transcribe_samples(state, samples, sample_rate).map(Some)
        }
        Ok(_) => Ok(None),
        Err(error) if is_stale_stop_error(&error) => Ok(None),
        Err(error) => Err(error),
    }
}

fn abandon_voice_session(state: &AppState) {
    let _ = state.recorder.stop();
    set_recording_flag(state, recording_after_stop_attempt());
    state.session_opening.store(false, Ordering::SeqCst);
    state.release_during_open.store(false, Ordering::SeqCst);
}

fn finish_voice_session(handle: &AppHandle) {
    let state = handle.state::<AppState>();
    let settings = state.settings.lock().map(|s| s.clone()).unwrap_or_default();
    set_tray_phase(handle, TrayPhase::Processing);
    let inject = *state
        .inject_on_auto_stop
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    match end_voice_session(&state) {
        Ok(Some(text)) if !text.is_empty() => {
            sounds::play_if_enabled(&settings.sound_theme, false);
            if inject {
                let _ = inject_transcript(&*state, &text);
                let _ = handle.emit("dictation-finished", text);
            } else {
                let _ = handle.emit("test-dictation-finished", text);
            }
        }
        Ok(_) => {}
        Err(error) => {
            sounds::play_error_if_enabled(&settings.sound_theme);
            logbuf::error_and_emit(handle, format!("Dictation error: {error}"));
            let _ = handle.emit("dictation-error", error);
        }
    }
    let _ = handle.emit("recording-changed", false);
    apply_ready_or_parked_tray(handle);
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
        set_tray_phase(&app, TrayPhase::Processing);
        let text = match end_voice_session(&state) {
            Ok(Some(text)) => text,
            Ok(None) => String::new(),
            Err(error) => {
                let _ = app.emit("recording-changed", false);
                apply_ready_or_parked_tray(&app);
                sounds::play_error_if_enabled(&sound);
                return Err(error);
            }
        };
        sounds::play_if_enabled(&sound, false);
        let _ = app.emit("recording-changed", false);
        apply_ready_or_parked_tray(&app);
        Ok(text)
    })
    .await
    .map_err(|error| format!("Stop dictation was cancelled: {error}"))?
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
    let model_loaded = state.whisper_cache.is_loaded();
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
    let copy_to_clipboard = state
        .settings
        .lock()
        .map(|settings| settings.copy_to_clipboard)
        .unwrap_or(false);
    logbuf::debug(format!(
        "Inject {} chars (copy_to_clipboard={copy_to_clipboard})",
        text.chars().count()
    ));
    match output::inject(text, output::InjectOptions { copy_to_clipboard }) {
        Ok(()) => Ok(()),
        Err(error) => {
            logbuf::error(format!("Inject failed: {error}"));
            Err(error)
        }
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
    inject_transcript(&*state, &text)
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
            let history_path = app_data.join("history.json");
            let models_path = app_data.join("models");
            fs::create_dir_all(&models_path)?;
            let mut settings = load_settings(&settings_path);
            if fallback_selected_model_if_needed(&mut settings, &models_path) {
                if let Err(error) = persist_settings(&settings_path, &settings) {
                    eprintln!("VocaWin could not persist selected model fallback: {error}");
                }
            }
            let handle = app.handle().clone();
            let whisper_cache = whisper_cache::WhisperCache::new();
            whisper_cache
                .configure_idle(settings.idle_unload_enabled, settings.idle_unload_seconds);
            logbuf::set_debug_enabled(settings.debug_logging);
            app.manage(AppState {
                settings: Mutex::new(settings.clone()),
                settings_path,
                history_path,
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
            });
            let _ = apply_launch_at_login(&handle, settings.launch_at_login);
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
            list_input_devices,
            recommend_model,
            get_runtime_status,
            list_running_apps,
            start_recording,
            stop_and_transcribe,
            preview_sound,
            inject_text,
            copy_text,
            open_external
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
        }
    }
    Ok(())
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

#[cfg_attr(not(windows), allow(dead_code))]
pub(crate) fn on_hotkey_event(handle: &AppHandle, event: hook::HookEvent) {
    let state = handle.state::<AppState>();
    if *state
        .dictation_paused
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        && event == hook::HookEvent::Pressed
    {
        return;
    }
    let settings = state.settings.lock().map(|s| s.clone()).unwrap_or_default();
    let toggle = settings.activation_mode == "toggle";
    match event {
        hook::HookEvent::Pressed => {
            let flag = *state.recording.lock().unwrap_or_else(|e| e.into_inner());
            let live = state.recorder.capture_live();
            match ptt_pressed_action(flag, live) {
                PttPressedAction::Ignore => {}
                PttPressedAction::Start => {
                    logbuf::debug_and_emit(handle, "Hotkey pressed.");
                    if let Err(error) = begin_voice_session(handle, true) {
                        if error.contains("No speech model is installed") {
                            sounds::play_error_if_enabled(&settings.sound_theme);
                            logbuf::error_and_emit(handle, error.clone());
                            let _ = handle.emit("dictation-error", error);
                        }
                    }
                }
            }
        }
        hook::HookEvent::Released => {
            if toggle {
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
                abandon_voice_session(&state);
                let _ = app.emit("recording-changed", false);
                state.whisper_cache.unload();
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

            let loaded = state.whisper_cache.is_loaded();
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
                            logbuf::info_and_emit(&app, "Unloaded Whisper after idle.");
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
    begin_voice_session(app, true)
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
    settings.launch_at_login = !settings.launch_at_login;
    persist_settings(&state.settings_path, &settings)?;
    *state
        .settings
        .lock()
        .map_err(|_| "Settings lock was poisoned")? = settings.clone();
    apply_launch_at_login(app, settings.launch_at_login)?;
    let _ = app.emit("settings-changed", settings);
    refresh_tray_menu(app)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
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

    #[test]
    fn resampling_preserves_duration() {
        let input = vec![0.5; 48_000];
        let output = resample_to_16khz(&input, 48_000);
        assert_eq!(output.len(), 16_000);
        assert!(output
            .iter()
            .all(|sample| (*sample - 0.5).abs() < f32::EPSILON));
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
}
