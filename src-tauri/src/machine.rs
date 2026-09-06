//! Best-effort machine readout for the Debug pane and copyable report.

use crate::gpu::GpuStatus;
use serde::Serialize;

const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DebugReport {
    pub version: String,
    pub os: String,
    pub cpu: String,
    pub ram: String,
    pub gpu: GpuStatus,
    pub debug_logging: bool,
    pub text: String,
}

pub fn debug_report(gpu: GpuStatus, debug_logging: bool, logs: &str) -> DebugReport {
    let os = os_name();
    let cpu = cpu_name();
    let ram = ram_summary();
    let text = format_report(APP_VERSION, &os, &cpu, &ram, &gpu, debug_logging, logs);
    DebugReport {
        version: APP_VERSION.into(),
        os,
        cpu,
        ram,
        gpu,
        debug_logging,
        text,
    }
}

fn format_report(
    version: &str,
    os: &str,
    cpu: &str,
    ram: &str,
    gpu: &GpuStatus,
    debug_logging: bool,
    logs: &str,
) -> String {
    let gpu_kind = if !gpu.available {
        gpu.backend.clone()
    } else if gpu.discrete {
        "discrete".into()
    } else {
        "integrated".into()
    };
    let gpu_line = if gpu.vram_mb > 0 {
        format!("{} ({gpu_kind}, ~{} MB)", gpu.name, gpu.vram_mb)
    } else {
        format!("{} ({gpu_kind})", gpu.name)
    };
    let flag = if debug_logging { "on" } else { "off" };
    let mut body = format!(
        "VocaWin {version}\n\
OS: {os}\n\
CPU: {cpu}\n\
RAM: {ram}\n\
GPU: {gpu_line}\n\
GPU backend: {}\n\
Debug logging: {flag}\n",
        gpu.backend
    );
    body.push('\n');
    if logs.trim().is_empty() {
        body.push_str("No log lines.\n");
    } else {
        body.push_str(&redact_home_paths(logs.trim_end()));
        body.push('\n');
    }
    body
}

/// The report is meant to be pasted into a GitHub issue. Strip profile
/// directories so a log line with an APPDATA model path does not leak a
/// Windows user name. Users/home are matched case-insensitively with `/` or `\`.
fn redact_home_paths(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut rest = text;
    while let Some((idx, prefix_len)) = find_profile_prefix(rest) {
        result.push_str(&rest[..idx + prefix_len]);
        let after = &rest[idx + prefix_len..];
        let end = username_end(after);
        if end > 0 {
            result.push_str("<user>");
            rest = &after[end..];
        } else {
            rest = after;
        }
    }
    result.push_str(rest);
    result
}

fn find_profile_prefix(text: &str) -> Option<(usize, usize)> {
    let bytes = text.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if let Some(prefix_len) = profile_prefix_len(&bytes[i..]) {
            return Some((i, prefix_len));
        }
        i += text[i..].chars().next()?.len_utf8();
    }
    None
}

fn profile_prefix_len(bytes: &[u8]) -> Option<usize> {
    let sep = *bytes.first()?;
    if sep != b'/' && sep != b'\\' {
        return None;
    }
    for marker in [b"users".as_slice(), b"home".as_slice()] {
        let marker_end = 1 + marker.len();
        if bytes.len() > marker_end
            && bytes[1..marker_end].eq_ignore_ascii_case(marker)
            && (bytes[marker_end] == b'/' || bytes[marker_end] == b'\\')
        {
            return Some(marker_end + 1);
        }
    }
    None
}

/// Account names can contain spaces and apostrophes (`C:\Users\O'Brien`,
/// `C:\Users\John Doe\...`). If a later `/` or `\` is present, take
/// everything up to it. Otherwise end only at newline, CR, or the end of
/// the string. Do not stop at space, apostrophe, quote, or `<>|?*` — a
/// bare profile path may sit on the rest of a support-report line.
fn username_end(after: &str) -> usize {
    if let Some(sep) = after.find(|c: char| c == '/' || c == '\\') {
        return sep;
    }
    after
        .find(|c: char| matches!(c, '\n' | '\r'))
        .unwrap_or(after.len())
}

fn os_name() -> String {
    #[cfg(windows)]
    {
        return windows_os_name();
    }
    #[cfg(not(windows))]
    {
        unix_os_name()
    }
}

fn cpu_name() -> String {
    let threads = std::thread::available_parallelism()
        .map(|value| value.get())
        .unwrap_or(0);
    let name = {
        #[cfg(windows)]
        {
            windows_cpu_name()
        }
        #[cfg(not(windows))]
        {
            unix_cpu_name()
        }
    };
    if threads > 0 && !name.contains("logical") {
        format!("{name} ({threads} logical)")
    } else {
        name
    }
}

fn ram_summary() -> String {
    match total_ram_bytes() {
        Some(bytes) => format_bytes(bytes),
        None => "unknown".into(),
    }
}

fn format_bytes(bytes: u64) -> String {
    let gb = bytes as f64 / (1024.0 * 1024.0 * 1024.0);
    if gb >= 10.0 {
        format!("{} GB", gb.round() as u64)
    } else if gb >= 1.0 {
        format!("{gb:.1} GB")
    } else {
        let mb = bytes as f64 / (1024.0 * 1024.0);
        format!("{} MB", mb.round() as u64)
    }
}

fn total_ram_bytes() -> Option<u64> {
    #[cfg(windows)]
    {
        return windows_total_ram_bytes();
    }
    #[cfg(not(windows))]
    {
        unix_total_ram_bytes()
    }
}

#[cfg(not(windows))]
fn unix_os_name() -> String {
    if let Ok(text) = std::fs::read_to_string("/etc/os-release") {
        for line in text.lines() {
            if let Some(value) = line.strip_prefix("PRETTY_NAME=") {
                let pretty = value.trim().trim_matches('"').trim();
                if !pretty.is_empty() {
                    return pretty.to_string();
                }
            }
        }
    }
    format!("{} {}", std::env::consts::OS, std::env::consts::ARCH)
}

#[cfg(not(windows))]
fn unix_cpu_name() -> String {
    if let Ok(text) = std::fs::read_to_string("/proc/cpuinfo") {
        for key in ["model name", "Hardware", "cpu model"] {
            for line in text.lines() {
                if let Some((found, value)) = line.split_once(':') {
                    if found.trim().eq_ignore_ascii_case(key) {
                        let name = value.trim();
                        if !name.is_empty() {
                            return name.to_string();
                        }
                    }
                }
            }
        }
    }
    std::env::var("PROCESSOR_IDENTIFIER").unwrap_or_else(|_| "unknown CPU".into())
}

#[cfg(not(windows))]
fn unix_total_ram_bytes() -> Option<u64> {
    let text = std::fs::read_to_string("/proc/meminfo").ok()?;
    for line in text.lines() {
        if let Some(rest) = line.strip_prefix("MemTotal:") {
            let kb: u64 = rest.split_whitespace().next()?.parse().ok()?;
            return Some(kb.saturating_mul(1024));
        }
    }
    None
}

#[cfg(windows)]
fn windows_os_name() -> String {
    let product = registry_sz(
        r"SOFTWARE\Microsoft\Windows NT\CurrentVersion",
        "ProductName",
    )
    .unwrap_or_else(|| "Windows".into());
    let build = registry_sz(
        r"SOFTWARE\Microsoft\Windows NT\CurrentVersion",
        "CurrentBuildNumber",
    )
    .unwrap_or_default();
    let display = registry_sz(
        r"SOFTWARE\Microsoft\Windows NT\CurrentVersion",
        "DisplayVersion",
    )
    .unwrap_or_default();
    let build_num = build.parse::<u32>().unwrap_or(0);
    let mut name = if build_num >= 22000 {
        product.replace("Windows 10", "Windows 11")
    } else {
        product
    };
    if !display.is_empty() {
        name.push(' ');
        name.push_str(&display);
    }
    if !build.is_empty() {
        name.push_str(" (build ");
        name.push_str(&build);
        name.push(')');
    }
    name
}

#[cfg(windows)]
fn windows_cpu_name() -> String {
    registry_sz(
        r"HARDWARE\DESCRIPTION\System\CentralProcessor\0",
        "ProcessorNameString",
    )
    .or_else(|| std::env::var("PROCESSOR_IDENTIFIER").ok())
    .filter(|value| !value.is_empty())
    .unwrap_or_else(|| "unknown CPU".into())
}

#[cfg(windows)]
fn windows_total_ram_bytes() -> Option<u64> {
    use windows::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};
    let mut status = MEMORYSTATUSEX::default();
    status.dwLength = std::mem::size_of::<MEMORYSTATUSEX>() as u32;
    unsafe { GlobalMemoryStatusEx(&mut status) }.ok()?;
    Some(status.ullTotalPhys)
}

#[cfg(windows)]
fn registry_sz(subkey: &str, name: &str) -> Option<String> {
    use windows::core::PCWSTR;
    use windows::Win32::Foundation::ERROR_SUCCESS;
    use windows::Win32::System::Registry::{
        RegCloseKey, RegOpenKeyExW, RegQueryValueExW, HKEY, HKEY_LOCAL_MACHINE, KEY_READ,
        REG_VALUE_TYPE,
    };

    let subkey_w: Vec<u16> = subkey.encode_utf16().chain(std::iter::once(0)).collect();
    let name_w: Vec<u16> = name.encode_utf16().chain(std::iter::once(0)).collect();
    let mut hkey = HKEY::default();
    let open = unsafe {
        RegOpenKeyExW(
            HKEY_LOCAL_MACHINE,
            PCWSTR(subkey_w.as_ptr()),
            0,
            KEY_READ,
            &mut hkey,
        )
    };
    if open != ERROR_SUCCESS {
        return None;
    }
    let mut data = vec![0u16; 512];
    let mut data_size = (data.len() * 2) as u32;
    let mut value_type = REG_VALUE_TYPE::default();
    let query = unsafe {
        RegQueryValueExW(
            hkey,
            PCWSTR(name_w.as_ptr()),
            None,
            Some(&mut value_type),
            Some(data.as_mut_ptr() as *mut u8),
            Some(&mut data_size),
        )
    };
    let _ = unsafe { RegCloseKey(hkey) };
    if query != ERROR_SUCCESS || data_size < 2 {
        return None;
    }
    let chars = (data_size as usize / 2).min(data.len());
    let raw = String::from_utf16_lossy(&data[..chars]);
    let trimmed = raw.trim_end_matches('\0').trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gpu::GpuStatus;

    fn sample_gpu() -> GpuStatus {
        GpuStatus {
            available: true,
            name: "NVIDIA GeForce RTX 3080".into(),
            backend: "Vulkan (whisper.cpp) · DirectML (ONNX)".into(),
            detail: "discrete".into(),
            device_index: 0,
            discrete: true,
            vram_mb: 10240,
        }
    }

    #[test]
    fn report_lists_version_os_cpu_ram_gpu_and_debug_flag() {
        let report = debug_report(sample_gpu(), false, "[warn] hotkey busy\n[error] boom");
        assert!(
            report.text.starts_with(&format!("VocaWin {APP_VERSION}\n")),
            "{}",
            report.text
        );
        assert!(!report.text.contains("Machine:"));
        assert!(!report.text.to_ascii_lowercase().contains("hostname"));
        assert!(report.text.contains("OS: "));
        assert!(report.text.contains("CPU: "));
        assert!(report.text.contains("RAM: "));
        assert!(report
            .text
            .contains("GPU: NVIDIA GeForce RTX 3080 (discrete, ~10240 MB)"));
        assert!(report
            .text
            .contains("GPU backend: Vulkan (whisper.cpp) · DirectML (ONNX)"));
        assert!(report.text.contains("Debug logging: off"));
        assert!(report.text.contains("[warn] hotkey busy"));
        assert!(report.text.contains("[error] boom"));
        assert_eq!(report.version, APP_VERSION);
        assert!(!report.debug_logging);
    }

    #[test]
    fn report_redacts_profile_directories_in_logs() {
        let report = debug_report(
            sample_gpu(),
            true,
            r"[error] Model missing at C:\Users\Ada\AppData\Roaming\com.vocahq.vocawin\models\x.bin
[error] also C:/Users/Ada/AppData/Roaming/com.vocahq.vocawin/models/x.bin
[error] also C:\users\Ada\AppData\Roaming\com.vocahq.vocawin\models\x.bin
[error] also C:/Users\Ada\AppData\Roaming\com.vocahq.vocawin\models\x.bin
[error] also /home/ada/.local/share/vocawin/models/x.bin
[error] also /Users/ada/Library/Application Support/vocawin/x.bin",
        );
        assert!(report.text.contains(r"C:\Users\<user>\AppData"));
        assert!(report.text.contains("C:/Users/<user>/AppData"));
        assert!(report.text.contains(r"C:\users\<user>\AppData"));
        assert!(report.text.contains(r"C:/Users\<user>\AppData"));
        assert!(report.text.contains("/home/<user>/.local"));
        assert!(report.text.contains("/Users/<user>/Library"));
        assert!(report.text.contains("<user>"));
        assert!(!report.text.contains(r"\Users\Ada\"));
        assert!(!report.text.contains(r"/Users/Ada/"));
        assert!(!report.text.contains(r"\users\Ada\"));
        assert!(!report.text.contains(r"/Users\Ada\"));
        assert!(!report.text.contains("/home/ada/"));
        assert!(!report.text.contains("/Users/ada/"));
    }

    #[test]
    fn redact_home_paths_accepts_case_and_separator_variants() {
        let out = redact_home_paths(
            r"C:\Users\Ada\x.bin
C:/Users/Ada/x.bin
C:\users\Ada\x.bin
C:/Users\Ada\x.bin
/home/ada/x.bin
/Users/ada/x.bin",
        );
        assert_eq!(
            out,
            r"C:\Users\<user>\x.bin
C:/Users/<user>/x.bin
C:\users\<user>\x.bin
C:/Users\<user>\x.bin
/home/<user>/x.bin
/Users/<user>/x.bin"
        );
        assert!(!out.contains("Ada"));
        assert!(!out.contains("ada"));
        assert!(out.contains("<user>"));
    }

    #[test]
    fn redact_keeps_spaces_inside_a_profile_name() {
        let windows = redact_home_paths(
            r"[error] Model missing at C:\Users\John Doe\AppData\Roaming\com.vocahq.vocawin\models\x.bin",
        );
        assert!(windows.contains(r"C:\Users\<user>\AppData"));
        assert!(!windows.contains("John"));
        assert!(!windows.contains("Doe"));

        let mixed = redact_home_paths(
            r"[error] also C:\Users\John Doe/AppData\Roaming\com.vocahq.vocawin\models\x.bin",
        );
        assert!(mixed.contains(r"C:\Users\<user>/AppData"));
        assert!(!mixed.contains("John"));
        assert!(!mixed.contains("Doe"));

        let bare = redact_home_paths(r"C:\Users\Ada");
        assert_eq!(bare, r"C:\Users\<user>");
    }

    #[test]
    fn redact_bare_windows_profile_with_spaces() {
        let bare = redact_home_paths(r"C:\Users\John Doe");
        assert_eq!(bare, r"C:\Users\<user>");
        assert!(!bare.contains("John"));
        assert!(!bare.contains("Doe"));
    }

    #[test]
    fn redact_windows_profile_with_apostrophe() {
        let bare = redact_home_paths(r"C:\Users\O'Brien");
        assert_eq!(bare, r"C:\Users\<user>");
        assert!(!bare.contains("O'Brien"));
        assert!(!bare.contains("Brien"));

        let nested = redact_home_paths(
            r"[error] Model missing at C:\Users\O'Brien\AppData\Roaming\com.vocahq.vocawin\models\x.bin",
        );
        assert!(nested.contains(r"C:\Users\<user>\AppData"));
        assert!(!nested.contains("O'Brien"));
        assert!(!nested.contains("Brien"));

        // Bare profile on a log line: rest of the line is redacted on purpose.
        let rest = redact_home_paths(r"[error] missing at C:\Users\O'Brien (profile)");
        assert_eq!(rest, r"[error] missing at C:\Users\<user>");
        assert!(!rest.contains("O'Brien"));
        assert!(!rest.contains("profile"));
    }

    #[test]
    fn empty_logs_still_copy_the_machine_block() {
        let report = debug_report(sample_gpu(), true, "  \n");
        assert!(report.text.contains("Debug logging: on"));
        assert!(report.text.contains("No log lines."));
    }

    #[test]
    fn ram_formatter_uses_gb_for_typical_pcs() {
        assert_eq!(format_bytes(32 * 1024 * 1024 * 1024), "32 GB");
        assert_eq!(format_bytes(8 * 1024 * 1024 * 1024), "8.0 GB");
        assert_eq!(format_bytes(512 * 1024 * 1024), "512 MB");
    }

    #[test]
    fn collect_never_returns_empty_required_fields() {
        let report = debug_report(
            GpuStatus {
                available: false,
                name: "Not available".into(),
                backend: "CPU".into(),
                detail: "".into(),
                device_index: -1,
                discrete: false,
                vram_mb: 0,
            },
            false,
            "",
        );
        assert!(!report.os.trim().is_empty());
        assert!(!report.cpu.trim().is_empty());
        assert!(!report.ram.trim().is_empty());
        assert!(report.text.contains("OS: "));
        assert!(report.text.contains("CPU: "));
        assert!(report.text.contains("RAM: "));
    }
}
