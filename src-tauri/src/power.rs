//! Sleep/wake recovery via RegisterSuspendResumeNotification.

use tauri::AppHandle;

pub fn start_sleep_wake_watcher(app: AppHandle, on_wake: impl Fn(AppHandle) + Send + 'static) {
    #[cfg(windows)]
    {
        std::thread::Builder::new()
            .name("vocawin-power".into())
            .spawn(move || windows_power_loop(app, on_wake))
            .ok();
    }
    #[cfg(not(windows))]
    {
        let _ = (app, on_wake);
    }
}

#[cfg(windows)]
fn windows_power_loop(app: AppHandle, on_wake: impl Fn(AppHandle) + Send + 'static) {
    use std::sync::mpsc;
    use windows::Win32::Foundation::HANDLE;
    use windows::Win32::System::Power::RegisterSuspendResumeNotification;
    use windows::Win32::System::SystemServices::DEVICE_NOTIFY_CALLBACK;
    use windows::Win32::UI::WindowsAndMessaging::DEVICE_NOTIFY_SUBSCRIBE_PARAMETERS;

    // winuser.h PBT_* values
    const PBT_APMRESUMESUSPEND: u32 = 0x0007;
    const PBT_APMRESUMEAUTOMATIC: u32 = 0x0012;

    struct Ctx {
        tx: mpsc::Sender<()>,
    }

    unsafe extern "system" fn power_callback(
        context: *const core::ffi::c_void,
        typ: u32,
        _setting: *const core::ffi::c_void,
    ) -> u32 {
        if typ == PBT_APMRESUMEAUTOMATIC || typ == PBT_APMRESUMESUSPEND {
            if !context.is_null() {
                let ctx = &*(context as *const Ctx);
                let _ = ctx.tx.send(());
            }
        }
        0
    }

    let (tx, rx) = mpsc::channel::<()>();
    let ctx = Box::new(Ctx { tx });
    let ctx_ptr = Box::into_raw(ctx);

    std::thread::spawn(move || {
        while rx.recv().is_ok() {
            on_wake(app.clone());
        }
    });

    unsafe {
        let mut params = DEVICE_NOTIFY_SUBSCRIBE_PARAMETERS {
            Callback: Some(power_callback),
            Context: ctx_ptr as *mut _,
        };
        match RegisterSuspendResumeNotification(
            HANDLE(&mut params as *mut _ as _),
            DEVICE_NOTIFY_CALLBACK,
        ) {
            Ok(_handle) => {
                // Keep this thread alive so the registration and context stay valid.
                loop {
                    std::thread::park();
                }
            }
            Err(error) => {
                eprintln!("VocaWin could not register suspend/resume notification: {error}");
                let _ = Box::from_raw(ctx_ptr);
            }
        }
    }
}
