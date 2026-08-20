//! Low-level keyboard hook for dictation hotkeys on Windows.
//!
//! RegisterHotKey cannot bind a lone modifier. This WH_KEYBOARD_LL hook can see
//! VK_RCONTROL vs VK_RMENU and consumes matching keys so they do not leak.
//! Lone Right Alt is AltGr-safe: Ctrl+Right Alt (AltGr) is never consumed.
//!
//! Windows sends Alt as a SYSKEY. Down and up are matched from WM_KEY* and
//! WM_SYSKEY*, from LLKHF_UP, and from VK_MENU plus the extended bit (Right
//! Alt). If a key-up is lost, a hold watchdog and the next ordinary key recover
//! so the modifier does not stay logically down.

#![allow(dead_code)] // Hook symbols are Windows-only; Linux CI still typechecks the module.

use crate::hotkey::HotkeySpec;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::{Mutex, OnceLock};
use std::time::Duration;
use tauri::AppHandle;

// Aggregate modifier VKs used with GetAsyncKeyState for combos.
const VK_SHIFT: i32 = 0x10;
const VK_CONTROL: i32 = 0x11;
const VK_MENU: u32 = 0x12;
const LLKHF_EXTENDED: u32 = 0x01;
const LLKHF_INJECTED: u32 = 0x10;
const LLKHF_UP: u32 = 0x80;
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
    /// VK we consumed on key-down and still owe a release for.
    held_vk: Option<u32>,
}

static SHARED: OnceLock<Mutex<HookShared>> = OnceLock::new();
static HOOK_ACTIVE: AtomicBool = AtomicBool::new(false);
static WATCHDOG_ACTIVE: AtomicBool = AtomicBool::new(false);
static EVENT_TX: OnceLock<mpsc::Sender<HookEvent>> = OnceLock::new();

fn shared() -> &'static Mutex<HookShared> {
    SHARED.get_or_init(|| {
        Mutex::new(HookShared {
            app: None,
            binding: None,
            capture_paused: false,
            dictation_paused: false,
            armed: false,
            held_vk: None,
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
    start_hold_watchdog();
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
    let held = guard.held_vk.take();
    drop(guard);
    if let Some(vk) = held {
        emit_released();
        unstick_modifier(vk);
    }
}

pub fn set_capture_paused(paused: bool) {
    let mut guard = shared().lock().unwrap_or_else(|e| e.into_inner());
    guard.capture_paused = paused;
}

pub fn set_dictation_paused(paused: bool) {
    let mut guard = shared().lock().unwrap_or_else(|e| e.into_inner());
    guard.dictation_paused = paused;
}

fn emit(event: HookEvent) {
    if let Some(tx) = EVENT_TX.get() {
        let _ = tx.send(event);
    }
}

fn emit_released() {
    emit(HookEvent::Released);
}

fn start_hold_watchdog() {
    if WATCHDOG_ACTIVE.swap(true, Ordering::SeqCst) {
        return;
    }
    std::thread::Builder::new()
        .name("vocawin-hotkey-hold".into())
        .spawn(|| {
            let mut misses = 0u8;
            loop {
                std::thread::sleep(Duration::from_millis(50));
                let held = {
                    let guard = shared().lock().unwrap_or_else(|e| e.into_inner());
                    guard.held_vk
                };
                let Some(vk) = held else {
                    misses = 0;
                    continue;
                };
                if physical_key_down(vk) {
                    misses = 0;
                    continue;
                }
                misses = misses.saturating_add(1);
                if misses >= 3 {
                    recover_lost_up(vk);
                    misses = 0;
                }
            }
        })
        .ok();
}

fn recover_lost_up(vk: u32) {
    let mut guard = shared().lock().unwrap_or_else(|e| e.into_inner());
    if guard.held_vk != Some(vk) {
        return;
    }
    guard.held_vk = None;
    drop(guard);
    emit_released();
    unstick_modifier(vk);
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

    let flags = info.flags.0;
    let down = is_key_down(wparam.0 as u32, flags);
    let up = is_key_up(wparam.0 as u32, flags);
    if !down && !up {
        return unsafe { CallNextHookEx(None, code, wparam, lparam) };
    }

    let vk = resolve_vk(info.vkCode, flags & LLKHF_EXTENDED != 0);
    let consume = {
        let mut guard = match shared().lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };

        if up {
            if let Some(held) = guard.held_vk {
                if vks_are_same_hold(held, vk) {
                    guard.held_vk = None;
                    drop(guard);
                    emit_released();
                    unstick_modifier(held);
                    return LRESULT(1);
                }
            }
            return unsafe { CallNextHookEx(None, code, wparam, lparam) };
        }

        if let Some(held) = guard.held_vk {
            if vks_are_same_hold(held, vk) {
                return LRESULT(1);
            }
            if !physical_key_down(held) {
                guard.held_vk = None;
                drop(guard);
                emit_released();
                unstick_modifier(held);
                return unsafe { CallNextHookEx(None, code, wparam, lparam) };
            }
        }

        if guard.capture_paused || guard.dictation_paused || !guard.armed {
            return unsafe { CallNextHookEx(None, code, wparam, lparam) };
        }
        let Some(binding) = guard.binding.clone() else {
            return unsafe { CallNextHookEx(None, code, wparam, lparam) };
        };
        if !down_matches(&binding, vk) {
            return unsafe { CallNextHookEx(None, code, wparam, lparam) };
        }
        guard.held_vk = Some(vk);
        drop(guard);
        emit(HookEvent::Pressed);
        true
    };

    if consume {
        return LRESULT(1);
    }

    unsafe { CallNextHookEx(None, code, wparam, lparam) }
}

fn is_key_down(wparam: u32, flags: u32) -> bool {
    (wparam == WM_KEYDOWN || wparam == WM_SYSKEYDOWN) && flags & LLKHF_UP == 0
}

fn is_key_up(wparam: u32, flags: u32) -> bool {
    wparam == WM_KEYUP || wparam == WM_SYSKEYUP || flags & LLKHF_UP != 0
}

/// Map generic VK_MENU onto the side Windows meant (extended = Right Alt).
fn resolve_vk(vk: u32, extended: bool) -> u32 {
    if vk == VK_MENU {
        if extended {
            crate::hotkey::VK_RMENU
        } else {
            crate::hotkey::VK_LMENU
        }
    } else {
        vk
    }
}

fn vks_are_same_hold(held: u32, vk: u32) -> bool {
    if held == vk {
        return true;
    }
    matches!(
        (held, vk),
        (crate::hotkey::VK_RMENU, crate::hotkey::VK_LMENU)
            | (crate::hotkey::VK_LMENU, crate::hotkey::VK_RMENU)
            | (crate::hotkey::VK_RMENU, VK_MENU)
            | (crate::hotkey::VK_LMENU, VK_MENU)
            | (VK_MENU, crate::hotkey::VK_RMENU)
            | (VK_MENU, crate::hotkey::VK_LMENU)
    ) || (is_alt_vk(held) && is_alt_vk(vk))
}

fn is_alt_vk(vk: u32) -> bool {
    vk == VK_MENU || vk == crate::hotkey::VK_LMENU || vk == crate::hotkey::VK_RMENU
}

fn down_matches(binding: &HotkeySpec, vk: u32) -> bool {
    match binding {
        HotkeySpec::Lone { vk: bound } => {
            if !lone_vk_matches(*bound, vk) {
                return false;
            }
            // AltGr is Left Ctrl + Right Alt on most Windows layouts. Apply
            // this only on key-down so a Ctrl flicker cannot drop the matching
            // key-up and leave Alt logically held.
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

fn lone_vk_matches(bound: u32, vk: u32) -> bool {
    if vk == bound {
        return true;
    }
    if is_alt_vk(bound) {
        return is_alt_vk(vk) && (bound == vk || vk == VK_MENU || bound == VK_MENU);
    }
    false
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
        let alt_down = unsafe { GetAsyncKeyState(VK_MENU as i32) } as u16 & 0x8000 != 0;
        let shift_down = unsafe { GetAsyncKeyState(VK_SHIFT) } as u16 & 0x8000 != 0;
        ctrl_down == want_ctrl && alt_down == want_alt && shift_down == want_shift
    }
    #[cfg(not(windows))]
    {
        let _ = (want_ctrl, want_alt, want_shift);
        false
    }
}

fn physical_key_down(vk: u32) -> bool {
    #[cfg(windows)]
    {
        use windows::Win32::UI::Input::KeyboardAndMouse::GetAsyncKeyState;
        let down = |code: i32| (unsafe { GetAsyncKeyState(code) } as u16) & 0x8000 != 0;
        if is_alt_vk(vk) {
            return down(VK_MENU as i32)
                || down(crate::hotkey::VK_LMENU as i32)
                || down(crate::hotkey::VK_RMENU as i32);
        }
        down(vk as i32)
    }
    #[cfg(not(windows))]
    {
        let _ = vk;
        false
    }
}

fn unstick_modifier(vk: u32) {
    #[cfg(windows)]
    {
        if is_alt_vk(vk) {
            inject_key_up(crate::hotkey::VK_RMENU as u16, true);
            inject_key_up(crate::hotkey::VK_LMENU as u16, false);
            inject_key_up(VK_MENU as u16, false);
        } else if vk == crate::hotkey::VK_RCONTROL || vk == crate::hotkey::VK_LCONTROL {
            inject_key_up(vk as u16, vk == crate::hotkey::VK_RCONTROL);
            inject_key_up(VK_CONTROL as u16, false);
        } else if vk == crate::hotkey::VK_RSHIFT || vk == crate::hotkey::VK_LSHIFT {
            inject_key_up(vk as u16, false);
            inject_key_up(VK_SHIFT as u16, false);
        }
    }
    #[cfg(not(windows))]
    {
        let _ = vk;
    }
}

#[cfg(windows)]
fn inject_key_up(vk: u16, extended: bool) {
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_EXTENDEDKEY,
        KEYEVENTF_KEYUP, VIRTUAL_KEY,
    };
    let mut flags = KEYEVENTF_KEYUP;
    if extended {
        flags |= KEYEVENTF_EXTENDEDKEY;
    }
    let input = INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: VIRTUAL_KEY(vk),
                wScan: 0,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    };
    unsafe {
        let _ = SendInput(&[input], std::mem::size_of::<INPUT>() as i32);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hotkey::{VK_LMENU, VK_RMENU};

    #[test]
    fn syskey_and_keyup_flag_both_count_as_up() {
        assert!(is_key_up(WM_SYSKEYUP, 0));
        assert!(is_key_up(WM_KEYUP, 0));
        assert!(is_key_up(WM_SYSKEYDOWN, LLKHF_UP));
        assert!(is_key_down(WM_SYSKEYDOWN, 0));
        assert!(!is_key_down(WM_SYSKEYDOWN, LLKHF_UP));
        assert!(is_key_down(WM_KEYDOWN, 0));
    }

    #[test]
    fn menu_plus_extended_is_right_alt() {
        assert_eq!(resolve_vk(VK_MENU, true), VK_RMENU);
        assert_eq!(resolve_vk(VK_MENU, false), VK_LMENU);
        assert_eq!(resolve_vk(VK_RMENU, true), VK_RMENU);
    }

    #[test]
    fn right_alt_hold_matches_menu_alias() {
        assert!(vks_are_same_hold(VK_RMENU, VK_MENU));
        assert!(vks_are_same_hold(VK_RMENU, VK_RMENU));
        assert!(lone_vk_matches(VK_RMENU, VK_RMENU));
        let binding = HotkeySpec::Lone { vk: VK_RMENU };
        assert!(down_matches(&binding, VK_RMENU));
    }
}
