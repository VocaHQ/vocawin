#pragma once

#include <cstdint>
#include <string>

namespace vocawin {

struct Settings {
    // Default to the smallest English model so first-run download is ~75MB.
    std::string model_id{"tiny.en"};
    std::string language{"auto"};
    bool launch_at_startup{true};
    bool sound_effects{true};
    bool preserve_clipboard{true};
    bool show_cursor_indicator{true};
    bool translate_to_english{false};

    // VK_RCONTROL
    std::uint32_t hotkey_vk_code{0xA3};
    // 0=PushToTalk, 1=DoubleTapToggle
    int activation_mode{0};
    double double_tap_threshold_ms{400.0};

    float silence_threshold{0.01f};
    std::uint32_t silence_duration_ms{2000};
    std::uint32_t max_recording_duration_s{60};

    // 0=SendInput, 1=ClipboardPaste
    int text_injection_method{0};
    std::uint32_t paste_delay_ms{100};
    std::uint32_t restore_delay_ms{2000};

    // Empty = default under data_root
    std::string models_dir;
};

}  // namespace vocawin
