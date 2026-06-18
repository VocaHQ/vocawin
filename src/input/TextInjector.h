#pragma once

#include <cstdint>
#include <string>
#include <utility>
#include <vector>

#include "input/ClipboardManager.h"

namespace vocawin {

// System-wide text delivery. Primary method is SendInput with
// KEYEVENTF_UNICODE (per SPEC \u00a74.2.6); fallback for problematic apps is
// clipboard-set + Ctrl+V. The fallback is opt-in via Config::method.
//
// On non-Win32 platforms both methods are no-ops and `inject()` returns false.
class TextInjector {
public:
    enum class Method { SendInput, ClipboardPaste };

    struct Config {
        bool preserveClipboard = true;
        Method method = Method::SendInput;
        std::uint32_t pasteDelayMs = 100;     // Wait for clipboard to settle
        std::uint32_t restoreDelayMs = 2000;  // Wait before restoring clipboard
    };

    explicit TextInjector(Config config = makeDefaultConfig());

    // Returns true on success (or if `text` is empty). On non-Win32 this
    // returns false for non-empty text.
    bool inject(const std::wstring& text);

    // Build the sequence of Unicode scan-code events for SendInput. Public
    // so it can be unit-tested without invoking the OS.
    //
    // Returns a list of (scanCode, isKeyUp) pairs. For BMP characters a
    // single wchar_t maps to a (down, up) pair. For non-BMP characters
    // (surrogate pair) four events are produced (high-surrogate down/up,
    // low-surrogate down/up).
    static std::vector<std::pair<std::uint16_t, bool>> buildUnicodeEvents(
        const std::wstring& text);

    const Config& config() const { return config_; }

private:
    static Config makeDefaultConfig() { return Config{}; }

    Config config_;
    ClipboardManager clipboard_;
};

}  // namespace vocawin
