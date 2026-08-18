//! Low-level keyboard hook for dictation hotkeys on Windows.
//!
//! RegisterHotKey cannot bind a lone modifier. This WH_KEYBOARD_LL hook can see
//! VK_RCONTROL vs VK_RMENU and consumes matching keys so they do not leak.
//! Lone Right Alt is AltGr-safe: Ctrl+Right Alt (AltGr) is never consumed.

#![allow(dead_code)] // Hook symbols are Windows-only; Linux CI still typechecks the module.

use crate::hotkey::HotkeySpec;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::{Mutex, OnceLock};
use tauri::AppHandle;

// Aggregate modifier VKs used with GetAsyncKeyState for combos.
const VK_SHIFT: i32 = 0x10;
const VK_CONTROL: i32 = 0x11;
const VK_MENU: i32 = 0x12;
const LLKHF_INJECTED: u32 = 0x10;
const WH_KEYBOARD_LL: i32 = 13;
const WM_KEYDOWN: u32 = 0x0100;
const WM_KEYUP: u32 = 0x0101;
const WM_SYSKEYDOWN: u32 = 0x0104;
const WM_SYSKEYUP: u32 = 0x0105;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HookEvent {
    Pressed,
    Released,
}

struct HookShared {
    app: Option<AppHandle>,
    binding: Option<HotkeySpec>,
    /// True while Settings Record is capturing a new combo (Mac-style pause).
    capture_paused: bool,
    /// True while auto-pause apps are running.
    dictation_paused: bool,
    armed: bool,
}

static SHARED: OnceLock<Mutex<HookShared>> = OnceLock::new();
static HOOK_ACTIVE: AtomicBool = AtomicBool::new(false);
static EVENT_TX: OnceLock<mpsc::Sender<HookEvent>> = OnceLock::new();

fn shared() -> &'static Mutex<HookShared> {
    SHARED.get_or_init(|| {
        Mutex::new(HookShared {
            app: None,
            binding: None,
            capture_paused: false,
            dictation_paused: false,
            armed: false,
        })
    })
}

pub fn start(app: AppHandle) {
    {
        let mut guard = shared().lock().unwrap_or_else(|e| e.into_inner());
        guard.app = Some(app.clone());
    }
    if HOOK_ACTIVE.swap(true, Ordering::SeqCst) {
        return;
    }
    let (tx, rx) = mpsc::channel();
    let _ = EVENT_TX.set(tx);
    std::thread::Builder::new()
        .name("vocawin-hotkey-actor".into())
        .spawn(move || {
            while let Ok(event) = rx.recv() {
                crate::on_hotkey_event(&app, event);
            }
        })
        .ok();
    std::thread::Builder::new()
        .name("vocawin-hotkey".into())
        .spawn(|| {
            if let Err(error) = hook_thread_main() {
                eprintln!("VocaWin hotkey hook stopped: {error}");
            }
            HOOK_ACTIVE.store(false, Ordering::SeqCst);
        })
        .ok();
}

pub fn set_binding(spec: HotkeySpec) {
    let mut guard = shared().lock().unwrap_or_else(|e| e.into_inner());
    guard.binding = Some(spec);
    guard.armed = true;
}

pub fn clear_binding() {
    let mut guard = shared().lock().unwrap_or_else(|e| e.into_inner());
    guard.binding = None;
    guard.armed = false;
}

pub fn set_capture_paused(paused: bool) {
    let mut guard = shared().lock().unwrap_or_else(|e| e.into_inner());
    guard.capture_paused = paused;
}

pub fn set_dictation_paused(paused: bool) {
    let mut guard = shared().lock().unwrap_or_else(|e| e.into_inner());
    guard.dictation_paused = paused;
}

#[cfg(windows)]
fn hook_thread_main() -> Result<(), String> {
    use windows::Win32::Foundation::HINSTANCE;
    use windows::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows::Win32::UI::WindowsAndMessaging::{
        DispatchMessageW, GetMessageW, SetWindowsHookExW, TranslateMessage, UnhookWindowsHookEx,
        MSG, WINDOWS_HOOK_ID,
    };

    unsafe {
        let module = GetModuleHandleW(None).map_err(|error| error.to_string())?;
        let hook = SetWindowsHookExW(
            WINDOWS_HOOK_ID(WH_KEYBOARD_LL),
            Some(low_level_proc),
            HINSTANCE(module.0),
            0,
        )
        .map_err(|error| format!("Could not install keyboard hook: {error}"))?;

        let mut msg = MSG::default();
        while GetMessageW(&mut msg, windows::Win32::Foundation::HWND::default(), 0, 0).into() {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }

        let _ = UnhookWindowsHookEx(hook);
    }
    Ok(())
}

#[cfg(not(windows))]
fn hook_thread_main() -> Result<(), String> {
    Ok(())
}

#[cfg(windows)]
unsafe extern "system" fn low_level_proc(
    code: i32,
    wparam: windows::Win32::Foundation::WPARAM,
    lparam: windows::Win32::Foundation::LPARAM,
) -> windows::Win32::Foundation::LRESULT {
    use windows::Win32::Foundation::LRESULT;
    use windows::Win32::UI::WindowsAndMessaging::{CallNextHookEx, KBDLLHOOKSTRUCT};

    if code < 0 {
        return unsafe { CallNextHookEx(None, code, wparam, lparam) };
    }

    let info = unsafe { &*(lparam.0 as *const KBDLLHOOKSTRUCT) };
    if info.flags.0 & LLKHF_INJECTED != 0 {
        return unsafe { CallNextHookEx(None, code, wparam, lparam) };
    }

    let down = wparam.0 as u32 == WM_KEYDOWN || wparam.0 as u32 == WM_SYSKEYDOWN;
    let up = wparam.0 as u32 == WM_KEYUP || wparam.0 as u32 == WM_SYSKEYUP;
    if !down && !up {
        return unsafe { CallNextHookEx(None, code, wparam, lparam) };
    }

    let (should_handle, event) = {
        let guard = match shared().lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        if guard.capture_paused || guard.dictation_paused || !guard.armed {
            return unsafe { CallNextHookEx(None, code, wparam, lparam) };
        }
        let Some(binding) = guard.binding.as_ref() else {
            return unsafe { CallNextHookEx(None, code, wparam, lparam) };
        };
        if !key_matches(binding, info.vkCode) {
            return unsafe { CallNextHookEx(None, code, wparam, lparam) };
        }
        let event = if down {
            HookEvent::Pressed
        } else {
            HookEvent::Released
        };
        (true, event)
    };

    if should_handle {
        if let Some(tx) = EVENT_TX.get() {
            let _ = tx.send(event);
        }
        return LRESULT(1);
    }

    unsafe { CallNextHookEx(None, code, wparam, lparam) }
}

fn key_matches(binding: &HotkeySpec, vk: u32) -> bool {
    match binding {
        HotkeySpec::Lone { vk: bound } => {
            if vk != *bound {
                return false;
            }
            // AltGr is Left Ctrl + Right Alt on most Windows layouts. If we
            // consumed VK_RMENU then, characters like @/€ would never type.
            // Lone Right Alt (no Ctrl) is still a clean PTT, matching Linux.
            if *bound == crate::hotkey::VK_RMENU && ctrl_is_down() {
                return false;
            }
            true
        }
        HotkeySpec::Combo {
            ctrl,
            alt,
            shift,
            vk: bound,
        } => vk == *bound && mods_match(*ctrl, *alt, *shift),
    }
}

fn ctrl_is_down() -> bool {
    #[cfg(windows)]
    {
        use windows::Win32::UI::Input::KeyboardAndMouse::GetAsyncKeyState;
        (unsafe { GetAsyncKeyState(VK_CONTROL) } as u16) & 0x8000 != 0
    }
    #[cfg(not(windows))]
    {
        false
    }
}

fn mods_match(want_ctrl: bool, want_alt: bool, want_shift: bool) -> bool {
    #[cfg(windows)]
    {
        use windows::Win32::UI::Input::KeyboardAndMouse::GetAsyncKeyState;
        let ctrl_down = unsafe { GetAsyncKeyState(VK_CONTROL) } as u16 & 0x8000 != 0;
        let alt_down = unsafe { GetAsyncKeyState(VK_MENU) } as u16 & 0x8000 != 0;
        let shift_down = unsafe { GetAsyncKeyState(VK_SHIFT) } as u16 & 0x8000 != 0;
        ctrl_down == want_ctrl && alt_down == want_alt && shift_down == want_shift
    }
    #[cfg(not(windows))]
    {
        let _ = (want_ctrl, want_alt, want_shift);
        false
    }
}
