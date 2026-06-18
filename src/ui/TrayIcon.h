#pragma once

#include <string>

namespace vocawin {

class TrayIcon {
public:
    enum class State { Idle, Recording, Processing, Error, NoModel };

    TrayIcon();
    ~TrayIcon();

    TrayIcon(const TrayIcon&) = delete;
    TrayIcon& operator=(const TrayIcon&) = delete;

    bool initialize();
    void shutdown();

    // Returns false if not initialized or after shutdown.
    bool setState(State state);

    State state() const { return state_; }
    void setTooltip(const std::wstring& tooltip);
    bool isInitialized() const { return initialized_; }

    static std::wstring defaultTooltipFor(State state);

private:
    bool updateNotifyArea();

    State state_{State::Idle};
    std::wstring tooltip_;
    bool initialized_{false};

#if defined(_WIN32)
    void* hwnd_{nullptr};        // HWND
    void* nid_{nullptr};         // NOTIFYICONDATAW*
    void* currentIcon_{nullptr}; // HICON
#endif
};

}  // namespace vocawin
