#pragma once

#include <atomic>
#include <cstdint>
#include <functional>
#include <string>

namespace vocawin {

// Global keyboard hook for push-to-talk / double-tap-toggle activation
// (per SPEC \u00a74.2.5). On Windows, uses SetWindowsHookExW(WH_KEYBOARD_LL)
// with a small message-pump thread. On non-Win32 platforms `start()` is a
// no-op that returns false.
class HotkeyManager {
public:
    enum class ActivationMode { PushToTalk, DoubleTapToggle };

    struct Config {
        std::uint32_t virtualKeyCode = 0xA3;  // VK_RCONTROL
        ActivationMode mode = ActivationMode::PushToTalk;
        double doubleTapThresholdMs = 400.0;
    };

    HotkeyManager();
    ~HotkeyManager();

    HotkeyManager(const HotkeyManager&) = delete;
    HotkeyManager& operator=(const HotkeyManager&) = delete;

    // Install the low-level keyboard hook and start the message-pump thread.
    // Returns true on success. Returns false (and is a no-op) on non-Win32.
    bool start(Config config);

    // Uninstall the hook and stop the message-pump thread. Safe to call when
    // not running.
    void stop();

    bool isRunning() const { return running_.load(); }

    // Callbacks (fired on the message-pump thread; the consumer is
    // responsible for marshaling to the main thread if needed).
    std::function<void()> onHotkeyPressed;
    std::function<void()> onHotkeyReleased;

    const Config& config() const { return config_; }

    // Handle a raw keyboard event for the configured hotkey. Public so the
    // platform low-level keyboard proc (a C function with no `this`) can
    // dispatch to the instance. `isKeyDown` distinguishes press vs release.
    void handleKeyEvent(bool isKeyDown);

private:
    void runMessageLoop();

    Config config_{};
    std::atomic<bool> running_{false};
    std::atomic<bool> stopRequested_{false};

#if defined(_WIN32)
    void* hook_{nullptr};        // HHOOK
    void* threadHandle_{nullptr};  // HANDLE to message-pump thread
    std::uint32_t threadId_{0};
    // For double-tap detection
    long long lastKeyDownMs_{0};
    int consecutiveDownCount_{0};
#else
    // Placeholders to keep the class layout stable in non-Win32 builds.
    void* hook_{nullptr};
    void* threadHandle_{nullptr};
    std::uint32_t threadId_{0};
    long long lastKeyDownMs_{0};
    int consecutiveDownCount_{0};
#endif
};

}  // namespace vocawin
