//! Live preview: text in the pill while the user speaks (Settings, off by
//! default), as Handy shows. The final take is decoded as before; this only
//! shows what the model hears so far.
//!
//! A worker decodes the last `WINDOW_SECONDS` of the take about once a
//! `INTERVAL`, with the model the take will use (kept loaded, so each pass
//! is one decode). It stops when the take ends: `stop` joins it before the
//! final decode, so the two never hold the model at once, and the wait is at
//! most one short preview decode.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use tauri::{AppHandle, Manager};

const INTERVAL: Duration = Duration::from_millis(900);
/// The preview decodes only the most recent audio, which bounds each pass.
const WINDOW_SECONDS: f32 = 10.0;
/// Less than this much audio is not worth a pass.
const MINIMUM_SAMPLES: usize = 8_000;

static WORKER: Mutex<Option<(Arc<AtomicBool>, JoinHandle<()>)>> = Mutex::new(None);

/// Starts previewing take `session`, replacing any earlier worker.
pub fn start(app: AppHandle, session: u64) {
    stop();
    let stopping = Arc::new(AtomicBool::new(false));
    let flag = Arc::clone(&stopping);
    let Ok(handle) = std::thread::Builder::new()
        .name("vocawin-live-preview".into())
        .spawn(move || run(app, session, flag))
    else {
        return;
    };
    *WORKER.lock().unwrap_or_else(|e| e.into_inner()) = Some((stopping, handle));
}

/// Stops the worker and waits for a pass in flight to finish.
pub fn stop() {
    let worker = WORKER.lock().unwrap_or_else(|e| e.into_inner()).take();
    if let Some((stopping, handle)) = worker {
        stopping.store(true, Ordering::SeqCst);
        let _ = handle.join();
    }
}

fn run(app: AppHandle, session: u64, stopping: Arc<AtomicBool>) {
    let mut shown = String::new();
    loop {
        let wake = Instant::now() + INTERVAL;
        while Instant::now() < wake {
            if stopping.load(Ordering::SeqCst) {
                return;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        let state = app.state::<crate::AppState>();
        if state.session_id.load(Ordering::SeqCst) != session {
            return;
        }
        let Some((samples, rate)) = state.recorder.snapshot(WINDOW_SECONDS) else {
            return;
        };
        let pcm = crate::resample_to_16khz(&samples, rate);
        if pcm.len() < MINIMUM_SAMPLES
            || crate::silence::decide(&pcm) == crate::silence::Decision::NoSpeech
        {
            continue;
        }
        let settings = match state.settings.lock() {
            Ok(settings) => settings.clone(),
            Err(_) => return,
        };
        if stopping.load(Ordering::SeqCst) {
            return;
        }
        match crate::recognize(&state, &settings, pcm) {
            Ok(text) => {
                let text = text.trim();
                if !text.is_empty() && text != shown {
                    shown = text.to_string();
                    crate::overlay::live_text(&app, &shown);
                }
            }
            Err(error) => crate::logbuf::debug(format!("Live preview pass failed: {error}")),
        }
    }
}
