#pragma once

#include <functional>
#include <string>

namespace vocawin {

class TrayIcon {
public:
    enum class State { Idle, Recording, Processing, Error, NoModel };
    enum class MenuCommand {
        ToggleRecording,
        OpenSettings,
        OpenLogs,
        About,
        Quit,
    };

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

    // Set the file paths surfaced in the context menu (logs / settings).
    void setPaths(const std::wstring& settingsPath,
                  const std::wstring& logsDir);

    // Set the directory containing tray-*.ico files (resources/icons).
    // When set, updateNotifyArea loads the matching custom icon via
    // LoadImageW instead of using stock IDI_* icons.
    void setIconDirectory(const std::wstring& iconDir) { iconPath_ = iconDir; }

    // Fired on the Win32 message-pump thread when the user picks a menu
    // item. The consumer is responsible for marshaling to the main thread.
    std::function<void(MenuCommand)> onMenuCommand;

    // Fired on the Win32 message-pump thread when a silence-timeout
    // request is posted via postSilenceTimeoutRequest(). Used to defer
    // capture-thread callbacks to the main thread, avoiding running
    // Win32 UI APIs and inference on the wrong thread.
    std::function<void()> onSilenceTimeoutRequest;

    // Post a silence-timeout request to the tray window. Safe to call
    // from any thread; the callback fires on the thread that pumps the
    // tray window's messages (the main thread).
    void postSilenceTimeoutRequest();

    // Process any pending Win32 message for this tray. Returns true if a
    // callback was fired. Call from the main message loop.
    bool pumpMessage();

#if defined(_WIN32)
    void onTrayMessage(unsigned int msg);
    void onMenuCommandId(unsigned int id);
    void onSilenceTimeoutRequestInternal();
#endif

private:
    bool updateNotifyArea();
#if defined(_WIN32)
    void showContextMenu();
#endif

    State state_{State::Idle};
    std::wstring tooltip_;
    bool initialized_{false};
    std::wstring settingsPath_;
    std::wstring logsDir_;
    std::wstring iconPath_;

#if defined(_WIN32)
    void* hwnd_{nullptr};        // HWND
    void* nid_{nullptr};         // NOTIFYICONDATAW*
    void* currentIcon_{nullptr}; // HICON
    bool ownsCurrentIcon_{false};
    static constexpr unsigned int kMenuToggle = 1;
    static constexpr unsigned int kMenuSettings = 2;
    static constexpr unsigned int kMenuLogs = 3;
    static constexpr unsigned int kMenuAbout = 4;
    static constexpr unsigned int kMenuQuit = 5;
#endif
};

}  // namespace vocawin
