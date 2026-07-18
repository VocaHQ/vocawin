#pragma once

#include <functional>
#include <string>

#include "config/Settings.h"

namespace vocawin {

// Native Win32 settings dialog with 5 tabs (General / Models / Audio /
// Hotkeys / About) per SPEC \u00a75.2. The window is a modeless dialog
// that calls into a SettingsViewModel owned by AppController for load
// (current settings) and save (apply changes).
//
// We use a modeless dialog (not modal) so the user can keep using the
// app while the settings window is open. On non-Win32 platforms the
// window is a no-op stub that returns immediately; the same surface is
// still reachable via the tray menu's "Open Settings File" item.
class SettingsWindow {
public:
    using LoadFn = std::function<Settings()>;
    using SaveFn = std::function<bool(const Settings&)>;
    using RecommendFn = std::function<std::string(std::size_t ram,
                                                std::size_t vram,
                                                bool hasGpu)>;
    using SystemInfoFn = std::function<std::wstring()>;  // human-readable
    using OnAboutFn = std::function<std::wstring()>;       // about text
    // Download the given model id; progress is 0..1. Returns true on success.
    using DownloadFn = std::function<bool(const std::string& modelId,
                                          std::function<void(float)> onProgress)>;
    // Optional: report whether a model is already on disk (for status text).
    using IsDownloadedFn = std::function<bool(const std::string& modelId)>;

    SettingsWindow();
    ~SettingsWindow();

    SettingsWindow(const SettingsWindow&) = delete;
    SettingsWindow& operator=(const SettingsWindow&) = delete;

    // Bind the view-model callbacks. Must be called before show().
    void setLoadHandler(LoadFn fn) { load_ = std::move(fn); }
    void setSaveHandler(SaveFn fn) { save_ = std::move(fn); }
    void setRecommendHandler(RecommendFn fn) { recommend_ = std::move(fn); }
    void setSystemInfoHandler(SystemInfoFn fn) { system_info_ = std::move(fn); }
    void setOnAboutHandler(OnAboutFn fn) { about_text_ = std::move(fn); }
    void setDownloadHandler(DownloadFn fn) { download_ = std::move(fn); }
    void setIsDownloadedHandler(IsDownloadedFn fn) {
        is_downloaded_ = std::move(fn);
    }

    // Show the settings window. Returns true if the window was created
    // and entered its message loop; false on error or non-Win32.
    bool show();

    // Hide the settings window (no-op if not shown).
    void hide();

    bool isVisible() const { return visible_; }

    // Message handler for the dialog's window. Call from the main pump.
    bool pumpMessage();

private:
    bool createDialog();
    void populateGeneralTab();
    void populateModelsTab();
    void populateAudioTab();
    void populateHotkeysTab();
    void populateAboutTab();
    void readControlsInto(Settings& out) const;
    bool applyChanges();

#if defined(_WIN32)
    void showTabPage(int index);
    static long long __stdcall wndProc(void* hwnd, unsigned int msg,
                                       unsigned long long wparam,
                                       long long lparam);
#endif

    LoadFn load_;
    SaveFn save_;
    RecommendFn recommend_;
    SystemInfoFn system_info_;
    OnAboutFn about_text_;
    DownloadFn download_;
    IsDownloadedFn is_downloaded_;

    bool visible_{false};
    Settings pending_;  // Last loaded settings, used to populate controls

#if defined(_WIN32)
    void* hwnd_{nullptr};           // HWND
    void* h_tab_{nullptr};          // SysTabControl32
    void* h_general_{nullptr};      // tab page HWNDs (created inside dialog)
    void* h_models_{nullptr};
    void* h_audio_{nullptr};
    void* h_hotkeys_{nullptr};
    void* h_about_{nullptr};
    void* h_status_{nullptr};       // static label
    void* h_save_{nullptr};
    void* h_cancel_{nullptr};
    void* h_download_{nullptr};     // "Download model" on main dialog
#endif

    void setStatus(const std::wstring& text);
    bool downloadSelectedModel();
};

}  // namespace vocawin
