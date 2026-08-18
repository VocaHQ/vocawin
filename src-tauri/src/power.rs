//! Sleep/wake recovery via WM_POWERBROADCAST on a message-only window.

use tauri::AppHandle;

pub fn start_sleep_wake_watcher(app: AppHandle, on_wake: impl Fn(AppHandle) + Send + 'static) {
    #[cfg(windows)]
    {
        std::thread::Builder::new()
            .name("vocawin-power".into())
            .spawn(move || {
                if let Err(error) = windows_power_loop(app, on_wake) {
                    eprintln!("VocaWin power watcher stopped: {error}");
                }
            })
            .ok();
    }
    #[cfg(not(windows))]
    {
        let _ = (app, on_wake);
    }
}

#[cfg(windows)]
fn windows_power_loop(
    app: AppHandle,
    on_wake: impl Fn(AppHandle) + Send + 'static,
) -> Result<(), String> {
    use std::sync::{mpsc, OnceLock};
    use windows::core::PCWSTR;
    use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
    use windows::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows::Win32::UI::WindowsAndMessaging::{
        CreateWindowExW, DefWindowProcW, DispatchMessageW, GetMessageW, RegisterClassW,
        TranslateMessage, CW_USEDEFAULT, HWND_MESSAGE, MSG, WINDOW_EX_STYLE, WINDOW_STYLE,
        WM_POWERBROADCAST, WNDCLASSW,
    };

    // From winuser.h / PBT_* when SystemServices feature set differs by crate rev.
    const PBT_APMRESUMESUSPEND: usize = 0x0007;
    const PBT_APMRESUMEAUTOMATIC: usize = 0x0012;

    static WAKE: OnceLock<mpsc::Sender<()>> = OnceLock::new();
    let (tx, rx) = mpsc::channel::<()>();
    let _ = WAKE.set(tx);

    std::thread::spawn(move || {
        while rx.recv().is_ok() {
            on_wake(app.clone());
        }
    });

    unsafe extern "system" fn wnd_proc(
        hwnd: HWND,
        msg: u32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> LRESULT {
        if msg == WM_POWERBROADCAST {
            let event = wparam.0;
            if event == PBT_APMRESUMEAUTOMATIC || event == PBT_APMRESUMESUSPEND {
                if let Some(tx) = WAKE.get() {
                    let _ = tx.send(());
                }
            }
        }
        unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
    }

    unsafe {
        let class_name: Vec<u16> = "VocaWinPower\0".encode_utf16().collect();
        let module = GetModuleHandleW(None).map_err(|error| error.to_string())?;
        let class = WNDCLASSW {
            lpfnWndProc: Some(wnd_proc),
            hInstance: module.into(),
            lpszClassName: PCWSTR(class_name.as_ptr()),
            ..Default::default()
        };
        let _ = RegisterClassW(&class);
        let hwnd = CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            PCWSTR(class_name.as_ptr()),
            PCWSTR(class_name.as_ptr()),
            WINDOW_STYLE::default(),
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            HWND_MESSAGE,
            None,
            module,
            None,
        )
        .map_err(|error| error.to_string())?;

        let mut msg = MSG::default();
        while GetMessageW(&mut msg, HWND::default(), 0, 0).into() {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
        let _ = hwnd;
    }
    Ok(())
}
