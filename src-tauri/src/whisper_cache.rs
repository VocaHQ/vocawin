//! Whisper keep-alive cache with optional idle unload (opt-in).
//! Never / disabled keeps the model in RAM. A timeout unloads after quiet time.
//!
//! Whisper runs on transcribe.cpp (`transcribe-cpp`), Handy's engine, which
//! reads the same whisper.cpp GGML `.bin` files the catalog downloads (and
//! GGUF). It replaced whisper-rs so that one ggml serves every native model:
//! linking both put two ggml versions behind one set of symbols.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc};
use std::time::{Duration, Instant};

pub struct WhisperCache {
    commands: mpsc::Sender<CacheCommand>,
    loaded: Arc<AtomicBool>,
}

enum CacheCommand {
    Transcribe {
        model_path: PathBuf,
        pcm: Vec<f32>,
        language: Option<String>,
        use_gpu: bool,
        gpu_name: String,
        keep_alive: bool,
        initial_prompt: String,
        reply: mpsc::Sender<Result<String, String>>,
    },
    /// Load a model ahead of its take (hotkey press). Queued before the
    /// take's Transcribe, so that take finds it loaded.
    Preload {
        model_path: PathBuf,
        use_gpu: bool,
        gpu_name: String,
    },
    Unload,
    ConfigureIdle {
        enabled: bool,
        seconds: u32,
    },
}

impl WhisperCache {
    pub fn new() -> Self {
        let (commands, receiver) = mpsc::channel();
        let loaded = Arc::new(AtomicBool::new(false));
        let loaded_for_thread = loaded.clone();
        std::thread::Builder::new()
            .name("vocawin-whisper".into())
            .spawn(move || cache_thread_main(receiver, loaded_for_thread))
            .expect("Could not start Whisper cache thread");
        Self { commands, loaded }
    }

    pub fn is_loaded(&self) -> bool {
        self.loaded.load(Ordering::Relaxed)
    }

    pub fn transcribe(
        &self,
        model_path: PathBuf,
        pcm: Vec<f32>,
        language: Option<String>,
        use_gpu: bool,
        gpu_name: String,
        keep_alive: bool,
        initial_prompt: String,
    ) -> Result<String, String> {
        let (reply, response) = mpsc::channel();
        self.commands
            .send(CacheCommand::Transcribe {
                model_path,
                pcm,
                language,
                use_gpu,
                gpu_name,
                keep_alive,
                initial_prompt,
                reply,
            })
            .map_err(|_| "Whisper cache thread is not running".to_string())?;
        response
            .recv()
            .map_err(|_| "Whisper cache thread did not respond".to_string())?
    }

    /// Starts loading `model_path` without waiting for it.
    pub fn preload(&self, model_path: PathBuf, use_gpu: bool, gpu_name: String) {
        let _ = self.commands.send(CacheCommand::Preload {
            model_path,
            use_gpu,
            gpu_name,
        });
    }

    pub fn configure_idle(&self, enabled: bool, seconds: u32) {
        let _ = self
            .commands
            .send(CacheCommand::ConfigureIdle { enabled, seconds });
    }

    pub fn unload(&self) {
        let _ = self.commands.send(CacheCommand::Unload);
    }
}

fn cache_thread_main(commands: mpsc::Receiver<CacheCommand>, loaded: Arc<AtomicBool>) {
    let mut loaded_path: Option<PathBuf> = None;
    let mut context: Option<transcribe_cpp::Session> = None;
    let mut last_used = Instant::now();
    let mut idle_enabled = false;
    let mut idle_seconds = 300u32;

    loop {
        let timed_out = match commands.recv_timeout(Duration::from_secs(1)) {
            Ok(CacheCommand::Transcribe {
                model_path,
                pcm,
                language,
                use_gpu,
                gpu_name,
                keep_alive,
                initial_prompt,
                reply,
            }) => {
                let result = run_transcribe(
                    &mut loaded_path,
                    &mut context,
                    &model_path,
                    &pcm,
                    language.as_deref(),
                    use_gpu,
                    &gpu_name,
                    keep_alive,
                    &initial_prompt,
                );
                loaded.store(context.is_some(), Ordering::Relaxed);
                if result.is_ok() {
                    last_used = Instant::now();
                }
                let _ = reply.send(result);
                false
            }
            Ok(CacheCommand::Preload {
                model_path,
                use_gpu,
                gpu_name,
            }) => {
                match ensure_loaded(&mut loaded_path, &mut context, &model_path, use_gpu, &gpu_name) {
                    Ok(()) => last_used = Instant::now(),
                    Err(error) => crate::logbuf::debug(format!("Whisper preload failed: {error}")),
                }
                loaded.store(context.is_some(), Ordering::Relaxed);
                false
            }
            Ok(CacheCommand::Unload) => {
                if loaded_path.is_some() {
                    crate::logbuf::info("Whisper model unloaded.");
                }
                loaded_path = None;
                context = None;
                loaded.store(false, Ordering::Relaxed);
                false
            }
            Ok(CacheCommand::ConfigureIdle { enabled, seconds }) => {
                idle_enabled = enabled;
                idle_seconds = seconds.max(30);
                false
            }
            Err(mpsc::RecvTimeoutError::Timeout) => true,
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        };

        if timed_out
            && idle_enabled
            && context.is_some()
            && last_used.elapsed() >= Duration::from_secs(idle_seconds as u64)
        {
            loaded_path = None;
            context = None;
            loaded.store(false, Ordering::Relaxed);
        } else {
            loaded.store(context.is_some(), Ordering::Relaxed);
        }
    }
}

/// Loads `model_path` unless it is the model already loaded. With `use_gpu`
/// it asks for Vulkan on the adapter VocaWin picked (matched by name among
/// transcribe.cpp's devices, or its own choice when none matches), and falls
/// back to CPU if the GPU load fails.
fn ensure_loaded(
    loaded_path: &mut Option<PathBuf>,
    context: &mut Option<transcribe_cpp::Session>,
    model_path: &PathBuf,
    use_gpu: bool,
    gpu_name: &str,
) -> Result<(), String> {
    use transcribe_cpp::{Backend, Model, ModelOptions};

    if context.is_some() && loaded_path.as_ref() == Some(model_path) {
        return Ok(());
    }
    let cpu = || ModelOptions {
        backend: Backend::Cpu,
        device: None,
    };
    let (options, on) = if use_gpu {
        let device = transcribe_cpp::devices()
            .into_iter()
            .find(|device| device.kind == "vulkan" && same_adapter(&device.description, gpu_name));
        let label = device
            .as_ref()
            .map(|device| device.description.clone())
            .unwrap_or_else(|| "Vulkan".into());
        (
            ModelOptions {
                backend: Backend::Vulkan,
                device,
            },
            label,
        )
    } else {
        (cpu(), "CPU".to_string())
    };
    let loaded = Model::load_with(model_path, &options).or_else(|gpu_error| {
        if !use_gpu {
            return Err(gpu_error);
        }
        crate::logbuf::warn(format!("Whisper could not load on {on} ({gpu_error}); using CPU."));
        Model::load_with(model_path, &cpu())
    });
    let session = loaded
        .and_then(|model| model.session())
        .map_err(|error| format!("Could not load Whisper model: {error}"))?;
    *context = Some(session);
    *loaded_path = Some(model_path.clone());
    crate::logbuf::info(format!(
        "Loaded Whisper model {} on {on}",
        model_path
            .file_stem()
            .and_then(|name| name.to_str())
            .unwrap_or("whisper")
    ));
    Ok(())
}

/// Whether a transcribe.cpp device description names the DXGI adapter
/// (Vulkan and DXGI word the same GPU slightly differently).
fn same_adapter(description: &str, adapter: &str) -> bool {
    let normalize = |text: &str| {
        text.to_ascii_lowercase()
            .replace("(r)", "")
            .replace("(tm)", "")
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
    };
    let (description, adapter) = (normalize(description), normalize(adapter));
    !adapter.is_empty()
        && !description.is_empty()
        && (description.contains(&adapter) || adapter.contains(&description))
}

fn run_transcribe(
    loaded_path: &mut Option<PathBuf>,
    context: &mut Option<transcribe_cpp::Session>,
    model_path: &PathBuf,
    pcm: &[f32],
    language: Option<&str>,
    use_gpu: bool,
    gpu_name: &str,
    keep_alive: bool,
    initial_prompt: &str,
) -> Result<String, String> {
    use transcribe_cpp::{RunExtension, RunOptions, WhisperRunOptions};

    ensure_loaded(loaded_path, context, model_path, use_gpu, gpu_name)?;
    let session = context
        .as_mut()
        .ok_or("Whisper context missing after load")?;
    // Engine field is initial_prompt (whisper.cpp has no vocabulary param).
    let prompt = initial_prompt.replace('\0', "");
    let options = RunOptions {
        language: language.map(str::to_owned),
        family: (!prompt.is_empty()).then(|| {
            RunExtension::Whisper(WhisperRunOptions {
                initial_prompt: Some(prompt),
                ..Default::default()
            })
        }),
        ..Default::default()
    };
    let transcript = session
        .run(pcm, &options)
        .map_err(|error| format!("Transcription failed: {error}"))?;
    let segments: Vec<&str> = if transcript.segments.is_empty() {
        vec![transcript.text.as_str()]
    } else {
        transcript.segments.iter().map(|segment| segment.text.as_str()).collect()
    };
    let text = segments
        .into_iter()
        .map(spoken_text)
        .filter(|segment| !segment.is_empty())
        .collect::<Vec<_>>()
        .join(" ");
    if !keep_alive {
        *context = None;
        *loaded_path = None;
    }
    Ok(text)
}

/// A segment's text without whisper.cpp's non-speech markers. On silence or
/// noise Whisper writes tags such as `[BLANK_AUDIO]`, `[ Silence ]`,
/// `(music)` or `*sigh*` as ordinary text, and typing them is never right.
/// Square brackets are always markers: Whisper has no way to write dictated
/// words in them. Parentheses and asterisks count only when they are all
/// the segment holds, since spoken asides can use them.
fn spoken_text(segment: &str) -> String {
    let mut kept = String::with_capacity(segment.len());
    let mut rest = segment;
    while let Some(open) = rest.find('[') {
        let Some(close) = rest[open..].find(']') else {
            break;
        };
        kept.push_str(&rest[..open]);
        rest = &rest[open + close + 1..];
    }
    kept.push_str(rest);
    let text = kept.split_whitespace().collect::<Vec<_>>().join(" ");
    if only_markers(&text) {
        String::new()
    } else {
        text
    }
}

/// True when nothing but closed `[...]` / `(...)` / `*...*` groups, music
/// notes and punctuation is left. A group that never closes is speech.
fn only_markers(text: &str) -> bool {
    let mut inside: Option<char> = None;
    for ch in text.chars() {
        match inside {
            Some(close) if ch == close => inside = None,
            Some(_) => {}
            None => match ch {
                '[' => inside = Some(']'),
                '(' => inside = Some(')'),
                '*' => inside = Some('*'),
                _ if ch.is_alphanumeric() => return false,
                _ => {}
            },
        }
    }
    inside.is_none()
}

#[cfg(test)]
mod tests {
    use super::{same_adapter, spoken_text};

    #[test]
    fn vulkan_devices_match_their_dxgi_adapter() {
        assert!(same_adapter("NVIDIA GeForce RTX 4060 Laptop GPU", "NVIDIA GeForce RTX 4060 Laptop GPU"));
        assert!(same_adapter("Intel(R) Iris(R) Xe Graphics", "Intel Iris Xe Graphics"));
        assert!(same_adapter("AMD Radeon RX 7800 XT (RADV NAVI32)", "AMD Radeon RX 7800 XT"));
        assert!(!same_adapter("AMD Radeon RX 7800 XT", "NVIDIA GeForce RTX 4060"));
        assert!(!same_adapter("NVIDIA GeForce RTX 4060", ""));
    }

    #[test]
    fn non_speech_markers_are_dropped() {
        for marker in [
            "[BLANK_AUDIO]",
            " [BLANK_AUDIO]",
            "[ Silence ]",
            "[MUSIC PLAYING]",
            "(silence)",
            "(upbeat music)",
            "*sigh*",
            "\u{266a}",
            "[BLANK_AUDIO] (wind blowing)",
        ] {
            assert_eq!(spoken_text(marker), "", "{marker:?}");
        }
    }

    #[test]
    fn speech_around_markers_is_kept() {
        assert_eq!(spoken_text(" Hello there."), "Hello there.");
        assert_eq!(spoken_text("[BLANK_AUDIO] Hello [MUSIC] world"), "Hello world");
        assert_eq!(spoken_text("Call me (maybe) later"), "Call me (maybe) later");
        assert_eq!(spoken_text("Five * three"), "Five * three");
        assert_eq!(spoken_text("an open [bracket"), "an open [bracket");
        assert_eq!(spoken_text("Hello [ Silence ] world"), "Hello world");
        assert_eq!(spoken_text("* more words"), "* more words");
        assert_eq!(spoken_text("(and then"), "(and then");
    }
}
