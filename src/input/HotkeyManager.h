#pragma once

#include <atomic>
#include <cstdint>
#include <functional>
#include <mutex>
#include <thread>

namespace vocawin {

// Global keyboard hook for push-to-talk / double-tap-toggle activation
// (per SPEC §4.2.5). On Windows, uses SetWindowsHookExW(WH_KEYBOARD_LL)
// installed on a dedicated message-pump thread (hook + pump affinity).
// On non-Win32 platforms start() is a no-op that returns false.
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

    // Install the low-level keyboard hook and start the message-pump
    // thread. Returns true on success. False on non-Win32 or hook failure.
    // Always stops any previous pump first (join) so applySettings restarts
    // cannot race a detached ghost thread.
    bool start(Config config);

    // Uninstall the hook and join the pump thread. Safe when not running.
    void stop();

    bool isRunning() const { return running_.load(); }

    // Last Win32 error from a failed SetWindowsHookEx (0 if never failed).
    std::uint32_t lastErrorCode() const { return lastErrorCode_.load(); }

    // Callbacks may fire on the hook/pump thread — consumers must marshal
    // to the UI thread before calling Win32 UI or COM/WASAPI APIs.
    // Keep callbacks non-blocking (e.g. PostMessage only).
    std::function<void()> onHotkeyPressed;
    std::function<void()> onHotkeyReleased;

    const Config& config() const { return config_; }

    // Same entry the LL hook uses. Public for unit tests and synthetic
    // key injection paths. isKeyDown distinguishes press vs release.
    // Auto-repeat KEYDOWNs while held only fire pressed once until release.
    void handleKeyEvent(bool isKeyDown);

private:
    void runPumpThread(std::uint64_t generation);
    void installHookOnPumpThread();
    void uninstallHookOnPumpThread();

    Config config_{};
    std::atomic<bool> running_{false};
    std::atomic<bool> stopRequested_{false};
    std::atomic<std::uint32_t> lastErrorCode_{0};
    std::atomic<bool> keyHeld_{false};
    // Bumped on every start/stop so a late pump exit cannot touch new state.
    std::atomic<std::uint64_t> generation_{0};
    std::atomic<bool> pumpAlive_{false};

    std::thread pumpThread_;
    std::mutex callbackMutex_;
    std::mutex lifecycleMutex_;

#if defined(_WIN32)
    void* hook_{nullptr};  // HHOOK
    std::uint32_t threadId_{0};
#else
    void* hook_{nullptr};
    std::uint32_t threadId_{0};
#endif
};

}  // namespace vocawin
