//! In-process log buffer for the Debug pane. Also mirrors to stderr.

use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use tauri::{AppHandle, Emitter};

const CAPACITY: usize = 500;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Level {
    Debug,
    Info,
    Warn,
    Error,
}

impl Level {
    pub fn as_str(self) -> &'static str {
        match self {
            Level::Debug => "debug",
            Level::Info => "info",
            Level::Warn => "warn",
            Level::Error => "error",
        }
    }

    fn rank(self) -> u8 {
        match self {
            Level::Debug => 0,
            Level::Info => 1,
            Level::Warn => 2,
            Level::Error => 3,
        }
    }
}

#[derive(Clone, Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LogLine {
    pub level: String,
    pub text: String,
}

struct Entry {
    level: Level,
    text: String,
}

static LINES: Mutex<VecDeque<Entry>> = Mutex::new(VecDeque::new());
static DEBUG_ENABLED: AtomicBool = AtomicBool::new(false);

pub fn set_debug_enabled(enabled: bool) {
    DEBUG_ENABLED.store(enabled, Ordering::Relaxed);
}

pub fn debug_enabled() -> bool {
    DEBUG_ENABLED.load(Ordering::Relaxed)
}

pub fn push(line: impl Into<String>) {
    push_level(Level::Info, line);
}

pub fn debug(line: impl Into<String>) {
    push_level(Level::Debug, line);
}

pub fn warn(line: impl Into<String>) {
    push_level(Level::Warn, line);
}

pub fn error(line: impl Into<String>) {
    push_level(Level::Error, line);
}

pub fn push_level(level: Level, line: impl Into<String>) {
    let text = line.into();
    eprintln!("[{}] {text}", level.as_str());
    if let Ok(mut lines) = LINES.lock() {
        if lines.len() >= CAPACITY {
            lines.pop_front();
        }
        lines.push_back(Entry {
            level,
            text,
        });
    }
}

pub fn snapshot() -> Vec<LogLine> {
    snapshot_filtered(debug_enabled())
}

pub fn snapshot_filtered(include_debug: bool) -> Vec<LogLine> {
    let min = if include_debug {
        Level::Debug
    } else {
        Level::Warn
    };
    LINES
        .lock()
        .map(|lines| {
            lines
                .iter()
                .filter(|entry| entry.level.rank() >= min.rank())
                .map(|entry| LogLine {
                    level: entry.level.as_str().into(),
                    text: entry.text.clone(),
                })
                .collect()
        })
        .unwrap_or_default()
}

pub fn snapshot_text(include_debug: bool) -> String {
    snapshot_filtered(include_debug)
        .into_iter()
        .map(|line| format!("[{}] {}", line.level, line.text))
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn push_and_emit(app: &AppHandle, line: impl Into<String>) {
    push_level_and_emit(app, Level::Info, line);
}

pub fn push_level_and_emit(app: &AppHandle, level: Level, line: impl Into<String>) {
    let text = line.into();
    push_level(level, text.clone());
    let _ = app.emit(
        "log-line",
        LogLine {
            level: level.as_str().into(),
            text,
        },
    );
}

pub fn error_and_emit(app: &AppHandle, line: impl Into<String>) {
    push_level_and_emit(app, Level::Error, line);
}

pub fn warn_and_emit(app: &AppHandle, line: impl Into<String>) {
    push_level_and_emit(app, Level::Warn, line);
}

pub fn debug_and_emit(app: &AppHandle, line: impl Into<String>) {
    push_level_and_emit(app, Level::Debug, line);
}

pub fn clear() {
    if let Ok(mut lines) = LINES.lock() {
        lines.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_view_hides_debug_and_info() {
        clear();
        set_debug_enabled(false);
        push_level(Level::Debug, "mic opened");
        push_level(Level::Info, "ready");
        push_level(Level::Warn, "hotkey busy");
        push_level(Level::Error, "download failed");
        let shown = snapshot_filtered(false);
        assert_eq!(
            shown.iter().map(|line| line.text.as_str()).collect::<Vec<_>>(),
            vec!["hotkey busy", "download failed"]
        );
        let all = snapshot_filtered(true);
        assert_eq!(all.len(), 4);
        clear();
    }

    #[test]
    fn snapshot_text_keeps_levels() {
        clear();
        push_level(Level::Error, "boom");
        assert_eq!(snapshot_text(false), "[error] boom");
        clear();
    }
}
