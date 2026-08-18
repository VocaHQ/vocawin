//! Start/stop dictation cues via Win32 Beep.

#[cfg(windows)]
mod win {
    #[link(name = "kernel32")]
    extern "system" {
        pub fn Beep(frequency: u32, duration_ms: u32) -> i32;
    }
}

#[cfg(windows)]
pub fn play_start() {
    unsafe {
        let _ = win::Beep(880, 70);
    }
}

#[cfg(windows)]
pub fn play_stop() {
    unsafe {
        let _ = win::Beep(520, 90);
    }
}

#[cfg(not(windows))]
pub fn play_start() {}

#[cfg(not(windows))]
pub fn play_stop() {}

pub fn play_if_enabled(enabled: bool, start: bool) {
    if !enabled {
        return;
    }
    if start {
        play_start();
    } else {
        play_stop();
    }
}
