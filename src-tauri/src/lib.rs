//! VocaWin's platform shell. Recognition engines are deliberately behind a small
//! catalog/adapter boundary so model downloads never require a cloud account.

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
use tauri::{Manager, State};
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
            size: "756 MB",
            languages: "English",
            acceleration: "CPU · Vulkan",
            description: "Fast English-only Whisper derivative.",
        },
        Model {
            id: "parakeet-tdt-0.6b-v3",
            name: "Parakeet TDT 0.6B v3",
            engine: "ONNX Runtime",
            size: "1.2 GB",
            languages: "25 languages",
            acceleration: "CPU · DirectML",
            description: "High-speed multilingual dictation.",
        },
        Model {
            id: "parakeet-ctc-1.1b",
            name: "Parakeet CTC 1.1B",
            engine: "ONNX Runtime",
            size: "2.2 GB",
            languages: "English",
            acceleration: "CPU · DirectML",
            description: "High-recall English transcription.",
        },
        Model {
            id: "moonshine-tiny",
            name: "Moonshine v2 Tiny",
            engine: "ONNX Runtime",
            size: "60 MB",
            languages: "English",
            acceleration: "CPU",
            description: "Low-memory, quick English notes.",
        },
        Model {
            id: "moonshine-base",
            name: "Moonshine v2 Base",
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
            size: "270 MB",
            languages: "Russian",
            acceleration: "CPU",
            description: "Russian recognition with punctuation.",
        },
        Model {
            id: "canary-180m",
            name: "Canary 180M Flash",
            engine: "ONNX Runtime",
            size: "320 MB",
            languages: "English · Spanish · German · French",
            acceleration: "CPU · DirectML",
            description: "Fast four-language transcription.",
        },
        Model {
            id: "vosk-small-en",
            name: "Vosk Small English",
            engine: "Vosk",
            size: "40 MB",
            languages: "English",
            acceleration: "CPU",
            description: "Small-footprint offline fallback.",
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
    launch_at_login: bool,
    sound_effects: bool,
    selected_model: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            hotkey: "Ctrl+Alt+Space".into(),
            activation_mode: "pushToTalk".into(),
            language: "Auto-detect".into(),
            silence_seconds: 1.5,
            launch_at_login: false,
            sound_effects: true,
            selected_model: "whisper-tiny".into(),
        }
    }
}

/// WASAPI `cpal::Stream` is intentionally `!Send`/`!Sync` across platforms.
/// Keep the live stream on one dedicated thread and expose only channel handles
/// to Tauri `State`, which requires `Send + Sync`.
#[cfg(windows)]
enum AudioCommand {
    Start {
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
fn open_input_stream(
    samples: Arc<Mutex<Vec<f32>>>,
) -> Result<(cpal::Stream, u32), String> {
    let device = cpal::default_host()
        .default_input_device()
        .ok_or("No microphone was found. Connect or enable an input device and try again.")?;
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
        cpal::SampleFormat::F32 => device.build_input_stream(
            &config,
            move |data: &[f32], _| {
                samples.lock().unwrap().extend(
                    data.chunks(channels)
                        .map(|frame| frame.iter().sum::<f32>() / frame.len() as f32),
                )
            },
            error_callback,
            None,
        ),
        cpal::SampleFormat::I16 => device.build_input_stream(
            &config,
            move |data: &[i16], _| {
                samples
                    .lock()
                    .unwrap()
                    .extend(data.chunks(channels).map(|frame| {
                        frame
                            .iter()
                            .map(|sample| *sample as f32 / i16::MAX as f32)
                            .sum::<f32>()
                            / frame.len() as f32
                    }))
            },
            error_callback,
            None,
        ),
        cpal::SampleFormat::U16 => device.build_input_stream(
            &config,
            move |data: &[u16], _| {
                samples
                    .lock()
                    .unwrap()
                    .extend(data.chunks(channels).map(|frame| {
                        frame
                            .iter()
                            .map(|sample| (*sample as f32 / u16::MAX as f32) * 2.0 - 1.0)
                            .sum::<f32>()
                            / frame.len() as f32
                    }))
            },
            error_callback,
            None,
        ),
        format => return Err(format!("Unsupported microphone sample format: {format:?}")),
    }
    .map_err(|error| format!("Could not open microphone: {error}"))?;
    stream
        .play()
        .map_err(|error| format!("Could not start microphone: {error}"))?;
    Ok((stream, sample_rate))
}

#[cfg(windows)]
fn audio_thread_main(commands: mpsc::Receiver<AudioCommand>) {
    let samples = Arc::new(Mutex::new(Vec::<f32>::new()));
    let mut stream: Option<cpal::Stream> = None;
    let mut sample_rate: Option<u32> = None;

    while let Ok(command) = commands.recv() {
        match command {
            AudioCommand::Start { reply } => {
                let result = if stream.is_some() {
                    Err("A recording is already in progress".into())
                } else {
                    match open_input_stream(Arc::clone(&samples)) {
                        Ok((next_stream, rate)) => {
                            sample_rate = Some(rate);
                            stream = Some(next_stream);
                            Ok(())
                        }
                        Err(error) => Err(error),
                    }
                };
                let _ = reply.send(result);
            }
            AudioCommand::Stop { reply } => {
                let result = if stream.take().is_none() {
                    Err("No recording is in progress".into())
                } else {
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
                };
                let _ = reply.send(result);
            }
        }
    }
}

#[cfg(windows)]
impl AudioRecorder {
    fn new() -> Self {
        let (commands, receiver) = mpsc::channel();
        std::thread::Builder::new()
            .name("vocawin-audio".into())
            .spawn(move || audio_thread_main(receiver))
            .expect("Could not start VocaWin audio thread");
        Self { commands }
    }

    fn start(&self) -> Result<(), String> {
        let (reply, response) = mpsc::channel();
        self.commands
            .send(AudioCommand::Start { reply })
            .map_err(|_| "Audio thread is not running")?;
        response
            .recv()
            .map_err(|_| "Audio thread did not respond".into())?
    }

    fn stop(&self) -> Result<(Vec<f32>, u32), String> {
        let (reply, response) = mpsc::channel();
        self.commands
            .send(AudioCommand::Stop { reply })
            .map_err(|_| "Audio thread is not running")?;
        response
            .recv()
            .map_err(|_| "Audio thread did not respond".into())?
    }
}

/// Non-Windows builds keep an unavailable mic stub so Linux/macOS CI can validate
/// the shared UI and command layer. Real capture lives behind `cfg(windows)`.
#[cfg(not(windows))]
struct AudioRecorder;
#[cfg(not(windows))]
impl AudioRecorder {
    fn new() -> Self {
        Self
    }
    fn start(&self) -> Result<(), String> {
        Err("Microphone capture is available in Windows builds only.".into())
    }
    fn stop(&self) -> Result<(Vec<f32>, u32), String> {
        Err("Microphone capture is available in Windows builds only.".into())
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
fn save_settings(settings: Settings, state: State<'_, AppState>) -> Result<(), String> {
    if !model_catalog()
        .iter()
        .any(|model| model.id == settings.selected_model)
    {
        return Err("Unknown speech model".into());
    }
    if !(0.3..=10.0).contains(&settings.silence_seconds) {
        return Err("Silence timeout must be between 0.3 and 10 seconds".into());
    }
    persist_settings(&state.settings_path, &settings)?;
    *state
        .settings
        .lock()
        .map_err(|_| "Settings lock was poisoned")? = settings;
    Ok(())
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

fn whisper_download_url(id: &str) -> Option<&'static str> {
    match id {
        "whisper-tiny" => {
            Some("https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-tiny.bin")
        }
        "whisper-base" => {
            Some("https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.bin")
        }
        "whisper-small" => {
            Some("https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-small.bin")
        }
        "whisper-medium" => {
            Some("https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-medium.bin")
        }
        "whisper-large-v3" => {
            Some("https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-large-v3.bin")
        }
        "whisper-large-v3-turbo" => Some(
            "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-large-v3-turbo.bin",
        ),
        _ => None,
    }
}

fn installation_status(state: &AppState, id: &str) -> ModelInstallStatus {
    if let Ok(downloads) = state.downloads.lock() {
        if let Some(status) = downloads.get(id) {
            return status.clone();
        }
    }
    ModelInstallStatus {
        installed: model_path(&state.models_path, id).exists(),
        downloadable: whisper_download_url(id).is_some(),
        downloading: false,
        progress: 0,
        message: None,
    }
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
    let url = whisper_download_url(&model_id).ok_or("In-app download is not available for this model yet. See the model guide for its local layout.")?;
    let destination = model_path(&state.models_path, &model_id);
    if destination.exists() {
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
    let temporary = destination.with_extension("bin.download");
    let result = async {
        use futures_util::StreamExt;
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
            tokio::io::AsyncWriteExt::write_all(&mut file, &chunk)
                .await
                .map_err(|error| format!("Could not write model file: {error}"))?;
            downloaded += chunk.len() as u64;
            let progress = total
                .map(|length| ((downloaded.saturating_mul(100) / length).min(99)) as u8)
                .unwrap_or(0);
            if let Ok(mut downloads) = state.downloads.lock() {
                downloads.insert(
                    model_id.clone(),
                    ModelInstallStatus {
                        installed: false,
                        downloadable: true,
                        downloading: true,
                        progress,
                        message: Some(format!("Downloading… {}%", progress)),
                    },
                );
            }
        }
        tokio::io::AsyncWriteExt::flush(&mut file)
            .await
            .map_err(|error| format!("Could not finalize model file: {error}"))?;
        tokio::fs::rename(&temporary, &destination)
            .await
            .map_err(|error| format!("Could not install model: {error}"))?;
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
        Err(error) => {
            let _ = tokio::fs::remove_file(&temporary).await;
            ModelInstallStatus {
                installed: false,
                downloadable: true,
                downloading: false,
                progress: 0,
                message: Some(error.clone()),
            }
        }
    };
    state
        .downloads
        .lock()
        .map_err(|_| "Download lock was poisoned")?
        .insert(model_id, status);
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
fn start_recording(state: State<'_, AppState>) -> Result<(), String> {
    state.recorder.start()
}

fn transcribe_recording(state: &AppState) -> Result<String, String> {
    let (samples, sample_rate) = state.recorder.stop()?;
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
    if !settings.selected_model.starts_with("whisper-")
        && !settings.selected_model.starts_with("distil-whisper-")
    {
        let text = transcribe_onnx(
            &settings.selected_model,
            &state.models_path,
            &pcm,
            language_code,
        )?;
        if !text.is_empty() {
            append_history(&state.history_path, text.clone(), settings.selected_model)?;
        }
        return Ok(text);
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
    let context = whisper_rs::WhisperContext::new_with_params(
        &model_path,
        whisper_rs::WhisperContextParameters::default(),
    )
    .map_err(|error| format!("Could not load Whisper model: {error}"))?;
    let mut session = context
        .create_state()
        .map_err(|error| format!("Could not create Whisper session: {error}"))?;
    let mut parameters =
        whisper_rs::FullParams::new(whisper_rs::SamplingStrategy::Greedy { best_of: 1 });
    parameters.set_translate(false);
    parameters.set_language(language_code);
    parameters.set_print_special(false);
    parameters.set_print_progress(false);
    parameters.set_print_realtime(false);
    parameters.set_print_timestamps(false);
    session
        .full(parameters, &pcm)
        .map_err(|error| format!("Transcription failed: {error}"))?;
    let text = (0..session.full_n_segments())
        .filter_map(|index| {
            session
                .get_segment(index)
                .and_then(|segment| segment.to_str().ok().map(str::to_owned))
        })
        .collect::<Vec<_>>()
        .join(" ");
    let text = text.trim().to_string();
    if !text.is_empty() {
        append_history(&state.history_path, text.clone(), settings.selected_model)?;
    }
    Ok(text)
}

#[tauri::command]
fn stop_and_transcribe(state: State<'_, AppState>) -> Result<String, String> {
    transcribe_recording(&state)
}

#[tauri::command]
fn system_summary() -> serde_json::Value {
    serde_json::json!({
        "platform": "Windows 10/11",
        "runtime": "Rust + Tauri",
        "privacy": "Audio and transcription remain on this device",
        "gpuBackends": ["Vulkan (whisper.cpp)", "DirectML (ONNX Runtime)", "CPU fallback"]
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
    platform::inject(&text)
}

#[cfg(windows)]
mod platform {
    pub fn inject(text: &str) -> Result<(), String> {
        use windows::Win32::UI::Input::KeyboardAndMouse::{
            SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_UNICODE, VIRTUAL_KEY,
        };
        let inputs: Vec<INPUT> = text
            .encode_utf16()
            .flat_map(|unit| {
                [
                    INPUT {
                        r#type: INPUT_KEYBOARD,
                        Anonymous: INPUT_0 {
                            ki: KEYBDINPUT {
                                wVk: VIRTUAL_KEY(0),
                                wScan: unit,
                                dwFlags: KEYEVENTF_UNICODE,
                                time: 0,
                                dwExtraInfo: 0,
                            },
                        },
                    },
                    INPUT {
                        r#type: INPUT_KEYBOARD,
                        Anonymous: INPUT_0 {
                            ki: KEYBDINPUT {
                                wVk: VIRTUAL_KEY(0),
                                wScan: unit,
                                dwFlags: KEYEVENTF_UNICODE
                                    | windows::Win32::UI::Input::KeyboardAndMouse::KEYEVENTF_KEYUP,
                                time: 0,
                                dwExtraInfo: 0,
                            },
                        },
                    },
                ]
            })
            .collect();
        let sent = unsafe { SendInput(&inputs, std::mem::size_of::<INPUT>() as i32) };
        if sent as usize == inputs.len() {
            Ok(())
        } else {
            Err("Windows rejected text injection; try running VocaWin at the same privilege level as the target app.".into())
        }
    }
}
#[cfg(not(windows))]
mod platform {
    pub fn inject(_: &str) -> Result<(), String> {
        Err("Text injection is available in Windows builds only.".into())
    }
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .setup(|app| {
            let app_data = app.path().app_data_dir()?;
            let settings_path = app_data.join("settings.json");
            let history_path = app_data.join("history.json");
            let models_path = app_data.join("models");
            fs::create_dir_all(&models_path)?;
            let settings = load_settings(&settings_path);
            app.manage(AppState {
                settings: Mutex::new(settings.clone()),
                settings_path,
                history_path,
                models_path,
                downloads: Mutex::new(HashMap::new()),
                recorder: AudioRecorder::new(),
            });
            #[cfg(windows)]
            {
                use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};
                if let Ok(shortcut) = settings.hotkey.parse::<Shortcut>() {
                    let shortcut_for_handler = shortcut;
                    app.global_shortcut().on_shortcut(shortcut, move |handle, _, event| {
                        let state = handle.state::<AppState>();
                        match event.state {
                            ShortcutState::Pressed => { let _ = state.recorder.start(); }
                            ShortcutState::Released => {
                                if let Ok(text) = transcribe_recording(&state) {
                                    let _ = platform::inject(&text);
                                }
                            }
                        }
                    }).map_err(|error| format!("Could not register global shortcut {shortcut_for_handler:?}: {error}"))?;
                }
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
            start_recording,
            stop_and_transcribe,
            inject_text
        ])
        .run(tauri::generate_context!())
        .expect("error while running VocaWin");
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
}
