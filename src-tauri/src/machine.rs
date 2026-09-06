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
/// Windows user name.
fn redact_home_paths(text: &str) -> String {
    let mut out = text.to_string();
    for prefix in [r"\Users\", "/Users/", "/home/"] {
        out = redact_after_prefix(&out, prefix);
    }
    out
}

fn redact_after_prefix(text: &str, prefix: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(idx) = rest.find(prefix) {
        result.push_str(&rest[..idx + prefix.len()]);
        let after = &rest[idx + prefix.len()..];
        let end = after
            .find(|c: char| c == '/' || c == '\\' || c.is_whitespace() || c == '"' || c == '\'')
            .unwrap_or(after.len());
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
            report.text.starts_with("VocaWin 0.1.0\n"),
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
        assert_eq!(report.version, "0.1.0");
        assert!(!report.debug_logging);
    }

    #[test]
    fn report_redacts_profile_directories_in_logs() {
        let report = debug_report(
            sample_gpu(),
            true,
            r"[error] Model missing at C:\Users\Ada\AppData\Roaming\com.vocahq.vocawin\models\x.bin
[error] also /home/ada/.local/share/vocawin/models/x.bin
[error] also /Users/ada/Library/Application Support/vocawin/x.bin",
        );
        assert!(report.text.contains(r"C:\Users\<user>\AppData"));
        assert!(report.text.contains("/home/<user>/.local"));
        assert!(report.text.contains("/Users/<user>/Library"));
        assert!(!report.text.contains(r"\Users\Ada\"));
        assert!(!report.text.contains("/home/ada/"));
        assert!(!report.text.contains("/Users/ada/"));
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
