//! Opt-in auto-pause: while listed processes run, dictation hotkey is unloaded.

use serde::Serialize;

/// Returns true when any running process image name matches an entry
/// (case-insensitive). Entries are exe names like `fortnite.exe` or `obs64`.
pub fn matching_process_running(watch_list: &[String]) -> bool {
    matching_process_name(watch_list).is_some()
}

/// First watched process that is currently running, if any.
pub fn matching_process_name(watch_list: &[String]) -> Option<String> {
    if watch_list.is_empty() {
        return None;
    }
    let wanted: Vec<String> = watch_list
        .iter()
        .map(|entry| normalize_process_name(entry))
        .filter(|entry| !entry.is_empty())
        .collect();
    if wanted.is_empty() {
        return None;
    }
    running_process_names().into_iter().find(|name| {
        wanted
            .iter()
            .any(|watch| name == watch || name.starts_with(watch))
    })
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RunningApp {
    pub name: String,
    pub label: String,
}

/// Deduped running image names for the Settings task picker.
pub fn list_running_apps() -> Vec<RunningApp> {
    let mut names = running_process_names();
    names.sort();
    names.dedup();
    names
        .into_iter()
        .filter(|name| !is_noise_process(name))
        .map(|name| {
            let label = name
                .strip_suffix(".exe")
                .unwrap_or(name.as_str())
                .to_string();
            RunningApp { name, label }
        })
        .collect()
}

fn is_noise_process(name: &str) -> bool {
    const SKIP: &[&str] = &[
        "svchost.exe",
        "csrss.exe",
        "smss.exe",
        "wininit.exe",
        "services.exe",
        "lsass.exe",
        "winlogon.exe",
        "dwm.exe",
        "fontdrvhost.exe",
        "conhost.exe",
        "runtimebroker.exe",
        "searchhost.exe",
        "startmenuexperiencehost.exe",
        "textinputhost.exe",
        "sihost.exe",
        "taskhostw.exe",
        "ctfmon.exe",
        "securityhealthservice.exe",
        "securityhealthsystray.exe",
        "systemsettings.exe",
        "applicationframehost.exe",
        "shellexperiencehost.exe",
        "widgetservice.exe",
        "widgets.exe",
        "idle.exe",
        "registry.exe",
        "memory compression",
        "vocawin.exe",
    ];
    SKIP.contains(&name)
}

pub fn parse_app_list(raw: &str) -> Vec<String> {
    raw.split(|ch| ch == '\n' || ch == ',' || ch == ';')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .map(str::to_string)
        .collect()
}

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

#[cfg(windows)]
fn running_process_names() -> Vec<String> {
    use windows::Win32::Foundation::CloseHandle;
    use windows::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W,
        TH32CS_SNAPPROCESS,
    };

    let mut names = Vec::new();
    unsafe {
        let snap = match CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) {
            Ok(handle) => handle,
            Err(_) => return names,
        };
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
        if Process32FirstW(snap, &mut entry).is_ok() {
            loop {
                let len = entry
                    .szExeFile
                    .iter()
                    .position(|&c| c == 0)
                    .unwrap_or(entry.szExeFile.len());
                let name = String::from_utf16_lossy(&entry.szExeFile[..len]).to_ascii_lowercase();
                if !name.is_empty() {
                    names.push(name);
                }
                if Process32NextW(snap, &mut entry).is_err() {
                    break;
                }
            }
        }
        let _ = CloseHandle(snap);
    }
    names
}

#[cfg(not(windows))]
fn running_process_names() -> Vec<String> {
    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_and_normalizes_app_list() {
        let list = parse_app_list("OBS64\nfortnite.exe, steam");
        assert_eq!(
            list.iter()
                .map(|s| normalize_process_name(s))
                .collect::<Vec<_>>(),
            vec![
                "obs64.exe".to_string(),
                "fortnite.exe".to_string(),
                "steam.exe".to_string()
            ]
        );
    }

    #[test]
    fn empty_list_means_autopause_is_off() {
        assert!(parse_app_list("").is_empty());
        assert!(parse_app_list(" \n , ; ").is_empty());
        assert!(matching_process_name(&[]).is_none());
    }

    #[test]
    fn matching_name_needs_a_running_hit() {
        assert!(matching_process_name(&[]).is_none());
        assert!(!matching_process_running(&[]));
        assert!(matching_process_name(&["definitely-not-a-real-vocawin-task.exe".into()]).is_none());
    }

    #[test]
    fn noise_filter_skips_system_hosts() {
        assert!(is_noise_process("svchost.exe"));
        assert!(is_noise_process("vocawin.exe"));
        assert!(!is_noise_process("obs64.exe"));
        assert!(!is_noise_process("fortnite.exe"));
    }
}
