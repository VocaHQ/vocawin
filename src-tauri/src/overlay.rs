//! The on-screen pill, VocaMac's recording overlay for Windows.
//!
//! One small always-on-top window (`overlay.html`) shows:
//! - a "ready" pill for a few seconds after launch, since Windows 11 tucks new
//!   tray icons into the overflow and people cannot tell VocaWin started;
//! - listening (live level), transcribing, and a short error or cancel note.
//!
//! The window must never take focus: text is typed into whatever app is in
//! front, and a focused pill would receive it instead. It is created
//! non-focusable (`WS_EX_NOACTIVATE`) and on Windows is shown with
//! `SWP_NOACTIVATE`, so the foreground window keeps focus.
//!
//! It is not made click-through: tao does that with `WS_EX_LAYERED`, which
//! does not mix with a transparent WebView2 window. Instead the window is
//! sized to the pill the page reports, so it covers nothing else, and a click
//! on a finished pill dismisses it.

use serde::Serialize;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, Manager, WebviewUrl, WebviewWindowBuilder};

pub const LABEL: &str = "overlay";
/// Logical height of the transparent window: the 44 px pill and its shadow.
const HEIGHT: f64 = 60.0;
/// Width before the page reports the pill's, and the bounds it may report.
const DEFAULT_WIDTH: u32 = 360;
const MIN_WIDTH: u32 = 120;
const MAX_WIDTH: u32 = 440;
/// Gap from the screen edge (logical pixels).
const MARGIN: f64 = 28.0;

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Payload {
    /// `ready`, `listening`, `processing`, `error`, `cancelled`, `notice`, or `hidden`.
    pub phase: String,
    pub title: String,
    pub detail: String,
}

impl Payload {
    fn hidden() -> Self {
        Self {
            phase: "hidden".into(),
            title: String::new(),
            detail: String::new(),
        }
    }
}

pub enum Phase {
    Ready {
        title: String,
        detail: String,
    },
    /// `hint` is shown beside the level ("Esc to cancel"), or empty.
    Listening {
        hint: String,
    },
    Processing,
    Error(String),
    Cancelled,
    /// A short note, such as "Nothing to paste yet".
    Notice(String),
}

static CURRENT: Mutex<Option<Payload>> = Mutex::new(None);
static GENERATION: AtomicU64 = AtomicU64::new(0);
/// Logical width of the pill the page last drew, plus its shadow.
static WIDTH: AtomicU32 = AtomicU32::new(DEFAULT_WIDTH);

/// What the overlay page renders when it loads after an update was sent.
pub fn current() -> Payload {
    CURRENT
        .lock()
        .ok()
        .and_then(|guard| guard.clone())
        .unwrap_or_else(Payload::hidden)
}

pub fn create(app: &AppHandle) -> tauri::Result<()> {
    let builder = WebviewWindowBuilder::new(app, LABEL, WebviewUrl::App("overlay.html".into()))
        .title("VocaWin status")
        .inner_size(DEFAULT_WIDTH as f64, HEIGHT)
        .decorations(false)
        .resizable(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .shadow(false)
        .focused(false)
        .focusable(false)
        .visible(false);
    #[cfg(not(target_os = "macos"))]
    let builder = builder.transparent(true);
    builder.build()?;
    Ok(())
}

/// The page measured its pill: fit the window to it and re-centre.
pub fn set_width(app: &AppHandle, width: f64) {
    let width = (width.round() as u32).clamp(MIN_WIDTH, MAX_WIDTH);
    if WIDTH.swap(width, Ordering::SeqCst) != width && current().phase != "hidden" {
        place_and_show(app);
    }
}

/// A click on the pill. Listening and transcribing stay up; the rest go.
pub fn dismiss(app: &AppHandle) {
    if !matches!(current().phase.as_str(), "listening" | "processing") {
        hide(app);
    }
}

/// Show a phase. `enabled` is the user's overlay setting for this phase.
pub fn show(app: &AppHandle, phase: Phase, enabled: bool) {
    if !enabled {
        hide(app);
        return;
    }
    let (payload, hide_after_ms) = match phase {
        Phase::Ready { title, detail } => (
            Payload {
                phase: "ready".into(),
                title,
                detail,
            },
            Some(3_600),
        ),
        Phase::Listening { hint } => (
            Payload {
                phase: "listening".into(),
                title: "Listening".into(),
                detail: hint,
            },
            None,
        ),
        Phase::Processing => (
            Payload {
                phase: "processing".into(),
                title: "Transcribing".into(),
                detail: String::new(),
            },
            None,
        ),
        Phase::Error(message) => (
            Payload {
                phase: "error".into(),
                title: "Dictation failed".into(),
                detail: short(&message),
            },
            Some(3_200),
        ),
        Phase::Cancelled => (
            Payload {
                phase: "cancelled".into(),
                title: "Cancelled".into(),
                detail: String::new(),
            },
            Some(1_100),
        ),
        Phase::Notice(message) => (
            Payload {
                phase: "notice".into(),
                title: short(&message),
                detail: String::new(),
            },
            Some(1_800),
        ),
    };
    let generation = GENERATION.fetch_add(1, Ordering::SeqCst) + 1;
    set_current(payload.clone());
    let _ = app.emit_to(LABEL, "overlay-phase", payload);
    place_and_show(app);
    if let Some(delay) = hide_after_ms {
        let app = app.clone();
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(delay));
            if GENERATION.load(Ordering::SeqCst) == generation {
                hide(&app);
            }
        });
    }
}

/// Live preview text for the listening pill; ignored in any other phase.
pub fn live_text(app: &AppHandle, text: &str) {
    if current().phase != "listening" {
        return;
    }
    let _ = app.emit_to(LABEL, "overlay-live", text.to_string());
}

pub fn hide(app: &AppHandle) {
    GENERATION.fetch_add(1, Ordering::SeqCst);
    set_current(Payload::hidden());
    let _ = app.emit_to(LABEL, "overlay-phase", Payload::hidden());
    let Some(window) = app.get_webview_window(LABEL) else {
        return;
    };
    let _ = app.run_on_main_thread(move || {
        #[cfg(windows)]
        if let Ok(hwnd) = window.hwnd() {
            native::hide(hwnd.0);
            return;
        }
        let _ = window.hide();
    });
}

fn set_current(payload: Payload) {
    if let Ok(mut guard) = CURRENT.lock() {
        *guard = Some(payload);
    }
}

/// One line, short enough for the pill.
fn short(message: &str) -> String {
    let line = message.lines().next().unwrap_or("").trim();
    if line.chars().count() <= 90 {
        return line.to_string();
    }
    let cut: String = line.chars().take(87).collect();
    format!("{}…", cut.trim_end())
}

/// Bottom or top centre of the work area on the monitor under the cursor
/// (where the user is working), in physical pixels.
fn frame(app: &AppHandle, top: bool) -> Option<(i32, i32, u32, u32)> {
    let cursor = app.cursor_position().ok()?;
    let monitor = app
        .monitor_from_point(cursor.x, cursor.y)
        .ok()
        .flatten()
        .or_else(|| app.primary_monitor().ok().flatten())?;
    let scale = monitor.scale_factor();
    let area = monitor.work_area();
    let width = (WIDTH.load(Ordering::SeqCst) as f64 * scale).round() as u32;
    let height = (HEIGHT * scale).round() as u32;
    let margin = (MARGIN * scale).round() as i32;
    let x = area.position.x + (area.size.width as i32 - width as i32) / 2;
    let y = if top {
        area.position.y + margin
    } else {
        area.position.y + area.size.height as i32 - height as i32 - margin
    };
    Some((x, y, width, height))
}

fn place_and_show(app: &AppHandle) {
    let Some(window) = app.get_webview_window(LABEL) else {
        return;
    };
    let top = app
        .try_state::<crate::AppState>()
        .and_then(|state| {
            state
                .settings
                .lock()
                .ok()
                .map(|settings| settings.overlay_position == "top")
        })
        .unwrap_or(false);
    let Some((x, y, width, height)) = frame(app, top) else {
        return;
    };
    let _ = app.run_on_main_thread(move || {
        #[cfg(windows)]
        if let Ok(hwnd) = window.hwnd() {
            native::show_at(hwnd.0, x, y, width as i32, height as i32);
            return;
        }
        let _ = window.set_size(tauri::PhysicalSize::new(width, height));
        let _ = window.set_position(tauri::PhysicalPosition::new(x, y));
        let _ = window.show();
    });
}

#[cfg(windows)]
mod native {
    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::WindowsAndMessaging::{
        SetWindowPos, ShowWindow, HWND_TOPMOST, SWP_NOACTIVATE, SWP_SHOWWINDOW, SW_HIDE,
    };

    /// Show without activating, so the app the user is typing into keeps focus.
    pub fn show_at(raw: *mut core::ffi::c_void, x: i32, y: i32, width: i32, height: i32) {
        unsafe {
            let _ = SetWindowPos(
                HWND(raw),
                HWND_TOPMOST,
                x,
                y,
                width,
                height,
                SWP_NOACTIVATE | SWP_SHOWWINDOW,
            );
        }
    }

    pub fn hide(raw: *mut core::ffi::c_void) {
        unsafe {
            let _ = ShowWindow(HWND(raw), SW_HIDE);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn long_errors_are_cut_to_one_short_line() {
        assert_eq!(short("Model failed\nsecond line"), "Model failed");
        let long = "x".repeat(200);
        let cut = short(&long);
        assert_eq!(cut.chars().count(), 88);
        assert!(cut.ends_with('…'));
    }

    #[test]
    fn nothing_is_shown_before_the_first_update() {
        assert_eq!(current().phase, "hidden");
    }
}
