//! Low-level keyboard hook for dictation hotkeys on Windows.
//!
//! RegisterHotKey cannot bind a lone modifier. This WH_KEYBOARD_LL hook can see
//! VK_RCONTROL vs VK_RMENU and consumes matching keys so they do not leak.
//! Lone Right Alt is AltGr-safe: Ctrl+Right Alt (AltGr) is never consumed.
//!
//! Windows sends Alt as a SYSKEY. Down and up are matched from WM_KEY* and
//! WM_SYSKEY*, from LLKHF_UP, and from VK_MENU plus the extended bit (Right
//! Alt). Identity is the bound side only: Left Alt does not end a Right Alt
//! hold. Generic VK_MENU matches that bound side, never the other Alt.
//! Hold state is Idle or Recording (armed). First matching down while
//! Idle starts. Matching downs while Recording are typematic and swallow.
//! Matching up while Recording stops. Lost-up recovery is only the long
//! safety timeout (max recording + 5s). Do not poll GetAsyncKeyState on
//! the consumed Alt. A different vk (Ctrl for AltGr) is fine. Do not
//! SendInput from the hook callback; a synthetic unstick is queued on
//! vocawin-hotkey-actor after a real bound-side up.

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

/// Mac uses max recording + 5s. Default max is 60s, so 65s.
pub const DEFAULT_SAFETY_TIMEOUT: Duration = Duration::from_secs(65);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HookEvent {
    Pressed,
    Released,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum KeyEdge {
    Down,
    Up,
    Other,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum HoldAction {
    None,
    Start,
    /// Bound-side up. Unstick only this side, and only after a real up.
    Stop,
    /// Extra down while holding (Windows typematic). Eat it, keep the hold.
    Swallow,
}

/// OpenWhispr-style hold: idle until the first down, then recording
/// until the matching up. Typematic downs stay in Recording.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum HoldSession {
    Idle,
    Recording { vk: u32 },
}

impl HoldSession {
    fn armed(self) -> bool {
        matches!(self, HoldSession::Recording { .. })
    }

    fn held_vk(self) -> Option<u32> {
        match self {
            HoldSession::Idle => None,
            HoldSession::Recording { vk } => Some(vk),
        }
    }
}

struct HookShared {
    app: Option<AppHandle>,
    binding: Option<HotkeySpec>,
    /// True while Settings Record is capturing a new combo (Mac-style pause).
    capture_paused: bool,
    /// True while auto-pause apps are running.
    dictation_paused: bool,
    /// Listener is bound. Not the same as a live hold.
    listener_enabled: bool,
    session: HoldSession,
    hold_gen: u64,
    safety_timeout: Duration,
}

enum ActorMsg {
    Event(HookEvent),
    /// Bound-side key-up only, and only after the hook has returned.
    Unstick(u32),
}

static SHARED: OnceLock<Mutex<HookShared>> = OnceLock::new();
static HOOK_ACTIVE: AtomicBool = AtomicBool::new(false);
static ACTOR_TX: OnceLock<mpsc::Sender<ActorMsg>> = OnceLock::new();

fn shared() -> &'static Mutex<HookShared> {
    SHARED.get_or_init(|| {
        Mutex::new(HookShared {
            app: None,
            binding: None,
            capture_paused: false,
            dictation_paused: false,
            listener_enabled: false,
            session: HoldSession::Idle,
            hold_gen: 0,
            safety_timeout: DEFAULT_SAFETY_TIMEOUT,
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
    let _ = ACTOR_TX.set(tx);
    std::thread::Builder::new()
        .name("vocawin-hotkey-actor".into())
        .spawn(move || {
            while let Ok(msg) = rx.recv() {
                match msg {
                    ActorMsg::Event(event) => crate::on_hotkey_event(&app, event),
                    ActorMsg::Unstick(vk) => unstick_modifier(vk),
                }
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
    guard.listener_enabled = true;
}

pub fn set_safety_timeout(timeout: Duration) {
    let mut guard = shared().lock().unwrap_or_else(|e| e.into_inner());
    guard.safety_timeout = timeout;
}

pub fn clear_held_vk() {
    let mut guard = shared().lock().unwrap_or_else(|e| e.into_inner());
    guard.session = HoldSession::Idle;
    guard.hold_gen = guard.hold_gen.wrapping_add(1);
}

pub fn clear_binding() {
    let mut guard = shared().lock().unwrap_or_else(|e| e.into_inner());
    guard.binding = None;
    guard.listener_enabled = false;
    let held = guard.session.held_vk();
    guard.session = HoldSession::Idle;
    guard.hold_gen = guard.hold_gen.wrapping_add(1);
    drop(guard);
    if let Some(vk) = held {
        emit_released();
        queue_unstick(vk);
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
    if let Some(tx) = ACTOR_TX.get() {
        let _ = tx.send(ActorMsg::Event(event));
    }
}

fn emit_released() {
    emit(HookEvent::Released);
}

fn queue_unstick(vk: u32) {
    if let Some(tx) = ACTOR_TX.get() {
        let _ = tx.send(ActorMsg::Unstick(vk));
    }
}

fn arm_safety_timer(timeout: Duration, gen: u64) {
    std::thread::Builder::new()
        .name("vocawin-hotkey-safety".into())
        .spawn(move || {
            std::thread::sleep(timeout);
            let mut guard = shared().lock().unwrap_or_else(|e| e.into_inner());
            if guard.hold_gen != gen || !guard.session.armed() {
                return;
            }
            guard.session = HoldSession::Idle;
            drop(guard);
            emit_released();
        })
        .ok();
}

fn bump_hold_gen(guard: &mut HookShared) -> u64 {
    guard.hold_gen = guard.hold_gen.wrapping_add(1);
    guard.hold_gen
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
    let edge = classify_edge(wparam.0 as u32, flags);
    if edge == KeyEdge::Other {
        return unsafe { CallNextHookEx(None, code, wparam, lparam) };
    }

    let vk = resolve_vk(info.vkCode, flags & LLKHF_EXTENDED != 0);
    let consume = {
        let mut guard = match shared().lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };

        let altgr_blocks = edge == KeyEdge::Down
            && matches!(guard.binding, Some(HotkeySpec::Lone { vk: bound }) if bound == crate::hotkey::VK_RMENU)
            && ctrl_is_down();
        let action = hold_action(
            guard.session,
            vk,
            edge,
            guard.binding.as_ref(),
            altgr_blocks,
        );
        if action == HoldAction::None
            && combo_modifier_dropped(guard.binding.as_ref(), guard.session.held_vk(), vk, edge)
        {
            // Stay Recording so the later base key-up is still eaten.
            let _gen = bump_hold_gen(&mut guard);
            drop(guard);
            emit_released();
            return LRESULT(1);
        }

        match action {
            HoldAction::None => {
                return unsafe { CallNextHookEx(None, code, wparam, lparam) };
            }
            HoldAction::Swallow => {
                return LRESULT(1);
            }
            HoldAction::Start => {
                if guard.capture_paused || guard.dictation_paused || !guard.listener_enabled {
                    return unsafe { CallNextHookEx(None, code, wparam, lparam) };
                }
                guard.session = HoldSession::Recording { vk };
                let gen = bump_hold_gen(&mut guard);
                let timeout = guard.safety_timeout;
                drop(guard);
                emit(HookEvent::Pressed);
                arm_safety_timer(timeout, gen);
                true
            }
            HoldAction::Stop => {
                let held = guard.session.held_vk();
                guard.session = HoldSession::Idle;
                let _ = bump_hold_gen(&mut guard);
                drop(guard);
                emit_released();
                if let Some(held) = held {
                    queue_unstick(held);
                }
                true
            }
        }
    };

    if consume {
        return LRESULT(1);
    }

    unsafe { CallNextHookEx(None, code, wparam, lparam) }
}

fn classify_edge(wparam: u32, flags: u32) -> KeyEdge {
    if is_key_up(wparam, flags) {
        KeyEdge::Up
    } else if is_key_down(wparam, flags) {
        KeyEdge::Down
    } else {
        KeyEdge::Other
    }
}

fn bound_vk(binding: Option<&HotkeySpec>) -> Option<u32> {
    match binding {
        Some(HotkeySpec::Lone { vk }) | Some(HotkeySpec::Combo { vk, .. }) => Some(*vk),
        None => None,
    }
}

/// Generic VK_MENU matches the bound side only. Left Alt is not Right Alt.
fn vks_are_same_hold(held: u32, incoming: u32, bound: u32) -> bool {
    if held == incoming {
        return true;
    }
    (incoming == VK_MENU && held == bound) || (held == VK_MENU && incoming == bound)
}

fn is_same_hold(session: HoldSession, incoming: u32, binding: Option<&HotkeySpec>) -> bool {
    let Some(held) = session.held_vk() else {
        return false;
    };
    match bound_vk(binding) {
        Some(bound) => vks_are_same_hold(held, incoming, bound),
        None => held == incoming,
    }
}

fn hold_action(
    session: HoldSession,
    vk: u32,
    edge: KeyEdge,
    binding: Option<&HotkeySpec>,
    altgr_blocks_down: bool,
) -> HoldAction {
    match edge {
        KeyEdge::Other => HoldAction::None,
        KeyEdge::Up => {
            if session.armed() && is_same_hold(session, vk, binding) {
                HoldAction::Stop
            } else {
                HoldAction::None
            }
        }
        KeyEdge::Down => {
            if session.armed() {
                if is_same_hold(session, vk, binding) {
                    HoldAction::Swallow
                } else {
                    HoldAction::None
                }
            } else if altgr_blocks_down {
                HoldAction::None
            } else if binding.is_some_and(|spec| down_matches(spec, vk)) {
                HoldAction::Start
            } else {
                HoldAction::None
            }
        }
    }
}

fn combo_modifier_dropped(
    binding: Option<&HotkeySpec>,
    held: Option<u32>,
    vk: u32,
    edge: KeyEdge,
) -> bool {
    let Some(HotkeySpec::Combo {
        ctrl,
        alt,
        shift,
        vk: base,
    }) = binding
    else {
        return false;
    };
    if held != Some(*base) || edge != KeyEdge::Up {
        return false;
    }
    (*ctrl && is_ctrl_vk(vk)) || (*alt && is_alt_vk(vk)) || (*shift && is_shift_vk(vk))
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

fn down_matches(binding: &HotkeySpec, vk: u32) -> bool {
    match binding {
        HotkeySpec::Lone { vk: bound } => {
            if vk != *bound {
                return false;
            }
            // AltGr is Left Ctrl + Right Alt. Apply this only on key-down so a
            // Ctrl flicker cannot drop the matching key-up.
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

fn is_alt_vk(vk: u32) -> bool {
    vk == VK_MENU || vk == crate::hotkey::VK_LMENU || vk == crate::hotkey::VK_RMENU
}

fn is_ctrl_vk(vk: u32) -> bool {
    vk == VK_CONTROL as u32 || vk == crate::hotkey::VK_LCONTROL || vk == crate::hotkey::VK_RCONTROL
}

fn is_shift_vk(vk: u32) -> bool {
    vk == VK_SHIFT as u32 || vk == crate::hotkey::VK_LSHIFT || vk == crate::hotkey::VK_RSHIFT
}

fn ctrl_is_down() -> bool {
    #[cfg(windows)]
    {
        // Ctrl is a different vk from the consumed Right Alt. MSDN: async
        // state of an eaten key never updates, so never poll that key.
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

/// Inject a key-up for the bound side only. Call this from the actor
/// thread after the hook has returned, never from low_level_proc.
fn unstick_modifier(vk: u32) {
    #[cfg(windows)]
    {
        if vk == crate::hotkey::VK_RMENU {
            inject_key_up(crate::hotkey::VK_RMENU as u16, true);
        } else if vk == crate::hotkey::VK_LMENU {
            inject_key_up(crate::hotkey::VK_LMENU as u16, false);
        } else if vk == crate::hotkey::VK_RCONTROL {
            inject_key_up(vk as u16, true);
        } else if vk == crate::hotkey::VK_LCONTROL {
            inject_key_up(vk as u16, false);
        } else if vk == crate::hotkey::VK_RSHIFT || vk == crate::hotkey::VK_LSHIFT {
            inject_key_up(vk as u16, false);
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

    fn right_alt() -> HotkeySpec {
        HotkeySpec::Lone { vk: VK_RMENU }
    }

    #[test]
    fn syskey_and_keyup_flag_both_count_as_up() {
        assert!(is_key_up(WM_SYSKEYUP, 0));
        assert!(is_key_up(WM_KEYUP, 0));
        assert!(is_key_up(WM_SYSKEYDOWN, LLKHF_UP));
        assert!(is_key_down(WM_SYSKEYDOWN, 0));
        assert!(!is_key_down(WM_SYSKEYDOWN, LLKHF_UP));
        assert!(is_key_down(WM_KEYDOWN, 0));
        assert_eq!(classify_edge(WM_SYSKEYUP, 0), KeyEdge::Up);
        assert_eq!(classify_edge(WM_KEYUP, LLKHF_UP), KeyEdge::Up);
    }

    #[test]
    fn menu_plus_extended_is_right_alt() {
        assert_eq!(resolve_vk(VK_MENU, true), VK_RMENU);
        assert_eq!(resolve_vk(VK_MENU, false), VK_LMENU);
        assert_eq!(resolve_vk(VK_RMENU, true), VK_RMENU);
    }

    fn idle() -> HoldSession {
        HoldSession::Idle
    }

    fn recording() -> HoldSession {
        HoldSession::Recording { vk: VK_RMENU }
    }

    fn apply(session: HoldSession, vk: u32, edge: KeyEdge) -> HoldAction {
        hold_action(session, vk, edge, Some(&right_alt()), false)
    }

    #[test]
    fn left_alt_does_not_end_right_alt_hold() {
        assert_eq!(apply(recording(), VK_LMENU, KeyEdge::Up), HoldAction::None);
        assert_eq!(
            apply(recording(), VK_LMENU, KeyEdge::Down),
            HoldAction::None
        );
        assert_eq!(apply(recording(), VK_RMENU, KeyEdge::Up), HoldAction::Stop);
        assert_eq!(apply(recording(), VK_MENU, KeyEdge::Up), HoldAction::Stop);
    }

    #[test]
    fn generic_menu_matches_bound_side_only() {
        assert!(vks_are_same_hold(VK_RMENU, VK_RMENU, VK_RMENU));
        assert!(vks_are_same_hold(VK_RMENU, VK_MENU, VK_RMENU));
        assert!(vks_are_same_hold(VK_MENU, VK_RMENU, VK_RMENU));
        assert!(vks_are_same_hold(VK_LMENU, VK_MENU, VK_LMENU));
        assert!(!vks_are_same_hold(VK_RMENU, VK_LMENU, VK_RMENU));
        assert!(!vks_are_same_hold(VK_LMENU, VK_RMENU, VK_RMENU));
        assert!(!vks_are_same_hold(VK_LMENU, VK_MENU, VK_RMENU));
    }

    #[test]
    fn start_then_down_two_seconds_later_is_still_swallow() {
        assert_eq!(apply(idle(), VK_RMENU, KeyEdge::Down), HoldAction::Start);
        assert_eq!(
            apply(recording(), VK_RMENU, KeyEdge::Down),
            HoldAction::Swallow
        );
    }

    #[test]
    fn start_then_up_stops() {
        assert_eq!(apply(idle(), VK_RMENU, KeyEdge::Down), HoldAction::Start);
        assert_eq!(apply(recording(), VK_RMENU, KeyEdge::Up), HoldAction::Stop);
    }

    #[test]
    fn no_duplicate_press_events() {
        let binding = right_alt();
        let mut session = HoldSession::Idle;
        let mut starts = 0;
        let mut stops = 0;
        for _ in 0..40 {
            let action = hold_action(session, VK_RMENU, KeyEdge::Down, Some(&binding), false);
            match action {
                HoldAction::Start => {
                    starts += 1;
                    session = HoldSession::Recording { vk: VK_RMENU };
                }
                HoldAction::Swallow => {
                    assert!(session.armed());
                }
                other => panic!("unexpected down action {other:?}"),
            }
        }
        let up = hold_action(session, VK_RMENU, KeyEdge::Up, Some(&binding), false);
        assert_eq!(up, HoldAction::Stop);
        stops += 1;
        assert_eq!((starts, stops), (1, 1));
    }

    #[test]
    fn consumed_alt_repeat_is_not_treated_as_up() {
        assert_eq!(
            apply(recording(), VK_RMENU, KeyEdge::Down),
            HoldAction::Swallow
        );
        assert!(DEFAULT_SAFETY_TIMEOUT >= Duration::from_secs(60));
    }

    #[test]
    fn right_alt_down_starts_and_does_not_match_left() {
        let binding = right_alt();
        assert_eq!(apply(idle(), VK_RMENU, KeyEdge::Down), HoldAction::Start);
        assert_eq!(apply(idle(), VK_LMENU, KeyEdge::Down), HoldAction::None);
        assert!(!down_matches(&binding, VK_LMENU));
        assert!(down_matches(&binding, VK_RMENU) || cfg!(not(windows)));
    }

    #[test]
    fn altgr_filter_is_down_only() {
        let binding = right_alt();
        assert_eq!(
            hold_action(idle(), VK_RMENU, KeyEdge::Down, Some(&binding), true),
            HoldAction::None
        );
        assert_eq!(
            hold_action(recording(), VK_RMENU, KeyEdge::Up, Some(&binding), true),
            HoldAction::Stop
        );
    }
}
