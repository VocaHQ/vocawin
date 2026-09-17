//! Dictation output polish (parity with VocaMac DictationOutputFormatter)
//! and Windows text injection.
//!
//! Default insertion matches VocaMac (Accessibility first) and VocaLinux
//! (IBus/wtype first): type into the focused window and leave the clipboard
//! alone. Clipboard + Ctrl+V is the fallback, and that path restores the
//! previous clipboard unless the user opts into copy-to-clipboard.
//!
//! Notepad and WordPad are an exception: `KEYEVENTF_UNICODE` SendInput is
//! accepted (caret advances) but glyphs are dropped or blank, so those
//! targets prefer clipboard paste with restore. If the clipboard cannot
//! be fully restored, injection fails closed rather than reporting
//! SendInput success while the transcript never appears.

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

/// Classic Notepad / WordPad accept UNICODE SendInput (caret moves) but
/// drop or blank the glyphs. Clipboard Ctrl+V usually works.
const CLIPBOARD_INJECT_PROCESS_NAMES: &[&str] = &["notepad.exe", "wordpad.exe"];

/// Lowercase basename, ensure `.exe` — same shape as `autopause`.
fn normalize_process_name(name: &str) -> String {
    let trimmed = name.trim().trim_matches('"').to_ascii_lowercase();
    let file_name = trimmed
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or(&trimmed)
        .to_string();
    if file_name.ends_with(".exe") {
        file_name
    } else if file_name.is_empty() {
        file_name
    } else {
        format!("{file_name}.exe")
    }
}

fn prefers_clipboard_inject(process_name: &str) -> bool {
    CLIPBOARD_INJECT_PROCESS_NAMES.contains(&normalize_process_name(process_name).as_str())
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
    //
    // Notepad-like targets are the other way around: UNICODE SendInput
    // reports success but the app drops the glyphs, so paste first —
    // but only when the clipboard can be fully restored afterward.
    // GDI formats (bitmap / metafile / palette) cannot; EmptyClipboard
    // would drop them. Capture failure is the same: unknown must not
    // EmptyClipboard GDI. In both cases — and if paste itself fails —
    // do not fall through to SendInput (that would claim success while
    // dropping the transcript).
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
    if foreground_prefers_clipboard() {
        return inject_notepad_like(text);
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

/// Paste into Notepad/WordPad via clipboard+restore. Capture once and
/// reuse that snapshot; never treat UNICODE SendInput Ok as success.
#[cfg(windows)]
fn inject_notepad_like(text: &str) -> Result<(), String> {
    let captured = capture_clipboard_snapshot();
    match notepad_like_clipboard_decision(
        captured
            .as_ref()
            .map(|snapshot| snapshot.is_preservable())
            .map_err(|_| ()),
    ) {
        NotepadLikeClipboardDecision::PasteAndRestore => {
            let snapshot = match captured {
                Ok(snapshot) => snapshot,
                Err(_) => {
                    return Err(NOTEPAD_LIKE_CAPTURE_FAILED.into());
                }
            };
            match inject_via_clipboard_with_snapshot(text, snapshot) {
                Ok(()) => {
                    crate::logbuf::debug("Injected via clipboard (Notepad-like target).");
                    Ok(())
                }
                Err(clipboard_error) => {
                    crate::logbuf::warn(format!(
                        "Clipboard paste failed for Notepad-like target ({clipboard_error}); not falling back to SendInput (glyphs would drop)."
                    ));
                    Err(format!(
                        "Clipboard paste into Notepad/WordPad failed ({clipboard_error}). UNICODE SendInput would drop glyphs, so the transcript was not injected."
                    ))
                }
            }
        }
        NotepadLikeClipboardDecision::RejectUnpreservable => {
            crate::logbuf::warn(
                "Cannot inject into Notepad-like target: clipboard has unpreservable formats.",
            );
            Err(NOTEPAD_LIKE_UNPRESERVABLE.into())
        }
        NotepadLikeClipboardDecision::RejectCaptureFailed => {
            let detail = captured.err().unwrap_or_default();
            crate::logbuf::warn(format!(
                "Cannot inject into Notepad-like target: clipboard capture failed ({detail})."
            ));
            Err(NOTEPAD_LIKE_CAPTURE_FAILED.into())
        }
    }
}

#[cfg(windows)]
fn foreground_prefers_clipboard() -> bool {
    match foreground_process_name() {
        Some(name) => prefers_clipboard_inject(&name),
        None => false,
    }
}

#[cfg(windows)]
fn foreground_process_name() -> Option<String> {
    use windows::Win32::Foundation::{CloseHandle, HWND};
    use windows::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W,
        TH32CS_SNAPPROCESS,
    };
    use windows::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowThreadProcessId};

    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd == HWND::default() {
            return None;
        }
        let mut pid = 0u32;
        let _ = GetWindowThreadProcessId(hwnd, Some(&mut pid));
        if pid == 0 {
            return None;
        }
        let snap = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0).ok()?;
        let mut entry = PROCESSENTRY32W {
            dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32,
            cntUsage: 0,
            th32ProcessID: 0,
            th32DefaultHeapID: 0,
            th32ModuleID: 0,
            cntThreads: 0,
            th32ParentProcessID: 0,
            pcPriClassBase: 0,
            dwFlags: 0,
            szExeFile: [0; 260],
        };
        let mut name = None;
        if Process32FirstW(snap, &mut entry).is_ok() {
            loop {
                if entry.th32ProcessID == pid {
                    let len = entry
                        .szExeFile
                        .iter()
                        .position(|&c| c == 0)
                        .unwrap_or(entry.szExeFile.len());
                    let exe = String::from_utf16_lossy(&entry.szExeFile[..len]);
                    if !exe.is_empty() {
                        name = Some(exe);
                    }
                    break;
                }
                if Process32NextW(snap, &mut entry).is_err() {
                    break;
                }
            }
        }
        let _ = CloseHandle(snap);
        name
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
///
/// CF_BITMAP=2, CF_METAFILEPICT=3, CF_PALETTE=9, CF_ENHMETAFILE=14,
/// CF_OWNERDISPLAY=0x0080, CF_DSPBITMAP=0x0082, CF_DSPMETAFILEPICT=0x0083,
/// CF_DSPENHMETAFILE=0x008E.
fn is_gdi_clipboard_format(format: u32) -> bool {
    matches!(format, 2 | 3 | 9 | 14 | 0x0080 | 0x0082 | 0x0083 | 0x008E)
}

/// True when every enumerated format can be snapshotted and restored.
/// False if any GDI / unpreservable format is present.
fn clipboard_formats_are_preservable(formats: impl IntoIterator<Item = u32>) -> bool {
    formats
        .into_iter()
        .all(|format| !is_gdi_clipboard_format(format))
}

/// User-facing errors for the Notepad/WordPad prefer-clipboard path.
/// UNICODE SendInput reports success but those apps drop glyphs, so we
/// never claim Ok via SendInput when clipboard paste is unsafe or failed.
const NOTEPAD_LIKE_UNPRESERVABLE: &str = concat!(
    "Cannot inject into Notepad/WordPad: the clipboard has image or other ",
    "formats that cannot be restored after paste. Clear the clipboard or copy ",
    "text first, then try again.",
);

const NOTEPAD_LIKE_CAPTURE_FAILED: &str = concat!(
    "Cannot inject into Notepad/WordPad: the clipboard could not be captured, ",
    "so it cannot be restored after paste. Clear the clipboard or copy text ",
    "first, then try again.",
);

/// Generic restore-path reject (SendInput fallback). Notepad/WordPad uses
/// `NOTEPAD_LIKE_UNPRESERVABLE` instead so that path stays fail-closed.
const CLIPBOARD_RESTORE_UNPRESERVABLE: &str = concat!(
    "Clipboard has image or other formats that cannot be restored after paste; ",
    "refusing clipboard paste restore. Clear the clipboard or copy text first, ",
    "then try again.",
);

/// Outcome of the Notepad/WordPad clipboard-prefer path *before* paste.
/// `capture`: `Ok(true)` snapshot is fully preservable, `Ok(false)` has
/// unpreservable GDI formats, `Err` capture failed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum NotepadLikeClipboardDecision {
    PasteAndRestore,
    RejectUnpreservable,
    RejectCaptureFailed,
}

fn notepad_like_clipboard_decision(capture: Result<bool, ()>) -> NotepadLikeClipboardDecision {
    match capture {
        Ok(true) => NotepadLikeClipboardDecision::PasteAndRestore,
        Ok(false) => NotepadLikeClipboardDecision::RejectUnpreservable,
        Err(()) => NotepadLikeClipboardDecision::RejectCaptureFailed,
    }
}

impl NotepadLikeClipboardDecision {
    fn reject_message(self) -> Option<&'static str> {
        match self {
            Self::PasteAndRestore => None,
            Self::RejectUnpreservable => Some(NOTEPAD_LIKE_UNPRESERVABLE),
            Self::RejectCaptureFailed => Some(NOTEPAD_LIKE_CAPTURE_FAILED),
        }
    }
}

/// Empty captured formats means "clipboard was empty" only when no GDI
/// format was skipped. An incomplete snapshot must not EmptyClipboard.
fn should_clear_clipboard_on_restore(formats_empty: bool, skipped_unpreservable: bool) -> bool {
    formats_empty && !skipped_unpreservable
}

/// Restore-path paste must not overwrite the clipboard when the snapshot
/// skipped GDI formats. Callers reject before `write_clipboard_unicode`.
fn may_replace_clipboard_for_restore(skipped_unpreservable: bool) -> bool {
    !skipped_unpreservable
}

#[cfg(windows)]
#[derive(Clone, Default)]
struct ClipboardSnapshot {
    formats: Vec<(u32, Vec<u8>)>,
    /// Set when EnumClipboardFormats listed a GDI format we skipped.
    /// Restore cannot recover those; an empty `formats` vec is then
    /// incomplete rather than "clipboard was empty".
    skipped_unpreservable: bool,
}

#[cfg(windows)]
impl ClipboardSnapshot {
    fn is_preservable(&self) -> bool {
        !self.skipped_unpreservable
    }
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
    inject_via_clipboard_inner(text, restore, None)
}

/// Paste via clipboard and restore using a snapshot captured by the caller
/// so restore does not recapture (and does not copy every format twice).
#[cfg(windows)]
fn inject_via_clipboard_with_snapshot(
    text: &str,
    snapshot: ClipboardSnapshot,
) -> Result<(), String> {
    inject_via_clipboard_inner(text, true, Some(snapshot))
}

#[cfg(windows)]
fn inject_via_clipboard_inner(
    text: &str,
    restore: bool,
    snapshot: Option<ClipboardSnapshot>,
) -> Result<(), String> {
    use windows::Win32::UI::Input::KeyboardAndMouse::{SendInput, INPUT, VK_CONTROL, VK_V};

    let generation = {
        let mut state = clipboard_restore_state()
            .lock()
            .map_err(|_| "clipboard restore lock poisoned")?;
        if restore {
            if matches!(state.pending, PendingRestore::Idle) {
                let resolved = match snapshot {
                    Some(snapshot) => Ok(snapshot),
                    None => capture_clipboard_snapshot(),
                };
                match resolved {
                    Ok(snapshot) => {
                        // Fail closed before write: an unpreservable snapshot
                        // cannot be restored, and writing would destroy GDI.
                        if !may_replace_clipboard_for_restore(snapshot.skipped_unpreservable) {
                            return Err(CLIPBOARD_RESTORE_UNPRESERVABLE.into());
                        }
                        state.pending = PendingRestore::Snapshot(snapshot);
                    }
                    Err(_) => {
                        state.pending = PendingRestore::Failed;
                    }
                }
            }
        } else {
            state.pending = PendingRestore::Idle;
        }
        state.generation = state.generation.wrapping_add(1);
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
        PendingRestore::Snapshot(snapshot) => {
            let _ = restore_clipboard_snapshot(&snapshot);
        }
        PendingRestore::Failed | PendingRestore::Idle => {}
    }
}

#[cfg(windows)]
fn key_down(
    vk: windows::Win32::UI::Input::KeyboardAndMouse::VIRTUAL_KEY,
) -> windows::Win32::UI::Input::KeyboardAndMouse::INPUT {
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
fn key_up(
    vk: windows::Win32::UI::Input::KeyboardAndMouse::VIRTUAL_KEY,
) -> windows::Win32::UI::Input::KeyboardAndMouse::INPUT {
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
    let encoded: Vec<u16> = HSTRING::from(text)
        .as_wide()
        .iter()
        .copied()
        .chain([0])
        .collect();
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
                snapshot.skipped_unpreservable = true;
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
    if should_clear_clipboard_on_restore(
        snapshot.formats.is_empty(),
        snapshot.skipped_unpreservable,
    ) {
        return clear_clipboard();
    }
    if snapshot.formats.is_empty() {
        // Incomplete snapshot (GDI formats were skipped): do not
        // EmptyClipboard. Incomplete != empty.
        return Ok(());
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

    #[test]
    fn notepad_like_targets_prefer_clipboard_inject() {
        assert!(prefers_clipboard_inject("notepad.exe"));
        assert!(prefers_clipboard_inject("NOTEPAD.EXE"));
        assert!(prefers_clipboard_inject("Notepad"));
        assert!(prefers_clipboard_inject("wordpad.exe"));
        assert!(!prefers_clipboard_inject("chrome.exe"));
        assert!(!prefers_clipboard_inject("Code.exe"));
        assert!(!prefers_clipboard_inject("explorer.exe"));
        assert!(!prefers_clipboard_inject(""));
    }

    #[test]
    fn clipboard_without_gdi_formats_is_preservable() {
        const CF_TEXT: u32 = 1;
        const CF_DIB: u32 = 8;
        const CF_UNICODETEXT: u32 = 13;
        const CF_HDROP: u32 = 15;

        assert!(clipboard_formats_are_preservable([] as [u32; 0]));
        assert!(clipboard_formats_are_preservable([CF_UNICODETEXT]));
        assert!(clipboard_formats_are_preservable([
            CF_TEXT,
            CF_UNICODETEXT,
            CF_DIB,
            CF_HDROP
        ]));
        assert!(!is_gdi_clipboard_format(CF_UNICODETEXT));
        assert!(!is_gdi_clipboard_format(CF_DIB));
    }

    #[test]
    fn clipboard_with_gdi_formats_is_not_preservable() {
        const CF_TEXT: u32 = 1;
        const CF_BITMAP: u32 = 2;
        const CF_METAFILEPICT: u32 = 3;
        const CF_PALETTE: u32 = 9;
        const CF_UNICODETEXT: u32 = 13;
        const CF_ENHMETAFILE: u32 = 14;
        const CF_OWNERDISPLAY: u32 = 0x0080;
        const CF_DSPBITMAP: u32 = 0x0082;
        const CF_DSPMETAFILEPICT: u32 = 0x0083;
        const CF_DSPENHMETAFILE: u32 = 0x008E;

        for format in [
            CF_BITMAP,
            CF_METAFILEPICT,
            CF_PALETTE,
            CF_ENHMETAFILE,
            CF_OWNERDISPLAY,
            CF_DSPBITMAP,
            CF_DSPMETAFILEPICT,
            CF_DSPENHMETAFILE,
        ] {
            assert!(is_gdi_clipboard_format(format));
            assert!(!clipboard_formats_are_preservable([format]));
        }
        // Mixed: restore would drop the GDI object after EmptyClipboard.
        assert!(!clipboard_formats_are_preservable([
            CF_TEXT,
            CF_UNICODETEXT,
            CF_ENHMETAFILE
        ]));
        // GDI-only: formats vec is empty after skip; restore must not
        // treat that as "clipboard was empty" and EmptyClipboard.
        assert!(!clipboard_formats_are_preservable([
            CF_BITMAP,
            CF_ENHMETAFILE
        ]));
    }

    #[test]
    fn notepad_like_fails_closed_when_clipboard_cannot_be_restored() {
        assert_eq!(
            notepad_like_clipboard_decision(Ok(true)),
            NotepadLikeClipboardDecision::PasteAndRestore
        );
        assert_eq!(
            notepad_like_clipboard_decision(Ok(false)),
            NotepadLikeClipboardDecision::RejectUnpreservable
        );
        assert_eq!(
            notepad_like_clipboard_decision(Err(())),
            NotepadLikeClipboardDecision::RejectCaptureFailed
        );
        assert!(notepad_like_clipboard_decision(Ok(true))
            .reject_message()
            .is_none());

        let unpreservable = notepad_like_clipboard_decision(Ok(false))
            .reject_message()
            .expect("unpreservable must reject");
        assert!(
            unpreservable.contains("Clear the clipboard")
                && unpreservable.contains("copy text")
                && unpreservable.contains("try again")
        );

        let capture_failed = notepad_like_clipboard_decision(Err(()))
            .reject_message()
            .expect("capture failure must reject");
        assert!(
            capture_failed.contains("Clear the clipboard")
                && capture_failed.contains("copy text")
                && capture_failed.contains("try again")
        );
    }

    #[test]
    fn incomplete_clipboard_snapshot_must_not_clear_on_restore() {
        // Empty + fully captured: clipboard really was empty.
        assert!(should_clear_clipboard_on_restore(true, false));
        // Empty formats but GDI was skipped: incomplete != empty.
        assert!(!should_clear_clipboard_on_restore(true, true));
        // Non-empty HGLOBAL formats: restore those, do not clear.
        assert!(!should_clear_clipboard_on_restore(false, false));
        assert!(!should_clear_clipboard_on_restore(false, true));
    }

    #[test]
    fn unpreservable_snapshot_must_not_replace_clipboard_for_restore() {
        assert!(may_replace_clipboard_for_restore(false));
        assert!(!may_replace_clipboard_for_restore(true));
        assert!(
            CLIPBOARD_RESTORE_UNPRESERVABLE.contains("refusing clipboard paste restore")
                && !CLIPBOARD_RESTORE_UNPRESERVABLE.contains("Notepad")
        );
    }
}
