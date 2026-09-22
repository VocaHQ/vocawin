//! Windows HKCU Run registration with a quoted executable path.

const START_MINIMIZED_ARG: &str = "--start-minimized";

/// Command written to HKCU Run. Always quote the exe; Windows will not start
/// an unquoted path that contains spaces (common in profile directories).
pub fn windows_run_command_line(exe_path: &str) -> String {
    format!("\"{exe_path}\" {START_MINIMIZED_ARG}")
}

#[cfg(windows)]
mod windows_run {
    use super::windows_run_command_line;
    use windows::core::PCWSTR;
    use windows::Win32::Foundation::{
        ERROR_FILE_NOT_FOUND, ERROR_PATH_NOT_FOUND, ERROR_SUCCESS, WIN32_ERROR,
    };
    use windows::Win32::System::Registry::{
        RegCloseKey, RegCreateKeyExW, RegDeleteValueW, RegOpenKeyExW, RegSetValueExW, HKEY,
        HKEY_CURRENT_USER, KEY_SET_VALUE, REG_BINARY, REG_OPTION_NON_VOLATILE, REG_SZ,
    };

    const RUN_VALUE_NAME: &str = "VocaWin";
    const LEGACY_RUN_VALUE_NAME: &str = "vocawin";
    const RUN_SUBKEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
    const STARTUP_APPROVED_SUBKEY: &str =
        r"Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\Run";
    const STARTUP_APPROVED_ENABLED: [u8; 12] = [
        0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    ];

    struct Key(HKEY);

    impl Drop for Key {
        fn drop(&mut self) {
            let _ = unsafe { RegCloseKey(self.0) };
        }
    }

    pub fn apply(enabled: bool) -> Result<(), String> {
        if enabled {
            enable()
        } else {
            disable()
        }
    }

    fn enable() -> Result<(), String> {
        let exe = std::env::current_exe().map_err(|error| format!("locate executable: {error}"))?;
        let command = windows_run_command_line(&exe.to_string_lossy());
        let run = create_hkcu(RUN_SUBKEY)?;
        set_sz(run.0, RUN_VALUE_NAME, &command)?;
        delete_value_ok_if_missing(run.0, LEGACY_RUN_VALUE_NAME)?;
        let approved = create_hkcu(STARTUP_APPROVED_SUBKEY)?;
        set_bytes(
            approved.0,
            RUN_VALUE_NAME,
            REG_BINARY,
            &STARTUP_APPROVED_ENABLED,
        )?;
        delete_value_ok_if_missing(approved.0, LEGACY_RUN_VALUE_NAME)?;
        Ok(())
    }

    fn disable() -> Result<(), String> {
        let Some(run) = open_hkcu(RUN_SUBKEY)? else {
            return Ok(());
        };
        delete_value_ok_if_missing(run.0, RUN_VALUE_NAME)?;
        delete_value_ok_if_missing(run.0, LEGACY_RUN_VALUE_NAME)?;
        Ok(())
    }

    fn create_hkcu(subkey: &str) -> Result<Key, String> {
        let subkey_w = wide(subkey);
        let mut hkey = HKEY::default();
        let status = unsafe {
            RegCreateKeyExW(
                HKEY_CURRENT_USER,
                PCWSTR(subkey_w.as_ptr()),
                0,
                PCWSTR::null(),
                REG_OPTION_NON_VOLATILE,
                KEY_SET_VALUE,
                None,
                &mut hkey,
                None,
            )
        };
        check("open registry key", status)?;
        Ok(Key(hkey))
    }

    fn open_hkcu(subkey: &str) -> Result<Option<Key>, String> {
        let subkey_w = wide(subkey);
        let mut hkey = HKEY::default();
        let status = unsafe {
            RegOpenKeyExW(
                HKEY_CURRENT_USER,
                PCWSTR(subkey_w.as_ptr()),
                0,
                KEY_SET_VALUE,
                &mut hkey,
            )
        };
        if is_missing(status) {
            return Ok(None);
        }
        check("open registry key", status)?;
        Ok(Some(Key(hkey)))
    }

    fn set_sz(hkey: HKEY, name: &str, value: &str) -> Result<(), String> {
        let data: Vec<u8> = wide(value).into_iter().flat_map(u16::to_le_bytes).collect();
        set_bytes(hkey, name, REG_SZ, &data)
    }

    fn set_bytes(
        hkey: HKEY,
        name: &str,
        value_type: windows::Win32::System::Registry::REG_VALUE_TYPE,
        data: &[u8],
    ) -> Result<(), String> {
        let name_w = wide(name);
        let status =
            unsafe { RegSetValueExW(hkey, PCWSTR(name_w.as_ptr()), 0, value_type, Some(data)) };
        check("set registry value", status)
    }

    fn delete_value_ok_if_missing(hkey: HKEY, name: &str) -> Result<(), String> {
        let name_w = wide(name);
        let status = unsafe { RegDeleteValueW(hkey, PCWSTR(name_w.as_ptr())) };
        if status == ERROR_SUCCESS || is_missing(status) {
            Ok(())
        } else {
            Err(format_status("delete Run value", status))
        }
    }

    fn wide(text: &str) -> Vec<u16> {
        text.encode_utf16().chain(std::iter::once(0)).collect()
    }

    fn is_missing(status: WIN32_ERROR) -> bool {
        status == ERROR_FILE_NOT_FOUND || status == ERROR_PATH_NOT_FOUND
    }

    fn check(op: &str, status: WIN32_ERROR) -> Result<(), String> {
        if status == ERROR_SUCCESS {
            Ok(())
        } else {
            Err(format_status(op, status))
        }
    }

    fn format_status(op: &str, status: WIN32_ERROR) -> String {
        format!(
            "{op}: {}",
            std::io::Error::from_raw_os_error(status.0 as i32)
        )
    }
}

#[cfg(windows)]
pub fn apply(enabled: bool) -> Result<(), String> {
    windows_run::apply(enabled)
}

#[cfg(test)]
mod tests {
    use super::windows_run_command_line;

    #[test]
    fn windows_run_command_line_quotes_paths_with_and_without_spaces() {
        let with_spaces = r"C:\Users\First Last\AppData\Local\Programs\VocaWin\vocawin.exe";
        let without_spaces = r"C:\Users\Alice\AppData\Local\Programs\VocaWin\vocawin.exe";
        assert_eq!(
            windows_run_command_line(with_spaces),
            r#""C:\Users\First Last\AppData\Local\Programs\VocaWin\vocawin.exe" --start-minimized"#
        );
        assert_eq!(
            windows_run_command_line(without_spaces),
            r#""C:\Users\Alice\AppData\Local\Programs\VocaWin\vocawin.exe" --start-minimized"#
        );
        for exe in [with_spaces, without_spaces, r"C:\VocaWin\vocawin.exe"] {
            let command = windows_run_command_line(exe);
            assert!(command.starts_with('"'), "exe must be quoted: {command}");
            assert!(
                command.ends_with("\" --start-minimized"),
                "quoted exe plus flag: {command}"
            );
            assert_ne!(command, format!("{exe} --start-minimized"));
        }
    }
}
