#include "input/TextInjector.h"

#include <thread>

#if defined(_WIN32)
#include <windows.h>
#endif

namespace vocawin {

TextInjector::TextInjector(Config config) : config_(config) {}

std::vector<std::pair<std::uint16_t, bool>> TextInjector::buildUnicodeEvents(
    const std::wstring& text) {
    std::vector<std::pair<std::uint16_t, bool>> events;
    events.reserve(text.size() * 2);
    for (wchar_t ch : text) {
        events.emplace_back(static_cast<std::uint16_t>(ch), false);
        events.emplace_back(static_cast<std::uint16_t>(ch), true);
    }
    return events;
}

bool TextInjector::inject(const std::wstring& text) {
    if (text.empty()) {
        return true;
    }
#if !defined(_WIN32)
    (void)text;
    return false;
#else
    if (config_.method == Method::SendInput) {
        const auto events = buildUnicodeEvents(text);
        if (events.empty()) {
            return true;
        }
        std::vector<INPUT> inputs;
        inputs.reserve(events.size());
        for (const auto& [scan, isUp] : events) {
            INPUT in{};
            in.type = INPUT_KEYBOARD;
            in.ki.wScan = scan;
            in.ki.dwFlags = KEYEVENTF_UNICODE | (isUp ? KEYEVENTF_KEYUP : 0);
            inputs.push_back(in);
        }
        const UINT sent = SendInput(static_cast<UINT>(inputs.size()),
                                    inputs.data(), sizeof(INPUT));
        return sent == inputs.size();
    }

    // ClipboardPaste fallback.
    if (config_.preserveClipboard) {
        if (!clipboard_.save()) {
            // If save fails, continue anyway but skip restore later.
        }
    }
    if (!clipboard_.setText(text)) {
        return false;
    }
    std::this_thread::sleep_for(std::chrono::milliseconds(config_.pasteDelayMs));

    // Send Ctrl+V via SendInput scancodes (independent of keyboard layout).
    INPUT inputs[4] = {};
    inputs[0].type = INPUT_KEYBOARD; inputs[0].ki.wVk = VK_CONTROL;
    inputs[1].type = INPUT_KEYBOARD; inputs[1].ki.wVk = 'V';
    inputs[2].type = INPUT_KEYBOARD; inputs[2].ki.wVk = 'V';
    inputs[2].ki.dwFlags = KEYEVENTF_KEYUP;
    inputs[3].type = INPUT_KEYBOARD; inputs[3].ki.wVk = VK_CONTROL;
    inputs[3].ki.dwFlags = KEYEVENTF_KEYUP;
    SendInput(4, inputs, sizeof(INPUT));

    if (config_.preserveClipboard && clipboard_.hasSavedData()) {
        std::this_thread::sleep_for(
            std::chrono::milliseconds(config_.restoreDelayMs));
        clipboard_.restore();
    }
    return true;
#endif
}

}  // namespace vocawin
