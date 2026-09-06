//! Dictation output polish (parity with VocaMac DictationOutputFormatter)
//! and Windows text injection.
//!
//! Default insertion matches VocaMac (Accessibility first) and VocaLinux
//! (IBus/wtype first): type into the focused window and leave the clipboard
//! alone. Clipboard + Ctrl+V is the fallback, and that path restores the
//! previous clipboard unless the user opts into copy-to-clipboard.

pub fn append_trailing_space(text: &str) -> String {
    if text.is_empty() {
        return String::new();
    }
    if text.ends_with(char::is_whitespace) {
        return text.to_string();
    }
    format!("{text} ")
}

pub fn capitalize_sentences(text: &str) -> String {
    if text.is_empty() {
        return String::new();
    }
    let mut out = String::with_capacity(text.len());
    let mut capitalize_next = true;
    for ch in text.chars() {
        if capitalize_next && ch.is_ascii_lowercase() {
            out.push(ch.to_ascii_uppercase());
            capitalize_next = false;
            continue;
        }
        out.push(ch);
        if matches!(ch, '.' | '!' | '?') {
            capitalize_next = true;
        } else if !ch.is_whitespace() {
            capitalize_next = false;
        }
    }
    out
}

pub fn apply_output_polish(text: &str, auto_capitalize: bool, trailing_space: bool) -> String {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    let mut result = trimmed.to_string();
    if auto_capitalize {
        result = capitalize_sentences(&result);
    }
    if trailing_space {
        result = append_trailing_space(&result);
    }
    result
}

/// Controls whether dictation is also left on the system clipboard.
///
/// Matches VocaLinux `copy_to_clipboard` (default off) and VocaMac
/// `preserveClipboard` (default on): do not take over the clipboard unless
/// the user asks for it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct InjectOptions {
    pub copy_to_clipboard: bool,
}

impl InjectOptions {
    pub fn restore_clipboard(self) -> bool {
        !self.copy_to_clipboard
    }
}

pub fn inject(text: &str, options: InjectOptions) -> Result<(), String> {
    if text.is_empty() {
        return Ok(());
    }
    #[cfg(windows)]
    {
        inject_windows(text, options)
    }
    #[cfg(not(windows))]
    {
        let _ = options;
        Err("Text injection is available in Windows builds only.".into())
    }
}

#[cfg(windows)]
fn inject_windows(text: &str, options: InjectOptions) -> Result<(), String> {
    // Prefer SendInput so the default path never opens the clipboard.
    // Clipboard + Ctrl+V is the fallback (layout-independent, like VocaLinux
    // ydotool paste) and restores the previous clipboard unless the user
    // enabled copy-to-clipboard.
    if options.copy_to_clipboard {
        return match inject_via_clipboard(text, false) {
            Ok(()) => {
                crate::logbuf::debug("Injected via clipboard (copy-to-clipboard on).");
                Ok(())
            }
            Err(clipboard_error) => inject_send_input(text)
                .and_then(|_| write_clipboard_unicode(text))
                .map_err(|send_input_error| {
                    crate::logbuf::warn("Clipboard paste failed; SendInput also failed.");
                    format!(
                        "Clipboard paste failed ({clipboard_error}); SendInput also failed ({send_input_error})"
                    )
                }),
        };
    }
    match inject_send_input(text) {
        Ok(()) => {
            crate::logbuf::debug("Injected via SendInput.");
            Ok(())
        }
        Err(send_input_error) => inject_via_clipboard(text, true)
            .map(|()| {
                crate::logbuf::warn("SendInput failed; fell back to clipboard paste.");
            })
            .map_err(|clipboard_error| {
                crate::logbuf::error("SendInput and clipboard paste both failed.");
                format!(
                    "SendInput failed ({send_input_error}); clipboard paste also failed ({clipboard_error})"
                )
            }),
    }
}

#[cfg(windows)]
fn inject_send_input(text: &str) -> Result<(), String> {
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP, KEYEVENTF_UNICODE,
        VIRTUAL_KEY, VK_RETURN,
    };
    let mut inputs: Vec<INPUT> = Vec::new();
    let mut chars = text.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\r' {
            if chars.peek() == Some(&'\n') {
                let _ = chars.next();
            }
            inputs.extend([key_down(VK_RETURN), key_up(VK_RETURN)]);
            continue;
        }
        if ch == '\n' {
            inputs.extend([key_down(VK_RETURN), key_up(VK_RETURN)]);
            continue;
        }
        let mut units = [0u16; 2];
        for &unit in ch.encode_utf16(&mut units).iter() {
            inputs.extend([
                INPUT {
                    r#type: INPUT_KEYBOARD,
                    Anonymous: INPUT_0 {
                        ki: KEYBDINPUT {
                            wVk: VIRTUAL_KEY(0),
                            wScan: unit,
                            dwFlags: KEYEVENTF_UNICODE,
                            time: 0,
                            dwExtraInfo: 0,
                        },
                    },
                },
                INPUT {
                    r#type: INPUT_KEYBOARD,
                    Anonymous: INPUT_0 {
                        ki: KEYBDINPUT {
                            wVk: VIRTUAL_KEY(0),
                            wScan: unit,
                            dwFlags: KEYEVENTF_UNICODE | KEYEVENTF_KEYUP,
                            time: 0,
                            dwExtraInfo: 0,
                        },
                    },
                },
            ]);
        }
    }
    let sent = unsafe { SendInput(&inputs, std::mem::size_of::<INPUT>() as i32) };
    if sent as usize == inputs.len() {
        Ok(())
    } else {
        Err("Windows rejected SendInput".into())
    }
}

#[cfg(windows)]
const CF_UNICODETEXT: u32 = 13;

/// GDI clipboard formats that are not HGLOBAL and cannot be round-tripped
/// with GetClipboardData/SetClipboardData the same way text can.
#[cfg(windows)]
fn is_gdi_clipboard_format(format: u32) -> bool {
    matches!(
        format,
        2 | 3 | 9 | 14 | 0x0080 | 0x0082 | 0x0083 | 0x008E
    )
}

#[cfg(windows)]
#[derive(Clone, Default)]
struct ClipboardSnapshot {
    formats: Vec<(u32, Vec<u8>)>,
}

#[cfg(windows)]
#[derive(Clone)]
enum PendingRestore {
    Idle,
    Snapshot(ClipboardSnapshot),
    Failed,
}

#[cfg(windows)]
struct ClipboardRestoreState {
    generation: u64,
    pending: PendingRestore,
}

#[cfg(windows)]
fn clipboard_restore_state() -> &'static std::sync::Mutex<ClipboardRestoreState> {
    static STATE: std::sync::OnceLock<std::sync::Mutex<ClipboardRestoreState>> =
        std::sync::OnceLock::new();
    STATE.get_or_init(|| {
        std::sync::Mutex::new(ClipboardRestoreState {
            generation: 0,
            pending: PendingRestore::Idle,
        })
    })
}

#[cfg(windows)]
fn inject_via_clipboard(text: &str, restore: bool) -> Result<(), String> {
    use windows::Win32::UI::Input::KeyboardAndMouse::{SendInput, INPUT, VK_CONTROL, VK_V};

    let generation = {
        let mut state = clipboard_restore_state()
            .lock()
            .map_err(|_| "clipboard restore lock poisoned")?;
        state.generation = state.generation.wrapping_add(1);
        if restore {
            if matches!(state.pending, PendingRestore::Idle) {
                state.pending = match capture_clipboard_snapshot() {
                    Ok(snapshot) => PendingRestore::Snapshot(snapshot),
                    Err(_) => PendingRestore::Failed,
                };
            }
        } else {
            state.pending = PendingRestore::Idle;
        }
        state.generation
    };

    write_clipboard_unicode(text)?;

    let inputs = [
        key_down(VK_CONTROL),
        key_down(VK_V),
        key_up(VK_V),
        key_up(VK_CONTROL),
    ];
    let sent = unsafe { SendInput(&inputs, std::mem::size_of::<INPUT>() as i32) };
    if sent as usize != inputs.len() {
        if restore {
            restore_pending_clipboard(generation, None);
        }
        return Err("Ctrl+V SendInput failed".into());
    }
    // Match Mac TextInjector: give the target app time to consume Ctrl+V.
    std::thread::sleep(std::time::Duration::from_millis(150));
    if restore {
        restore_pending_clipboard(generation, Some(text));
    }
    Ok(())
}

#[cfg(windows)]
fn restore_pending_clipboard(generation: u64, expected_text: Option<&str>) {
    let pending = {
        let Ok(mut state) = clipboard_restore_state().lock() else {
            return;
        };
        if state.generation != generation {
            return;
        }
        std::mem::replace(&mut state.pending, PendingRestore::Idle)
    };
    // VocaLinux: if the user (or a clipboard manager) replaced our
    // transcription during the delay, leave that newer value alone.
    if let Some(text) = expected_text {
        if !clipboard_unicode_equals(text) {
            return;
        }
    }
    match pending {
        PendingRestore::Snapshot(snapshot) if snapshot.formats.is_empty() => {
            let _ = clear_clipboard();
        }
        PendingRestore::Snapshot(snapshot) => {
            let _ = restore_clipboard_snapshot(&snapshot);
        }
        PendingRestore::Failed | PendingRestore::Idle => {}
    }
}

#[cfg(windows)]
fn key_down(vk: windows::Win32::UI::Input::KeyboardAndMouse::VIRTUAL_KEY) -> windows::Win32::UI::Input::KeyboardAndMouse::INPUT {
    use windows::Win32::UI::Input::KeyboardAndMouse::{INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT};
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: vk,
                wScan: 0,
                dwFlags: Default::default(),
                time: 0,
                dwExtraInfo: 0,
            },
        },
    }
}

#[cfg(windows)]
fn key_up(vk: windows::Win32::UI::Input::KeyboardAndMouse::VIRTUAL_KEY) -> windows::Win32::UI::Input::KeyboardAndMouse::INPUT {
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP,
    };
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: vk,
                wScan: 0,
                dwFlags: KEYEVENTF_KEYUP,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    }
}

#[cfg(windows)]
fn open_clipboard_with_retry() -> Result<(), String> {
    use windows::Win32::Foundation::HWND;
    use windows::Win32::System::DataExchange::OpenClipboard;
    for _ in 0..10 {
        if unsafe { OpenClipboard(HWND::default()) }.is_ok() {
            return Ok(());
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    Err("OpenClipboard: busy".into())
}

#[cfg(windows)]
fn read_clipboard_unicode() -> Result<String, String> {
    use windows::Win32::System::DataExchange::{
        CloseClipboard, GetClipboardData, IsClipboardFormatAvailable,
    };
    use windows::Win32::System::Memory::{GlobalLock, GlobalUnlock};
    unsafe {
        if IsClipboardFormatAvailable(CF_UNICODETEXT).is_err() {
            return Err("no unicode clipboard".into());
        }
        open_clipboard_with_retry()?;
        let handle = match GetClipboardData(CF_UNICODETEXT) {
            Ok(handle) => handle,
            Err(error) => {
                let _ = CloseClipboard();
                return Err(format!("GetClipboardData: {error}"));
            }
        };
        let ptr = GlobalLock(windows::Win32::Foundation::HGLOBAL(handle.0)) as *const u16;
        if ptr.is_null() {
            let _ = CloseClipboard();
            return Err("clipboard lock failed".into());
        }
        let mut len = 0usize;
        while *ptr.add(len) != 0 {
            len += 1;
        }
        let slice = std::slice::from_raw_parts(ptr, len);
        let text = String::from_utf16_lossy(slice);
        let _ = GlobalUnlock(windows::Win32::Foundation::HGLOBAL(handle.0));
        let _ = CloseClipboard();
        Ok(text)
    }
}

#[cfg(windows)]
fn clipboard_unicode_equals(text: &str) -> bool {
    read_clipboard_unicode().is_ok_and(|current| current == text)
}

#[cfg(windows)]
fn write_clipboard_unicode(text: &str) -> Result<(), String> {
    use windows::core::HSTRING;
    use windows::Win32::Foundation::HANDLE;
    use windows::Win32::System::DataExchange::{CloseClipboard, EmptyClipboard, SetClipboardData};
    use windows::Win32::System::Memory::{GlobalAlloc, GlobalLock, GlobalUnlock, GMEM_MOVEABLE};
    let encoded: Vec<u16> = HSTRING::from(text).as_wide().iter().copied().chain([0]).collect();
    let bytes = encoded.len() * 2;
    unsafe {
        let mem = GlobalAlloc(GMEM_MOVEABLE, bytes)
            .map_err(|error| format!("clipboard alloc failed: {error}"))?;
        let ptr = GlobalLock(mem) as *mut u16;
        if ptr.is_null() {
            global_free(mem);
            return Err("clipboard lock failed".into());
        }
        std::ptr::copy_nonoverlapping(encoded.as_ptr(), ptr, encoded.len());
        let _ = GlobalUnlock(mem);
        if let Err(error) = open_clipboard_with_retry() {
            global_free(mem);
            return Err(error);
        }
        let result = (|| {
            EmptyClipboard().map_err(|error| format!("EmptyClipboard: {error}"))?;
            SetClipboardData(CF_UNICODETEXT, HANDLE(mem.0))
                .map_err(|error| format!("SetClipboardData: {error}"))?;
            Ok::<(), String>(())
        })();
        let _ = CloseClipboard();
        if result.is_err() {
            global_free(mem);
        }
        result
    }
}

/// windows 0.58 exports GlobalFree from Foundation, not System::Memory.
/// A successful free returns a null handle, which the crate reports as Err.
#[cfg(windows)]
unsafe fn global_free(mem: windows::Win32::Foundation::HGLOBAL) {
    let _ = windows::Win32::Foundation::GlobalFree(mem);
}

#[cfg(windows)]
fn clear_clipboard() -> Result<(), String> {
    use windows::Win32::System::DataExchange::{CloseClipboard, EmptyClipboard};
    open_clipboard_with_retry()?;
    let result = unsafe { EmptyClipboard() }.map_err(|error| format!("EmptyClipboard: {error}"));
    let _ = unsafe { CloseClipboard() };
    result.map(|_| ())
}

#[cfg(windows)]
fn capture_clipboard_snapshot() -> Result<ClipboardSnapshot, String> {
    use windows::Win32::System::DataExchange::{
        CloseClipboard, EnumClipboardFormats, GetClipboardData,
    };
    use windows::Win32::System::Memory::{GlobalLock, GlobalSize, GlobalUnlock};
    open_clipboard_with_retry()?;
    let result = (|| unsafe {
        let mut snapshot = ClipboardSnapshot::default();
        let mut format = 0u32;
        loop {
            format = EnumClipboardFormats(format);
            if format == 0 {
                break;
            }
            if is_gdi_clipboard_format(format) {
                continue;
            }
            let Ok(handle) = GetClipboardData(format) else {
                continue;
            };
            let mem = windows::Win32::Foundation::HGLOBAL(handle.0);
            let size = GlobalSize(mem);
            if size == 0 {
                continue;
            }
            let ptr = GlobalLock(mem) as *const u8;
            if ptr.is_null() {
                continue;
            }
            let bytes = std::slice::from_raw_parts(ptr, size).to_vec();
            let _ = GlobalUnlock(mem);
            snapshot.formats.push((format, bytes));
        }
        Ok(snapshot)
    })();
    let _ = unsafe { CloseClipboard() };
    result
}

#[cfg(windows)]
fn restore_clipboard_snapshot(snapshot: &ClipboardSnapshot) -> Result<(), String> {
    use windows::Win32::Foundation::HANDLE;
    use windows::Win32::System::DataExchange::{CloseClipboard, EmptyClipboard, SetClipboardData};
    use windows::Win32::System::Memory::{GlobalAlloc, GlobalLock, GlobalUnlock, GMEM_MOVEABLE};
    if snapshot.formats.is_empty() {
        return clear_clipboard();
    }
    open_clipboard_with_retry()?;
    let result = (|| unsafe {
        EmptyClipboard().map_err(|error| format!("EmptyClipboard: {error}"))?;
        for (format, bytes) in &snapshot.formats {
            let mem = GlobalAlloc(GMEM_MOVEABLE, bytes.len())
                .map_err(|error| format!("clipboard alloc failed: {error}"))?;
            let ptr = GlobalLock(mem) as *mut u8;
            if ptr.is_null() {
                global_free(mem);
                return Err("clipboard lock failed".into());
            }
            std::ptr::copy_nonoverlapping(bytes.as_ptr(), ptr, bytes.len());
            let _ = GlobalUnlock(mem);
            if SetClipboardData(*format, HANDLE(mem.0)).is_err() {
                global_free(mem);
            }
        }
        Ok(())
    })();
    let _ = unsafe { CloseClipboard() };
    result
}

/// Copy Debug log text (and other UI strings) to the system clipboard.
pub fn copy_to_clipboard(text: &str) -> Result<(), String> {
    #[cfg(windows)]
    {
        write_clipboard_unicode(text)
    }
    #[cfg(not(windows))]
    {
        let _ = text;
        Err("Clipboard copy is available in Windows builds only.".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capitalizes_sentences_like_mac() {
        assert_eq!(
            capitalize_sentences("hello world. next one! ok? yes"),
            "Hello world. Next one! Ok? Yes"
        );
    }

    #[test]
    fn trailing_space_skips_blank_and_existing() {
        assert_eq!(append_trailing_space(""), "");
        assert_eq!(append_trailing_space("hi "), "hi ");
        assert_eq!(append_trailing_space("hi"), "hi ");
    }

    #[test]
    fn polish_applies_in_order() {
        assert_eq!(
            apply_output_polish("hello. world", true, true),
            "Hello. World "
        );
    }

    #[test]
    fn clipboard_is_not_taken_over_by_default() {
        let options = InjectOptions::default();
        assert!(!options.copy_to_clipboard);
        assert!(options.restore_clipboard());
    }

    #[test]
    fn copy_to_clipboard_skips_restore() {
        let options = InjectOptions {
            copy_to_clipboard: true,
        };
        assert!(!options.restore_clipboard());
    }
}
