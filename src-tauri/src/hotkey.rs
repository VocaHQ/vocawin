//! Hotkey presets and parsing for VocaWin.
//!
//! Lone modifiers (Right Ctrl / Right Alt / Right Shift) cannot be bound with
//! RegisterHotKey. They are first-class presets here and are watched by the
//! WH_KEYBOARD_LL hook in `hook`.

/// Built-in presets shown in Settings. Values are stored in settings.json.
/// Right Alt is first: it matches VocaLinux hold-default (PTT). The hook leaves
/// AltGr (Ctrl+Right Alt) alone so layout characters still type.
pub const PRESETS: &[(&str, &str)] = &[
    ("AltRight", "Right Alt"),
    ("ControlRight", "Right Ctrl"),
    ("ShiftRight", "Right Shift"),
    ("F8", "F8"),
    ("F9", "F9"),
    ("F10", "F10"),
    ("Ctrl+Alt+Space", "Ctrl+Alt+Space"),
    ("Ctrl+Shift+Space", "Ctrl+Shift+Space"),
];

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HotkeySpec {
    /// A single key, including a side-specific modifier (VK_RCONTROL, …).
    Lone { vk: u32 },
    /// Modifier chord + key. Win/Super is rejected at parse time.
    Combo {
        ctrl: bool,
        alt: bool,
        shift: bool,
        vk: u32,
    },
}

// Virtual-key codes (winuser.h). Kept as u32 so non-Windows CI can parse too.
pub const VK_SPACE: u32 = 0x20;
pub const VK_LSHIFT: u32 = 0xA0;
pub const VK_RSHIFT: u32 = 0xA1;
pub const VK_LCONTROL: u32 = 0xA2;
pub const VK_RCONTROL: u32 = 0xA3;
pub const VK_LMENU: u32 = 0xA4;
pub const VK_RMENU: u32 = 0xA5;
pub const VK_F8: u32 = 0x77;
pub const VK_F9: u32 = 0x78;
pub const VK_F10: u32 = 0x79;

pub fn parse_hotkey(spec: &str) -> Result<HotkeySpec, String> {
    let trimmed = spec.trim();
    if trimmed.is_empty() {
        return Err("Hotkey is empty".into());
    }
    match trimmed {
        "ControlRight" | "Right Ctrl" | "RControl" | "RCtrl" => {
            Ok(HotkeySpec::Lone { vk: VK_RCONTROL })
        }
        "ControlLeft" | "Left Ctrl" | "LControl" | "LCtrl" => {
            Ok(HotkeySpec::Lone { vk: VK_LCONTROL })
        }
        "AltRight" | "Right Alt" | "RAlt" | "ROption" => Ok(HotkeySpec::Lone { vk: VK_RMENU }),
        "AltLeft" | "Left Alt" | "LAlt" => Ok(HotkeySpec::Lone { vk: VK_LMENU }),
        "ShiftRight" | "Right Shift" | "RShift" => Ok(HotkeySpec::Lone { vk: VK_RSHIFT }),
        "ShiftLeft" | "Left Shift" | "LShift" => Ok(HotkeySpec::Lone { vk: VK_LSHIFT }),
        "F8" => Ok(HotkeySpec::Lone { vk: VK_F8 }),
        "F9" => Ok(HotkeySpec::Lone { vk: VK_F9 }),
        "F10" => Ok(HotkeySpec::Lone { vk: VK_F10 }),
        "Space" => Ok(HotkeySpec::Lone { vk: VK_SPACE }),
        other => parse_combo(other),
    }
}

fn parse_combo(spec: &str) -> Result<HotkeySpec, String> {
    let mut ctrl = false;
    let mut alt = false;
    let mut shift = false;
    let mut key: Option<u32> = None;
    for raw in spec.split('+') {
        let part = raw.trim();
        if part.is_empty() {
            continue;
        }
        let lower = part.to_ascii_lowercase();
        match lower.as_str() {
            "ctrl" | "control" | "controlleft" | "controlright" | "lctrl" | "rctrl" => {
                ctrl = true;
            }
            "alt" | "option" | "altleft" | "altright" | "lalt" | "ralt" => alt = true,
            "shift" | "shiftleft" | "shiftright" | "lshift" | "rshift" => shift = true,
            "meta" | "win" | "super" | "cmd" | "command" | "windows" => {
                return Err(
                    "Win/Super shortcuts are reserved on Windows. Pick Right Alt, Right Ctrl, a function key, or another combo."
                        .into(),
                );
            }
            "space" => key = Some(VK_SPACE),
            "f8" => key = Some(VK_F8),
            "f9" => key = Some(VK_F9),
            "f10" => key = Some(VK_F10),
            other if other.len() == 1 => {
                let ch = other.chars().next().unwrap().to_ascii_uppercase();
                if ch.is_ascii_alphanumeric() {
                    key = Some(ch as u32);
                } else {
                    return Err(format!("Unsupported hotkey key '{part}'"));
                }
            }
            _ => return Err(format!("Unsupported hotkey part '{part}'")),
        }
    }
    let vk = key.ok_or_else(|| format!("Hotkey '{spec}' is missing a key"))?;
    if !ctrl && !alt && !shift {
        return Ok(HotkeySpec::Lone { vk });
    }
    Ok(HotkeySpec::Combo {
        ctrl,
        alt,
        shift,
        vk,
    })
}

#[allow(dead_code)]
pub fn display_name(spec: &str) -> String {
    for (id, label) in PRESETS {
        if parse_hotkey(id).ok() == parse_hotkey(spec).ok() {
            return (*label).to_string();
        }
        if spec.eq_ignore_ascii_case(id) || spec.eq_ignore_ascii_case(label) {
            return (*label).to_string();
        }
    }
    format!("Custom: {spec}")
}

/// Normalize a recorded/frontend combo into the canonical settings string.
pub fn canonicalize(spec: &str) -> Result<String, String> {
    let parsed = parse_hotkey(spec)?;
    for (id, _) in PRESETS {
        if parse_hotkey(id).ok() == Some(parsed.clone()) {
            return Ok((*id).to_string());
        }
    }
    Ok(match parsed {
        HotkeySpec::Lone { vk } => lone_id(vk).unwrap_or_else(|| spec.to_string()),
        HotkeySpec::Combo {
            ctrl,
            alt,
            shift,
            vk,
        } => {
            let mut parts = Vec::new();
            if ctrl {
                parts.push("Ctrl".to_string());
            }
            if alt {
                parts.push("Alt".to_string());
            }
            if shift {
                parts.push("Shift".to_string());
            }
            parts.push(key_token(vk));
            parts.join("+")
        }
    })
}

fn lone_id(vk: u32) -> Option<String> {
    Some(
        match vk {
            VK_RCONTROL => "ControlRight",
            VK_LCONTROL => "ControlLeft",
            VK_RMENU => "AltRight",
            VK_LMENU => "AltLeft",
            VK_RSHIFT => "ShiftRight",
            VK_LSHIFT => "ShiftLeft",
            VK_F8 => "F8",
            VK_F9 => "F9",
            VK_F10 => "F10",
            VK_SPACE => "Space",
            _ => return None,
        }
        .to_string(),
    )
}

fn key_token(vk: u32) -> String {
    match vk {
        VK_SPACE => "Space".into(),
        VK_F8 => "F8".into(),
        VK_F9 => "F9".into(),
        VK_F10 => "F10".into(),
        v if (0x30..=0x39).contains(&v) || (0x41..=0x5A).contains(&v) => {
            char::from_u32(v).unwrap_or('?').to_string()
        }
        other => format!("Vk{other}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn presets_all_parse() {
        for (id, _) in PRESETS {
            parse_hotkey(id).unwrap_or_else(|error| panic!("{id}: {error}"));
        }
    }

    #[test]
    fn right_alt_is_lone_rmenu() {
        assert_eq!(
            parse_hotkey("AltRight").unwrap(),
            HotkeySpec::Lone { vk: VK_RMENU }
        );
        assert_eq!(canonicalize("Right Alt").unwrap(), "AltRight");
    }

    #[test]
    fn right_ctrl_is_lone_rcontrol() {
        assert_eq!(
            parse_hotkey("ControlRight").unwrap(),
            HotkeySpec::Lone { vk: VK_RCONTROL }
        );
        assert_eq!(canonicalize("Right Ctrl").unwrap(), "ControlRight");
    }

    #[test]
    fn right_shift_and_f_keys_parse() {
        assert_eq!(
            parse_hotkey("ShiftRight").unwrap(),
            HotkeySpec::Lone { vk: VK_RSHIFT }
        );
        assert_eq!(parse_hotkey("F9").unwrap(), HotkeySpec::Lone { vk: VK_F9 });
    }

    #[test]
    fn default_combo_still_parses() {
        assert_eq!(
            parse_hotkey("Ctrl+Alt+Space").unwrap(),
            HotkeySpec::Combo {
                ctrl: true,
                alt: true,
                shift: false,
                vk: VK_SPACE
            }
        );
    }

    #[test]
    fn win_super_is_rejected() {
        assert!(parse_hotkey("Super+Space").unwrap_err().contains("reserved"));
        assert!(parse_hotkey("Win+A").unwrap_err().contains("reserved"));
    }
}
