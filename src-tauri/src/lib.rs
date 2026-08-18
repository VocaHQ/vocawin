//! VocaWin's platform shell. Recognition engines are deliberately behind a small
//! catalog/adapter boundary so model downloads never require a cloud account.

mod autopause;
mod devices;
mod gpu;
mod hardware;
mod hook;
mod hotkey;
mod output;
mod power;
mod sounds;
mod whisper_cache;

#[cfg(windows)]
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use serde::{Deserialize, Serialize};
#[cfg(windows)]
use std::sync::Arc;
use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    sync::Mutex,
};
#[cfg(windows)]
use std::sync::mpsc;
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

/// The catalog is intentionally engine-neutral. The transcription adapter can
/// select whisper.cpp (GGUF), ONNX Runtime, or Vosk without changing the UI.
fn model_catalog() -> Vec<Model> {
    vec![
        Model {
            id: "whisper-tiny",
            name: "Whisper Tiny",
            engine: "whisper.cpp",
            size: "75 MB",
            languages: "99 languages",
            acceleration: "CPU · Vulkan",
            description: "Fastest Whisper option; included as the first-run recommendation.",
        },
        Model {
            id: "whisper-base",
            name: "Whisper Base",
            engine: "whisper.cpp",
            size: "142 MB",
            languages: "99 languages",
            acceleration: "CPU · Vulkan",
            description: "Balanced accuracy for everyday dictation.",
        },
        Model {
            id: "whisper-small",
            name: "Whisper Small",
            engine: "whisper.cpp",
            size: "466 MB",
            languages: "99 languages",
            acceleration: "CPU · Vulkan",
            description: "Higher accuracy on modern PCs.",
        },
        Model {
            id: "whisper-medium",
            name: "Whisper Medium",
            engine: "whisper.cpp",
            size: "1.5 GB",
            languages: "99 languages",
            acceleration: "CPU · Vulkan",
            description: "Excellent multilingual recognition.",
        },
        Model {
            id: "whisper-large-v3",
            name: "Whisper Large v3",
            engine: "whisper.cpp",
            size: "3.1 GB",
            languages: "99 languages",
            acceleration: "CPU · Vulkan",
            description: "Maximum Whisper accuracy.",
        },
        Model {
            id: "whisper-large-v3-turbo",
            name: "Whisper Large v3 Turbo",
            engine: "whisper.cpp",
            size: "1.6 GB",
            languages: "99 languages",
            acceleration: "CPU · Vulkan",
            description: "Large-v3 quality tuned for lower latency.",
        },
        Model {
            id: "distil-whisper-large-v3",
            name: "Distil-Whisper Large v3",
            engine: "whisper.cpp",
            size: "1.5 GB",
            languages: "English",
            acceleration: "CPU · Vulkan",
            description: "Fast English-only Whisper derivative.",
        },
        Model {
            id: "parakeet-tdt-0.6b-v3",
            name: "Parakeet TDT 0.6B v3",
            engine: "ONNX Runtime",
            size: "478 MB",
            languages: "25 languages",
            acceleration: "CPU · DirectML",
            description: "High-speed multilingual dictation.",
        },
        Model {
            id: "moonshine-tiny",
            name: "Moonshine Tiny",
            engine: "ONNX Runtime",
            size: "145 MB",
            languages: "English",
            acceleration: "CPU",
            description: "Low-memory, quick English notes.",
        },
        Model {
            id: "moonshine-base",
            name: "Moonshine Base",
            engine: "ONNX Runtime",
            size: "190 MB",
            languages: "English",
            acceleration: "CPU",
            description: "Compact English model.",
        },
        Model {
            id: "sensevoice-small",
            name: "SenseVoice Small",
            engine: "ONNX Runtime",
            size: "240 MB",
            languages: "Chinese · Japanese · Korean · Cantonese · English",
            acceleration: "CPU · DirectML",
            description: "East Asian language specialist.",
        },
        Model {
            id: "gigaam-v3",
            name: "GigaAM v3",
            engine: "ONNX Runtime",
            size: "225 MB",
            languages: "Russian",
            acceleration: "CPU",
            description: "Russian recognition with punctuation.",
        },
        Model {
            id: "canary-180m",
            name: "Canary 180M Flash",
            engine: "ONNX Runtime",
            size: "150 MB",
            languages: "English · Spanish · German · French",
            acceleration: "CPU · DirectML",
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
    sound_effects: bool,
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
            hotkey: "Ctrl+Alt+Space".into(),
            activation_mode: "pushToTalk".into(),
            language: "Auto-detect".into(),
            silence_seconds: 1.5,
            max_recording_seconds: 60.0,
            launch_at_login: false,
            sound_effects: true,
            append_trailing_space: true,
            auto_capitalize: true,
            selected_model: "whisper-tiny".into(),
            input_device: String::new(),
            auto_pause_enabled: false,
            auto_pause_apps: String::new(),
            idle_unload_enabled: false,
            idle_unload_seconds: 300,
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
        reply: mpsc::Sender<Result<(), String>>,
    },
    Stop {
        reply: mpsc::Sender<Result<(Vec<f32>, u32), String>>,
    },
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
) {
    const VOICE_THRESHOLD: f32 = 0.015;
    if mono.abs() >= VOICE_THRESHOLD {
        if let Ok(mut heard) = heard_speech.lock() {
            *heard = true;
        }
        if let Ok(mut last) = last_voice_ms.lock() {
            *last = now_ms();
        }
    }
    samples.lock().unwrap().push(mono);
}

#[cfg(windows)]
fn open_input_stream(
    samples: Arc<Mutex<Vec<f32>>>,
    last_voice_ms: Arc<Mutex<u128>>,
    heard_speech: Arc<Mutex<bool>>,
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
    let error_callback = |error| eprintln!("VocaWin audio input error: {error}");
    let config: cpal::StreamConfig = supported.clone().into();
    let stream = match supported.sample_format() {
        cpal::SampleFormat::F32 => {
            let samples = Arc::clone(&samples);
            let last_voice_ms = Arc::clone(&last_voice_ms);
            let heard_speech = Arc::clone(&heard_speech);
            device.build_input_stream(
                &config,
                move |data: &[f32], _| {
                    for frame in data.chunks(channels) {
                        let mono = frame.iter().sum::<f32>() / frame.len() as f32;
                        note_audio_sample(mono, &samples, &last_voice_ms, &heard_speech);
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
            device.build_input_stream(
                &config,
                move |data: &[i16], _| {
                    for frame in data.chunks(channels) {
                        let mono = frame
                            .iter()
                            .map(|sample| *sample as f32 / i16::MAX as f32)
                            .sum::<f32>()
                            / frame.len() as f32;
                        note_audio_sample(mono, &samples, &last_voice_ms, &heard_speech);
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
            device.build_input_stream(
                &config,
                move |data: &[u16], _| {
                    for frame in data.chunks(channels) {
                        let mono = frame
                            .iter()
                            .map(|sample| (*sample as f32 / u16::MAX as f32) * 2.0 - 1.0)
                            .sum::<f32>()
                            / frame.len() as f32;
                        note_audio_sample(mono, &samples, &last_voice_ms, &heard_speech);
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
    let mut stream: Option<cpal::Stream> = None;
    let mut sample_rate: Option<u32> = None;
    let mut started_ms: Option<u128> = None;
    let mut silence_seconds = 1.5_f32;
    let mut max_seconds = 60.0_f32;

    loop {
        let timed_out = match commands.recv_timeout(std::time::Duration::from_millis(100)) {
            Ok(command) => {
                match command {
                    AudioCommand::Start {
                        silence_seconds: silence,
                        max_seconds: max,
                        device_name,
                        reply,
                    } => {
                        silence_seconds = silence.clamp(0.3, 10.0);
                        max_seconds = max.clamp(3.0, 300.0);
                        let result = if stream.is_some() {
                            Err("A recording is already in progress".into())
                        } else {
                            match open_input_stream(
                                Arc::clone(&samples),
                                Arc::clone(&last_voice_ms),
                                Arc::clone(&heard_speech),
                                &device_name,
                            ) {
                                Ok((next_stream, rate)) => {
                                    sample_rate = Some(rate);
                                    stream = Some(next_stream);
                                    started_ms = Some(now_ms());
                                    *last_voice_ms.lock().unwrap() = now_ms();
                                    *heard_speech.lock().unwrap() = false;
                                    let _ = app.emit("recording-changed", true);
                                    set_tray_recording(&app, true);
                                    Ok(())
                                }
                                Err(error) => Err(error),
                            }
                        };
                        let _ = reply.send(result);
                    }
                    AudioCommand::Stop { reply } => {
                        let result = take_recording(
                            &mut stream,
                            &mut sample_rate,
                            &mut started_ms,
                            &samples,
                        );
                        if result.is_ok() {
                            let _ = app.emit("recording-changed", false);
                            set_tray_recording(&app, false);
                        }
                        let _ = reply.send(result);
                    }
                }
                false
            }
            Err(mpsc::RecvTimeoutError::Timeout) => true,
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        };

        if timed_out && stream.is_some() {
            let started = started_ms.unwrap_or_else(now_ms);
            let elapsed = (now_ms().saturating_sub(started)) as f32 / 1000.0;
            let last_voice = *last_voice_ms.lock().unwrap_or_else(|e| e.into_inner());
            let quiet_for = (now_ms().saturating_sub(last_voice)) as f32 / 1000.0;
            let heard = *heard_speech.lock().unwrap_or_else(|e| e.into_inner());
            let silence_hit = heard && quiet_for >= silence_seconds;
            let max_hit = elapsed >= max_seconds;
            if silence_hit || max_hit {
                if let Ok((pcm, rate)) =
                    take_recording(&mut stream, &mut sample_rate, &mut started_ms, &samples)
                {
                    let _ = app.emit("recording-changed", false);
                    set_tray_recording(&app, false);
                    let app_for_finish = app.clone();
                    std::thread::spawn(move || {
                        finish_captured_audio(&app_for_finish, pcm, rate);
                    });
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

    fn start(&self, silence_seconds: f32, max_seconds: f32, device_name: String) -> Result<(), String> {
        let (reply, response) = mpsc::channel();
        self.commands
            .send(AudioCommand::Start {
                silence_seconds,
                max_seconds,
                device_name,
                reply,
            })
            .map_err(|_| "Audio thread is not running".to_string())?;
        response
            .recv()
            .map_err(|_| "Audio thread did not respond".to_string())?
    }

    fn stop(&self) -> Result<(Vec<f32>, u32), String> {
        let (reply, response) = mpsc::channel();
        self.commands
            .send(AudioCommand::Stop { reply })
            .map_err(|_| "Audio thread is not running".to_string())?;
        response
            .recv()
            .map_err(|_| "Audio thread did not respond".to_string())?
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
    fn start(&self, _: f32, _: f32, _: String) -> Result<(), String> {
        Err("Microphone capture is available in Windows builds only.".into())
    }
    fn stop(&self) -> Result<(Vec<f32>, u32), String> {
        Err("Microphone capture is available in Windows builds only.".into())
    }
}

fn set_tray_recording(app: &AppHandle, recording: bool) {
    if let Some(tray) = app.tray_by_id("main") {
        let tip = if recording {
            "VocaWin - Recording"
        } else {
            "VocaWin"
        };
        let _ = tray.set_tooltip(Some(tip));
    }
}

#[cfg(windows)]
fn finish_captured_audio(app: &AppHandle, samples: Vec<f32>, sample_rate: u32) {
    let state = app.state::<AppState>();
    {
        let mut flag = state.recording.lock().unwrap_or_else(|e| e.into_inner());
        *flag = false;
    }
    let sound = state
        .settings
        .lock()
        .map(|settings| settings.sound_effects)
        .unwrap_or(true);
    sounds::play_if_enabled(sound, false);
    match transcribe_samples(&state, samples, sample_rate) {
        Ok(text) if !text.is_empty() => {
            let _ = output::inject(&text);
            let _ = app.emit("dictation-finished", text);
        }
        Ok(_) => {
            let _ = app.emit("dictation-finished", String::new());
        }
        Err(error) => {
            let _ = app.emit("dictation-error", error);
        }
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ModelInstallStatus {
    installed: bool,
    downloadable: bool,
    downloading: bool,
    progress: u8,
    message: Option<String>,
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
    registered_hotkey: Mutex<String>,
    dictation_paused: Mutex<bool>,
    whisper_cache: whisper_cache::WhisperCache,
}

/// A malformed or partially-written settings file must never prevent dictation
/// from starting. In that case we retain the file for diagnosis and use safe
/// defaults until the user saves settings again.
fn load_settings(path: &std::path::Path) -> Settings {
    fs::read_to_string(path)
        .ok()
        .and_then(|contents| serde_json::from_str(&contents).ok())
        .unwrap_or_default()
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
    settings.hotkey = hotkey::canonicalize(&settings.hotkey)?;
    persist_settings(&state.settings_path, &settings)?;
    apply_launch_at_login(&app, settings.launch_at_login)?;
    state
        .whisper_cache
        .configure_idle(settings.idle_unload_enabled, settings.idle_unload_seconds);
    if !settings.idle_unload_enabled {
        state.whisper_cache.unload();
    }
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
    *state
        .settings
        .lock()
        .map_err(|_| "Settings lock was poisoned")? = settings;
    Ok(())
}

fn apply_launch_at_login(app: &AppHandle, enabled: bool) -> Result<(), String> {
    use tauri_plugin_autostart::ManagerExt;
    let autolaunch = app.autolaunch();
    if enabled {
        autolaunch
            .enable()
            .map_err(|error| format!("Could not enable launch at login: {error}"))
    } else {
        autolaunch
            .disable()
            .map_err(|error| format!("Could not disable launch at login: {error}"))
    }
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
    Files { files: &'static [(&'static str, &'static str)] },
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

fn installation_status(state: &AppState, id: &str) -> ModelInstallStatus {
    if let Ok(downloads) = state.downloads.lock() {
        if let Some(status) = downloads.get(id) {
            return status.clone();
        }
    }
    ModelInstallStatus {
        installed: model_is_installed(&state.models_path, id),
        downloadable: model_package(id).is_some(),
        downloading: false,
        progress: 0,
        message: None,
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
        },
    );
}

async fn download_url_to_file(
    state: &AppState,
    model_id: &str,
    url: &str,
    destination: &Path,
    progress_start: u8,
    progress_end: u8,
) -> Result<(), String> {
    use futures_util::StreamExt;
    use tokio::io::AsyncWriteExt;

    if let Some(parent) = destination.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .map_err(|error| format!("Could not create download directory: {error}"))?;
    }
    let temporary = destination.with_extension("partial");
    let result = async {
        let response = reqwest::get(url)
            .await
            .map_err(|error| format!("Could not start download: {error}"))?
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
    for entry in fs::read_dir(path).map_err(|error| format!("Could not read extract directory: {error}"))?
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
async fn download_model(model_id: String, state: State<'_, AppState>) -> Result<(), String> {
    let package = model_package(&model_id).ok_or_else(|| {
        "In-app download is not available for this model.".to_string()
    })?;
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
            },
        );
    }

    let result = async {
        match package {
            ModelPackage::GgmlBin { url } => {
                let destination = model_path(&state.models_path, &model_id);
                download_url_to_file(&state, &model_id, url, &destination, 0, 99).await?;
            }
            ModelPackage::TarGz { url } => {
                let archive = state
                    .models_path
                    .join(format!("{model_id}.tar.gz.partial"));
                download_url_to_file(&state, &model_id, url, &archive, 0, 85).await?;
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
                    download_url_to_file(&state, &model_id, url, &file_path, start, end).await?;
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
        Ok(()) => ModelInstallStatus {
            installed: true,
            downloadable: true,
            downloading: false,
            progress: 100,
            message: Some("Installed".into()),
        },
        Err(error) => ModelInstallStatus {
            installed: model_is_installed(&state.models_path, &model_id),
            downloadable: true,
            downloading: false,
            progress: 0,
            message: Some(error.clone()),
        },
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
    // Clean leftover partial downloads for this model.
    let partial_archive = state.models_path.join(format!("{model_id}.tar.gz.partial"));
    let _ = fs::remove_file(partial_archive);
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

/// Begins microphone capture. The UI should call `stop_and_transcribe` after
/// push-to-talk is released (or when toggle mode is stopped).
#[tauri::command]
fn start_recording(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    if *state
        .dictation_paused
        .lock()
        .map_err(|_| "Pause lock was poisoned")?
    {
        return Err("Dictation is paused while a watched app is running.".into());
    }
    let settings = state
        .settings
        .lock()
        .map_err(|_| "Settings lock was poisoned")?
        .clone();
    state.recorder.start(
        settings.silence_seconds,
        settings.max_recording_seconds,
        settings.input_device.clone(),
    )?;
    *state
        .recording
        .lock()
        .map_err(|_| "Recording lock was poisoned")? = true;
    sounds::play_if_enabled(settings.sound_effects, true);
    let _ = app.emit("recording-changed", true);
    set_tray_recording(&app, true);
    Ok(())
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
    if pcm.len() < 4_000 {
        return Err("Recording is too short. Hold the hotkey and speak for a moment.".into());
    }
    let language_code = match settings.language.as_str() {
        "English" => Some("en"),
        "Spanish" => Some("es"),
        "French" => Some("fr"),
        "German" => Some("de"),
        "Japanese" => Some("ja"),
        "Chinese" => Some("zh"),
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
        state.whisper_cache.transcribe(
            model_path,
            pcm,
            language_code.map(str::to_string),
            gpu.available,
            settings.idle_unload_enabled,
        )?
    };
    let text = output::apply_output_polish(
        &raw,
        settings.auto_capitalize,
        settings.append_trailing_space,
    );
    if !text.trim().is_empty() {
        append_history(
            &state.history_path,
            text.trim().to_string(),
            settings.selected_model,
        )?;
    }
    Ok(text)
}

fn transcribe_recording(state: &AppState) -> Result<String, String> {
    let (samples, sample_rate) = state.recorder.stop()?;
    *state
        .recording
        .lock()
        .map_err(|_| "Recording lock was poisoned")? = false;
    transcribe_samples(state, samples, sample_rate)
}

#[tauri::command]
fn stop_and_transcribe(app: AppHandle, state: State<'_, AppState>) -> Result<String, String> {
    let sound = state
        .settings
        .lock()
        .map(|settings| settings.sound_effects)
        .unwrap_or(true);
    let text = transcribe_recording(&state)?;
    sounds::play_if_enabled(sound, false);
    let _ = app.emit("recording-changed", false);
    set_tray_recording(&app, false);
    Ok(text)
}

#[tauri::command]
fn system_summary() -> serde_json::Value {
    let gpu = gpu::detect_gpu();
    serde_json::json!({
        "platform": "Windows 10/11",
        "runtime": "Rust + Tauri",
        "privacy": "Audio and transcription remain on this device",
        "gpuBackends": ["Vulkan (whisper.cpp)", "DirectML (ONNX Runtime)", "CPU fallback"],
        "gpu": gpu
    })
}

#[tauri::command]
fn get_gpu_status() -> gpu::GpuStatus {
    gpu::detect_gpu()
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

#[tauri::command]
fn get_runtime_status(state: State<'_, AppState>) -> serde_json::Value {
    let settings = state
        .settings
        .lock()
        .map(|s| s.clone())
        .unwrap_or_default();
    let recording = *state.recording.lock().unwrap_or_else(|e| e.into_inner());
    let paused = *state
        .dictation_paused
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    let gpu = gpu::detect_gpu();
    let status = if recording {
        "Recording"
    } else if paused {
        "Paused"
    } else {
        "Ready"
    };
    serde_json::json!({
        "status": status,
        "recording": recording,
        "paused": paused,
        "hotkey": settings.hotkey,
        "inputDevice": if settings.input_device.is_empty() { "Default microphone".into() } else { settings.input_device },
        "gpuName": gpu.name,
        "gpuBackend": gpu.backend,
    })
}

/// On Windows this is the final platform boundary: the recognizer gives us
/// text, then the injector enters it at the focused application. It is kept
/// separate from recognition so every engine gets identical insertion behavior.
#[tauri::command]
fn inject_text(text: String) -> Result<(), String> {
    if text.trim().is_empty() {
        return Ok(());
    }
    output::inject(&text)
}

pub fn run() {
    let mut builder = tauri::Builder::default();
    builder = builder.plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
        if let Some(window) = app.get_webview_window("main") {
            let _ = window.show();
            let _ = window.set_focus();
        }
    }));
    builder
        .plugin(tauri_plugin_autostart::Builder::new().build())
        .setup(|app| {
            let app_data = app.path().app_data_dir()?;
            let settings_path = app_data.join("settings.json");
            let history_path = app_data.join("history.json");
            let models_path = app_data.join("models");
            fs::create_dir_all(&models_path)?;
            let settings = load_settings(&settings_path);
            let handle = app.handle().clone();
            let whisper_cache = whisper_cache::WhisperCache::new();
            whisper_cache.configure_idle(
                settings.idle_unload_enabled,
                settings.idle_unload_seconds,
            );
            app.manage(AppState {
                settings: Mutex::new(settings.clone()),
                settings_path,
                history_path,
                models_path,
                downloads: Mutex::new(HashMap::new()),
                recorder: AudioRecorder::new(handle.clone()),
                recording: Mutex::new(false),
                registered_hotkey: Mutex::new(settings.hotkey.clone()),
                dictation_paused: Mutex::new(false),
                whisper_cache,
            });
            let _ = apply_launch_at_login(&handle, settings.launch_at_login);
            hook::start(handle.clone());
            if let Err(error) = register_dictation_hotkey(&handle, &settings.hotkey) {
                eprintln!("VocaWin hotkey registration failed: {error}");
            }
            setup_tray(app)?;
            start_auto_pause_watcher(handle.clone());
            power::start_sleep_wake_watcher(handle.clone(), |app| {
                let state = app.state::<AppState>();
                let paused = *state.dictation_paused.lock().unwrap_or_else(|e| e.into_inner());
                if paused {
                    return;
                }
                let hotkey = state
                    .registered_hotkey
                    .lock()
                    .map(|value| value.clone())
                    .unwrap_or_else(|_| "Ctrl+Alt+Space".into());
                if let Err(error) = register_dictation_hotkey(&app, &hotkey) {
                    eprintln!("VocaWin hotkey re-register after wake failed: {error}");
                } else {
                    let _ = app.emit("runtime-status", "Ready");
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
            }
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
            get_hotkey_presets,
            pause_hotkey_listener,
            resume_hotkey_listener,
            list_input_devices,
            recommend_model,
            get_runtime_status,
            start_recording,
            stop_and_transcribe,
            inject_text
        ])
        .run(tauri::generate_context!())
        .expect("error while running VocaWin");
}

fn register_dictation_hotkey(_app: &AppHandle, hotkey_spec: &str) -> Result<(), String> {
    let binding = hotkey::parse_hotkey(hotkey_spec)?;
    hook::set_binding(binding);
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
    if *state.dictation_paused.lock().unwrap_or_else(|e| e.into_inner()) {
        return;
    }
    let settings = state
        .settings
        .lock()
        .map(|s| s.clone())
        .unwrap_or_default();
    let toggle = settings.activation_mode == "toggle";
    match event {
        hook::HookEvent::Pressed => {
            let recording = *state.recording.lock().unwrap_or_else(|e| e.into_inner());
            if toggle {
                if recording {
                    if let Ok(text) = transcribe_recording(&state) {
                        sounds::play_if_enabled(settings.sound_effects, false);
                        let _ = output::inject(&text);
                        let _ = handle.emit("dictation-finished", text);
                    }
                    let _ = handle.emit("recording-changed", false);
                    set_tray_recording(handle, false);
                } else if state
                    .recorder
                    .start(
                        settings.silence_seconds,
                        settings.max_recording_seconds,
                        settings.input_device.clone(),
                    )
                    .is_ok()
                {
                    *state.recording.lock().unwrap_or_else(|e| e.into_inner()) = true;
                    sounds::play_if_enabled(settings.sound_effects, true);
                    let _ = handle.emit("recording-changed", true);
                    set_tray_recording(handle, true);
                }
            } else if !recording
                && state
                    .recorder
                    .start(
                        settings.silence_seconds,
                        settings.max_recording_seconds,
                        settings.input_device.clone(),
                    )
                    .is_ok()
            {
                *state.recording.lock().unwrap_or_else(|e| e.into_inner()) = true;
                sounds::play_if_enabled(settings.sound_effects, true);
                let _ = handle.emit("recording-changed", true);
                set_tray_recording(handle, true);
            }
        }
        hook::HookEvent::Released => {
            if toggle {
                return;
            }
            let recording = *state.recording.lock().unwrap_or_else(|e| e.into_inner());
            if recording {
                if let Ok(text) = transcribe_recording(&state) {
                    sounds::play_if_enabled(settings.sound_effects, false);
                    let _ = output::inject(&text);
                    let _ = handle.emit("dictation-finished", text);
                }
                let _ = handle.emit("recording-changed", false);
                set_tray_recording(handle, false);
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
            let should_pause = settings.auto_pause_enabled
                && autopause::matching_process_running(&autopause::parse_app_list(
                    &settings.auto_pause_apps,
                ));
            let mut paused = match state.dictation_paused.lock() {
                Ok(guard) => guard,
                Err(_) => continue,
            };
            if should_pause && !*paused {
                *paused = true;
                drop(paused);
                hook::set_dictation_paused(true);
                let _ = app.emit("runtime-status", "Paused");
            } else if !should_pause && *paused {
                *paused = false;
                drop(paused);
                hook::set_dictation_paused(false);
                if let Err(error) = register_dictation_hotkey(&app, &settings.hotkey) {
                    eprintln!("VocaWin hotkey restore after auto-pause failed: {error}");
                }
                let _ = app.emit("runtime-status", "Ready");
            }
        })
        .ok();
}

fn setup_tray(app: &tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    use tauri::menu::{Menu, MenuItem};
    use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};

    let show = MenuItem::with_id(app, "show", "Show window", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show, &quit])?;
    let icon = app
        .default_window_icon()
        .ok_or("VocaWin is missing its default window icon for the tray")?
        .clone();

    TrayIconBuilder::with_id("main")
        .icon(icon)
        .tooltip("VocaWin")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "show" => {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
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
        assert!(!catalog.iter().any(|m| m.id.contains("vosk") || m.id.contains("ctc")));
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
    fn settings_round_trip_to_disk() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("VocaWin/settings.json");
        let settings = Settings {
            hotkey: "Ctrl+Shift+V".into(),
            ..Settings::default()
        };
        persist_settings(&path, &settings).unwrap();
        assert_eq!(load_settings(&path).hotkey, "Ctrl+Shift+V");
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
}
