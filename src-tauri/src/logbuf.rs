//! In-process log buffer for the Logs view. Also mirrors to stderr.

use std::collections::VecDeque;
use std::sync::Mutex;
use tauri::{AppHandle, Emitter};

const CAPACITY: usize = 500;

static LINES: Mutex<VecDeque<String>> = Mutex::new(VecDeque::new());

pub fn push(line: impl Into<String>) {
    let line = line.into();
    eprintln!("{line}");
    if let Ok(mut lines) = LINES.lock() {
        if lines.len() >= CAPACITY {
            lines.pop_front();
        }
        lines.push_back(line);
    }
}

pub fn snapshot() -> Vec<String> {
    LINES
        .lock()
        .map(|lines| lines.iter().cloned().collect())
        .unwrap_or_default()
}

pub fn push_and_emit(app: &AppHandle, line: impl Into<String>) {
    let line = line.into();
    push(line.clone());
    let _ = app.emit("log-line", line);
}

pub fn clear() {
    if let Ok(mut lines) = LINES.lock() {
        lines.clear();
    }
}
