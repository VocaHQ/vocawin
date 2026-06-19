#pragma once

#include <atomic>
#include <cstdint>
#include <string>
#include <thread>

namespace vocawin {

// Floating cursor-area indicator shown while recording or processing.
// Per SPEC \u00a75.4. On Win32 this is a small transparent topmost window
// (WS_EX_TRANSPARENT + WS_EX_TOPMOST) that follows the cursor and
// shows the matching tray icon. A background thread polls the cursor
// every 500 ms. On non-Win32 platforms it is a no-op stub.
class OverlayWindow {
public:
    enum class State { Idle, Recording, Processing };

    OverlayWindow();
    ~OverlayWindow();

    OverlayWindow(const OverlayWindow&) = delete;
    OverlayWindow& operator=(const OverlayWindow&) = delete;

    // Set the directory containing tray-recording.ico and
    // tray-processing.ico. The overlay loads the matching icon at
    // show() time.
    void setIconDirectory(const std::wstring& dir) { iconDir_ = dir; }

    void show();
    void hide();
    bool isVisible() const { return visible_ && enabled_; }

    void setEnabled(bool enabled) { enabled_ = enabled; }
    bool isEnabled() const { return enabled_; }

    void setState(State s) { state_ = s; }
    State state() const { return state_; }

private:
    void followLoop();
    void ensureWindow();
    void paintIcon();
    void destroyWindow();

    bool visible_{false};
    bool enabled_{true};
    State state_{State::Idle};
    std::wstring iconDir_;

#if defined(_WIN32)
    void* hwnd_{nullptr};
    void* hicon_{nullptr};
    std::thread followThread_;
    std::atomic<bool> stopFollow_{false};
#endif
};

}  // namespace vocawin
