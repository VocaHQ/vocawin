//! Hotkey preset parsing for VocaWin. Maps friendly IDs to global-hotkey Shortcuts.

use tauri_plugin_global_shortcut::{Code, Shortcut};

/// Built-in presets shown in Settings. Values are stored in settings.json.
pub const PRESETS: &[(&str, &str)] = &[
    ("ControlRight", "Right Ctrl"),
    ("Ctrl+Alt+Space", "Ctrl+Alt+Space"),
    ("Ctrl+Shift+Space", "Ctrl+Shift+Space"),
    ("Alt+Shift+Space", "Alt+Shift+Space"),
    ("AltRight", "Right Alt"),
    ("F8", "F8"),
    ("Pause", "Pause"),
];

pub fn parse_hotkey(spec: &str) -> Result<Shortcut, String> {
    let trimmed = spec.trim();
    if trimmed.is_empty() {
        return Err("Hotkey is empty".into());
    }
    match trimmed {
        "ControlRight" | "Right Ctrl" | "RControl" | "RCtrl" => {
            Ok(Shortcut::new(None, Code::ControlRight))
        }
        "AltRight" | "Right Alt" | "RAlt" | "ROption" => Ok(Shortcut::new(None, Code::AltRight)),
        "ControlLeft" | "Left Ctrl" | "LControl" | "LCtrl" => {
            Ok(Shortcut::new(None, Code::ControlLeft))
        }
        "AltLeft" | "Left Alt" | "LAlt" => Ok(Shortcut::new(None, Code::AltLeft)),
        other => other
            .parse::<Shortcut>()
            .map_err(|error| format!("Unsupported hotkey '{other}': {error}")),
    }
}

#[allow(dead_code)]
pub fn display_name(spec: &str) -> String {
    for (id, label) in PRESETS {
        if let Ok(preset) = parse_hotkey(id) {
            if parse_hotkey(spec).ok() == Some(preset) {
                return (*label).to_string();
            }
        }
        if spec.eq_ignore_ascii_case(id) || spec.eq_ignore_ascii_case(label) {
            return (*label).to_string();
        }
    }
    format!("Custom: {spec}")
}

#[allow(dead_code)]
pub fn is_preset(spec: &str) -> bool {
    PRESETS.iter().any(|(id, _)| {
        parse_hotkey(id)
            .ok()
            .zip(parse_hotkey(spec).ok())
            .is_some_and(|(a, b)| a == b)
    })
}

/// Normalize a recorded/frontend combo into the canonical settings string.
pub fn canonicalize(spec: &str) -> Result<String, String> {
    let parsed = parse_hotkey(spec)?;
    for (id, _) in PRESETS {
        if parse_hotkey(id).ok() == Some(parsed) {
            return Ok((*id).to_string());
        }
    }
    Ok(spec
        .split('+')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("+"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tauri_plugin_global_shortcut::Modifiers;

    #[test]
    fn presets_all_parse() {
        for (id, _) in PRESETS {
            parse_hotkey(id).unwrap_or_else(|error| panic!("{id}: {error}"));
        }
    }

    #[test]
    fn right_ctrl_aliases_match() {
        let a = parse_hotkey("ControlRight").unwrap();
        let b = parse_hotkey("Right Ctrl").unwrap();
        assert_eq!(a, b);
        assert_eq!(canonicalize("Right Ctrl").unwrap(), "ControlRight");
    }

    #[test]
    fn default_combo_still_parses() {
        let shortcut = parse_hotkey("Ctrl+Alt+Space").unwrap();
        assert!(shortcut.mods.contains(Modifiers::CONTROL));
        assert!(shortcut.mods.contains(Modifiers::ALT));
        assert_eq!(shortcut.key, Code::Space);
    }
}
