//! Whisper keep-alive cache with optional idle unload (opt-in).

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
        gpu_device: i32,
        keep_alive: bool,
        reply: mpsc::Sender<Result<String, String>>,
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
        gpu_device: i32,
        keep_alive: bool,
    ) -> Result<String, String> {
        let (reply, response) = mpsc::channel();
        self.commands
            .send(CacheCommand::Transcribe {
                model_path,
                pcm,
                language,
                use_gpu,
                gpu_device,
                keep_alive,
                reply,
            })
            .map_err(|_| "Whisper cache thread is not running".to_string())?;
        response
            .recv()
            .map_err(|_| "Whisper cache thread did not respond".to_string())?
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
    let mut context: Option<whisper_rs::WhisperContext> = None;
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
                gpu_device,
                keep_alive,
                reply,
            }) => {
                let result = run_transcribe(
                    &mut loaded_path,
                    &mut context,
                    &model_path,
                    &pcm,
                    language.as_deref(),
                    use_gpu,
                    gpu_device,
                    keep_alive,
                );
                loaded.store(context.is_some(), Ordering::Relaxed);
                if result.is_ok() {
                    last_used = Instant::now();
                }
                let _ = reply.send(result);
                false
            }
            Ok(CacheCommand::Unload) => {
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

fn run_transcribe(
    loaded_path: &mut Option<PathBuf>,
    context: &mut Option<whisper_rs::WhisperContext>,
    model_path: &PathBuf,
    pcm: &[f32],
    language: Option<&str>,
    use_gpu: bool,
    gpu_device: i32,
    keep_alive: bool,
) -> Result<String, String> {
    let needs_reload = context.is_none()
        || loaded_path
            .as_ref()
            .map(|path| path != model_path)
            .unwrap_or(true);
    if needs_reload {
        let mut context_params = whisper_rs::WhisperContextParameters::default();
        context_params.use_gpu(use_gpu);
        context_params.gpu_device(if gpu_device >= 0 { gpu_device } else { 0 });
        let next = whisper_rs::WhisperContext::new_with_params(
            model_path.to_string_lossy().as_ref(),
            context_params,
        )
        .map_err(|error| format!("Could not load Whisper model: {error}"))?;
        *context = Some(next);
        *loaded_path = Some(model_path.clone());
    }
    let ctx = context
        .as_ref()
        .ok_or("Whisper context missing after load")?;
    let mut session = ctx
        .create_state()
        .map_err(|error| format!("Could not create Whisper session: {error}"))?;
    let mut parameters =
        whisper_rs::FullParams::new(whisper_rs::SamplingStrategy::Greedy { best_of: 1 });
    parameters.set_translate(false);
    parameters.set_language(language);
    parameters.set_print_special(false);
    parameters.set_print_progress(false);
    parameters.set_print_realtime(false);
    parameters.set_print_timestamps(false);
    session
        .full(parameters, pcm)
        .map_err(|error| format!("Transcription failed: {error}"))?;
    let text = (0..session.full_n_segments())
        .filter_map(|index| {
            session
                .get_segment(index)
                .and_then(|segment| segment.to_str().ok().map(str::to_owned))
        })
        .collect::<Vec<_>>()
        .join(" ");
    if !keep_alive {
        *context = None;
        *loaded_path = None;
    }
    Ok(text)
}
