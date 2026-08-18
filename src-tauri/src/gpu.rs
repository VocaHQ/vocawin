//! Best-effort GPU readout for Settings. Detection failure falls back to CPU.

use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GpuStatus {
    pub available: bool,
    pub name: String,
    pub backend: String,
    pub detail: String,
}

pub fn detect_gpu() -> GpuStatus {
    #[cfg(windows)]
    {
        return detect_windows_gpu();
    }
    #[cfg(not(windows))]
    {
        GpuStatus {
            available: false,
            name: "Not available".into(),
            backend: "CPU".into(),
            detail: "GPU detection runs in Windows builds.".into(),
        }
    }
}

#[cfg(windows)]
fn detect_windows_gpu() -> GpuStatus {
    use windows::Win32::Graphics::Dxgi::{
        CreateDXGIFactory1, IDXGIAdapter1, IDXGIFactory1, DXGI_ADAPTER_FLAG,
        DXGI_ADAPTER_FLAG_SOFTWARE,
    };

    let factory: IDXGIFactory1 = match unsafe { CreateDXGIFactory1() } {
        Ok(factory) => factory,
        Err(error) => {
            return GpuStatus {
                available: false,
                name: "Unknown".into(),
                backend: "CPU".into(),
                detail: format!("DXGI unavailable ({error}); Whisper falls back to CPU."),
            };
        }
    };

    let mut index = 0u32;
    let mut chosen: Option<String> = None;
    loop {
        let adapter: IDXGIAdapter1 = match unsafe { factory.EnumAdapters1(index) } {
            Ok(adapter) => adapter,
            Err(_) => break,
        };
        let desc = match unsafe { adapter.GetDesc1() } {
            Ok(desc) => desc,
            Err(_) => {
                index += 1;
                continue;
            }
        };
        let flags = DXGI_ADAPTER_FLAG(desc.Flags as i32);
        let name = wchar_to_string(&desc.Description);
        let software = flags.contains(DXGI_ADAPTER_FLAG_SOFTWARE)
            || name.to_ascii_lowercase().contains("microsoft basic render")
            || name.to_ascii_lowercase().contains("warp");
        if !software {
            chosen = Some(name);
            break;
        }
        index += 1;
    }

    match chosen {
        Some(name) => GpuStatus {
            available: true,
            name,
            backend: "Vulkan (whisper.cpp) · DirectML (ONNX)".into(),
            detail: "Whisper uses Vulkan when the runtime finds a GPU; ONNX uses DirectML.".into(),
        },
        None => GpuStatus {
            available: false,
            name: "No discrete/integrated GPU found".into(),
            backend: "CPU".into(),
            detail: "Whisper and ONNX will use CPU.".into(),
        },
    }
}

#[cfg(windows)]
fn wchar_to_string(buf: &[u16]) -> String {
    let len = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
    String::from_utf16_lossy(&buf[..len])
}
