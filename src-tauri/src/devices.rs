//! WASAPI input device listing via cpal (Windows host).

use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InputDevice {
    pub name: String,
    pub is_default: bool,
}

pub fn list_input_devices() -> Result<Vec<InputDevice>, String> {
    #[cfg(windows)]
    {
        use cpal::traits::{DeviceTrait, HostTrait};
        let host = cpal::default_host();
        let default_name = host
            .default_input_device()
            .and_then(|device| device.name().ok());
        let mut devices = Vec::new();
        let inputs = host
            .input_devices()
            .map_err(|error| format!("Could not enumerate microphones: {error}"))?;
        for device in inputs {
            let name = match device.name() {
                Ok(name) => name,
                Err(_) => continue,
            };
            let is_default = default_name.as_deref() == Some(name.as_str());
            devices.push(InputDevice { name, is_default });
        }
        if devices.is_empty() {
            return Err("No microphone was found.".into());
        }
        Ok(devices)
    }
    #[cfg(not(windows))]
    {
        Ok(vec![InputDevice {
            name: "Default microphone".into(),
            is_default: true,
        }])
    }
}
