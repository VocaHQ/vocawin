//! Dictation output polish (parity with VocaMac DictationOutputFormatter)
//! and Windows text injection with a clipboard paste fallback.

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

#[cfg(windows)]
pub fn inject(text: &str) -> Result<(), String> {
    if text.is_empty() {
        return Ok(());
    }
    // Mac TextInjector path: clipboard + paste chord + restore. SendInput remains
    // as a fallback when the clipboard path cannot run.
    match inject_via_clipboard(text) {
        Ok(()) => Ok(()),
        Err(clipboard_error) => inject_send_input(text).map_err(|send_input_error| {
            format!(
                "Clipboard paste failed ({clipboard_error}); SendInput also failed ({send_input_error})"
            )
        }),
    }
}

#[cfg(not(windows))]
pub fn inject(_: &str) -> Result<(), String> {
    Err("Text injection is available in Windows builds only.".into())
}

#[cfg(windows)]
fn inject_send_input(text: &str) -> Result<(), String> {
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_UNICODE, VIRTUAL_KEY,
    };
    let inputs: Vec<INPUT> = text
        .encode_utf16()
        .flat_map(|unit| {
            [
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
                            dwFlags: KEYEVENTF_UNICODE
                                | windows::Win32::UI::Input::KeyboardAndMouse::KEYEVENTF_KEYUP,
                            time: 0,
                            dwExtraInfo: 0,
                        },
                    },
                },
            ]
        })
        .collect();
    let sent = unsafe { SendInput(&inputs, std::mem::size_of::<INPUT>() as i32) };
    if sent as usize == inputs.len() {
        Ok(())
    } else {
        Err("Windows rejected SendInput".into())
    }
}

#[cfg(windows)]
const CF_UNICODETEXT: u32 = 13;

#[cfg(windows)]
fn inject_via_clipboard(text: &str) -> Result<(), String> {
    use windows::core::HSTRING;
    use windows::Win32::Foundation::{HANDLE, HWND};
    use windows::Win32::System::DataExchange::{
        CloseClipboard, EmptyClipboard, OpenClipboard, SetClipboardData,
    };
    use windows::Win32::System::Memory::{GlobalAlloc, GlobalLock, GlobalUnlock, GMEM_MOVEABLE};
    use windows::Win32::UI::Input::KeyboardAndMouse::{SendInput, INPUT, VK_CONTROL, VK_V};

    // Preserve prior clipboard text when possible.
    let previous = read_clipboard_unicode().ok();
    let encoded: Vec<u16> = HSTRING::from(text).as_wide().iter().copied().chain([0]).collect();
    let bytes = encoded.len() * 2;
    unsafe {
        let mem = GlobalAlloc(GMEM_MOVEABLE, bytes)
            .map_err(|error| format!("clipboard alloc failed: {error}"))?;
        let ptr = GlobalLock(mem) as *mut u16;
        if ptr.is_null() {
            return Err("clipboard lock failed".into());
        }
        std::ptr::copy_nonoverlapping(encoded.as_ptr(), ptr, encoded.len());
        let _ = GlobalUnlock(mem);
        OpenClipboard(HWND::default()).map_err(|error| format!("OpenClipboard: {error}"))?;
        let result = (|| {
            EmptyClipboard().map_err(|error| format!("EmptyClipboard: {error}"))?;
            SetClipboardData(CF_UNICODETEXT, HANDLE(mem.0))
                .map_err(|error| format!("SetClipboardData: {error}"))?;
            Ok::<(), String>(())
        })();
        let _ = CloseClipboard();
        result?;
    }

    // Ctrl+V
    let inputs = [
        key_down(VK_CONTROL),
        key_down(VK_V),
        key_up(VK_V),
        key_up(VK_CONTROL),
    ];
    let sent = unsafe { SendInput(&inputs, std::mem::size_of::<INPUT>() as i32) };
    if sent as usize != inputs.len() {
        return Err("Ctrl+V SendInput failed".into());
    }
    // Match Mac TextInjector: give the target app time to consume Ctrl+V.
    std::thread::sleep(std::time::Duration::from_millis(150));
    if let Some(previous) = previous {
        let _ = write_clipboard_unicode(&previous);
    }
    Ok(())
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
fn read_clipboard_unicode() -> Result<String, String> {
    use windows::Win32::Foundation::HWND;
    use windows::Win32::System::DataExchange::{
        CloseClipboard, GetClipboardData, IsClipboardFormatAvailable, OpenClipboard,
    };
    use windows::Win32::System::Memory::{GlobalLock, GlobalUnlock};
    unsafe {
        if IsClipboardFormatAvailable(CF_UNICODETEXT).is_err() {
            return Err("no unicode clipboard".into());
        }
        OpenClipboard(HWND::default()).map_err(|error| format!("OpenClipboard: {error}"))?;
        let handle = GetClipboardData(CF_UNICODETEXT)
            .map_err(|error| format!("GetClipboardData: {error}"))?;
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
fn write_clipboard_unicode(text: &str) -> Result<(), String> {
    use windows::core::HSTRING;
    use windows::Win32::Foundation::{HANDLE, HWND};
    use windows::Win32::System::DataExchange::{
        CloseClipboard, EmptyClipboard, OpenClipboard, SetClipboardData,
    };
    use windows::Win32::System::Memory::{GlobalAlloc, GlobalLock, GlobalUnlock, GMEM_MOVEABLE};
    let encoded: Vec<u16> = HSTRING::from(text).as_wide().iter().copied().chain([0]).collect();
    let bytes = encoded.len() * 2;
    unsafe {
        let mem = GlobalAlloc(GMEM_MOVEABLE, bytes)
            .map_err(|error| format!("clipboard alloc failed: {error}"))?;
        let ptr = GlobalLock(mem) as *mut u16;
        if ptr.is_null() {
            return Err("clipboard lock failed".into());
        }
        std::ptr::copy_nonoverlapping(encoded.as_ptr(), ptr, encoded.len());
        let _ = GlobalUnlock(mem);
        OpenClipboard(HWND::default()).map_err(|error| format!("OpenClipboard: {error}"))?;
        let result = (|| {
            EmptyClipboard().map_err(|error| format!("EmptyClipboard: {error}"))?;
            SetClipboardData(CF_UNICODETEXT, HANDLE(mem.0))
                .map_err(|error| format!("SetClipboardData: {error}"))?;
            Ok::<(), String>(())
        })();
        let _ = CloseClipboard();
        result
    }
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
}
