#pragma once

#include <atomic>
#include <filesystem>
#include <functional>
#include <mutex>
#include <string>
#include <thread>
#include <vector>

#include "app/SingleInstance.h"
#include "audio/AudioCapture.h"
#include "audio/SilenceDetector.h"
#include "audio/SoundFeedback.h"
#include "config/Settings.h"
#include "config/SettingsStore.h"
#include "input/ClipboardManager.h"
#include "input/HotkeyManager.h"
#include "input/TextInjector.h"
#include "platform/Autostart.h"
#include "platform/GpuDetector.h"
#include "platform/Notification.h"
#include "speech/ModelManager.h"
#include "speech/WhisperEngine.h"
#include "ui/OnboardingWindow.h"
#include "ui/OverlayWindow.h"
#include "ui/SettingsWindow.h"
#include "ui/TrayIcon.h"
#include "updater/Updater.h"
#include "util/Logger.h"

namespace vocawin {

class AppController {
public:
    enum class State {
        NotLoaded,
        Idle,
        Recording,
        Processing,
        Error,
    };

    explicit AppController(std::filesystem::path data_root = "vocawin");
    ~AppController();

    bool initialize();
    void shutdown();

    bool isInitialized() const;
    State state() const;
    const std::wstring& lastError() const;
    const Settings& settings() const;

    // Direct access to the settings window (used by main loop to pump its
    // messages even when no tray callback fired).
    SettingsWindow& settingsWindow() { return settings_window_; }

    void startRecording();
    void stopRecordingAndTranscribe();
    void cancelRecording();

    // Download the configured model (or a specific id) with progress
    // updates. Returns true on success.
    bool downloadModel(const std::string& modelId = "");
    void setDownloadProgressHandler(std::function<void(float)> cb) {
        download_progress_ = std::move(cb);
    }

    std::function<void(State)> onStateChanged;
    std::function<void(float)> onAudioLevelChanged;
    std::function<void(std::wstring)> onTranscriptionComplete;
    std::function<void()> onAboutRequested;
    std::function<void()> onQuitRequested;

private:
    void setState(State newState);
    void wireCallbacks();
    void runInference(std::vector<float> audio);
    void loadConfiguredModel();
    void applySettings();
    void syncAutostartFromSettings();
    TrayIcon::State mapState(State s) const;

    std::filesystem::path data_root_;
    bool initialized_{false};
    SingleInstance single_instance_;
    SettingsStore settings_store_;
    Settings settings_{};
    TrayIcon tray_icon_{};
    SettingsWindow settings_window_{};
    OnboardingWindow onboarding_window_;
    OverlayWindow overlay_window_;
    Autostart autostart_;
    Notifier notifier_;
    Logger logger_;

    AudioCapture audio_capture_;
    WhisperEngine whisper_engine_;
    ModelManager model_manager_;
    HotkeyManager hotkey_manager_;
    TextInjector text_injector_;
    SoundFeedback sound_feedback_;
    SilenceDetector silence_detector_;

    State state_{State::NotLoaded};
    std::wstring last_error_;
    mutable std::mutex state_mutex_;
    std::thread inference_thread_;
    std::atomic<bool> inference_running_{false};
    std::function<void(float)> download_progress_;
};

}  // namespace vocawin
