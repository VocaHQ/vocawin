#include "app/AppController.h"

#if defined(_WIN32)
#include <windows.h>
#include <shellapi.h>
#include <shlobj.h>
#endif
#include <cstdio>
#include <filesystem>
#include <utility>

namespace vocawin {

namespace {

std::wstring mutexNameFromPath(const std::filesystem::path& path) {
    const auto value = path.string();
    return std::wstring(value.begin(), value.end());
}

std::filesystem::path resolveExecutablePath() {
#if defined(_WIN32)
    wchar_t buf[MAX_PATH];
    const DWORD n = ::GetModuleFileNameW(nullptr, buf, MAX_PATH);
    if (n == 0 || n >= MAX_PATH) {
        return std::filesystem::current_path() / "vocawin.exe";
    }
    return std::filesystem::path(std::wstring(buf, buf + n));
#else
    return std::filesystem::current_path() / "vocawin.exe";
#endif
}

std::filesystem::path defaultDataRoot() {
#if defined(_WIN32)
    // SHGetFolderPathW is the legacy-but-reliable API that does NOT
    // require COM initialization, unlike SHGetKnownFolderPath. We use
    // it because _wgetenv("LOCALAPPDATA") returns null when the process
    // is launched via Start-Process in some PowerShell contexts.
    wchar_t path[MAX_PATH] = {};
    if (SUCCEEDED(::SHGetFolderPathW(nullptr, CSIDL_LOCAL_APPDATA,
                                      nullptr, SHGFP_TYPE_CURRENT, path))) {
        return std::filesystem::path(path) / L"VocaWin";
    }
    if (const wchar_t* local = _wgetenv(L"LOCALAPPDATA");
        local != nullptr && *local != L'\0') {
        return std::filesystem::path(local) / L"VocaWin";
    }
    if (const char* profile = std::getenv("USERPROFILE");
        profile != nullptr && *profile != '\0') {
        return std::filesystem::path(profile) / L"AppData" / L"Local" / L"VocaWin";
    }
#endif
    return std::filesystem::path("vocawin");
}

}  // namespace

AppController::AppController(std::filesystem::path data_root)
    : data_root_(data_root.empty() ? defaultDataRoot() : std::move(data_root)),
      single_instance_(L"Global\\VocaWinMutex-" + mutexNameFromPath(data_root_)),
      settings_store_(data_root_ / "config.json"),
      logger_(data_root_ / "logs" / "vocawin.log"),
      model_manager_(data_root_ / "models"),
      sound_feedback_(data_root_ / "sounds"),
      onboarding_window_(data_root_ / "onboarded.json"),
      autostart_(resolveExecutablePath()) {
    tray_icon_.setPaths(settings_store_.configPath().wstring(),
                        (data_root_ / "logs").wstring());
#if defined(_WIN32)
    {
        wchar_t exeDir[MAX_PATH] = {};
        const DWORD n = ::GetModuleFileNameW(nullptr, exeDir, MAX_PATH);
        if (n > 0 && n < MAX_PATH) {
            std::filesystem::path p(exeDir);
            p = p.parent_path();
            if (std::filesystem::exists(p / "tray-idle.ico")) {
                tray_icon_.setIconDirectory(p.wstring());
                overlay_window_.setIconDirectory(p.wstring());
            }
        }
    }
#endif
    settings_window_.setLoadHandler([this]() { return settings_store_.load(); });
    settings_window_.setSaveHandler([this](const Settings& s) {
        if (!settings_store_.save(s)) {
            logger_.error("failed to save settings");
            return false;
        }
        settings_ = s;
        applySettings();
        syncAutostartFromSettings();
        logger_.info("settings saved");
        return true;
    });
    settings_window_.setSystemInfoHandler([]() { return std::wstring(L"Windows"); });
    settings_window_.setOnAboutHandler([]() {
        return std::wstring(L"VocaWin MVP - 100% offline voice-to-text");
    });
    settings_window_.setDownloadHandler(
        [this](const std::string& id, std::function<void(float)> progress) {
            setDownloadProgressHandler(std::move(progress));
            return downloadModel(id);
        });
    settings_window_.setIsDownloadedHandler(
        [this](const std::string& id) {
            return model_manager_.isModelDownloaded(id);
        });
    settings_window_.setRecommendHandler(
        [](std::size_t ram, std::size_t vram, bool hasGpu) {
            return ModelManager::recommendModel(ram, vram, hasGpu).id;
        });

    onboarding_window_.setRecommendModel([]() {
        const auto caps = GpuDetector::detect();
        // Conservative 8GB RAM default when we cannot query; prefers tiny.en.
        const std::size_t ram = 8ULL * 1024 * 1024 * 1024;
        const auto rec = ModelManager::recommendModel(
            ram, caps.vramBytes,
            caps.cudaAvailable || caps.vulkanAvailable);
        return rec.id.empty() ? std::string("tiny.en") : rec.id;
    });
    onboarding_window_.setOnModelSelected([this](const std::string& id) {
        if (id.empty()) {
            return;
        }
        settings_.model_id = id;
        if (!settings_store_.save(settings_)) {
            logger_.error("failed to save model selection from onboarding");
        }
        logger_.info("onboarding selected model: " + id);
    });
    onboarding_window_.setOnFinished([this]() {
        logger_.info("onboarding completed");
    });

    tray_icon_.onMenuCommand = [this](TrayIcon::MenuCommand cmd) {
        switch (cmd) {
            case TrayIcon::MenuCommand::ToggleRecording:
                if (state_ == State::Recording) {
                    stopRecordingAndTranscribe();
                } else {
                    startRecording();
                }
                break;
            case TrayIcon::MenuCommand::OpenSettings:
                if (!settings_window_.show()) {
                    logger_.error("failed to open settings window");
                }
                break;
            case TrayIcon::MenuCommand::OpenLogs: {
                const auto dir = data_root_ / "logs";
                std::error_code ec;
                std::filesystem::create_directories(dir, ec);
#if defined(_WIN32)
                ShellExecuteW(nullptr, L"open", dir.wstring().c_str(),
                              nullptr, nullptr, SW_SHOWNORMAL);
#else
                (void)dir;
#endif
                break;
            }
            case TrayIcon::MenuCommand::About:
                if (onAboutRequested) onAboutRequested();
                break;
            case TrayIcon::MenuCommand::Quit:
                if (onQuitRequested) onQuitRequested();
                break;
        }
    };
}

AppController::~AppController() {
    shutdown();
}

bool AppController::initialize() {
    if (initialized_) {
        return true;
    }
    if (!single_instance_.acquire()) {
        return false;
    }
    if (!logger_.initialize()) {
        return false;
    }
    settings_ = settings_store_.load();
    logger_.info("settings loaded (model=" + settings_.model_id +
                 ", language=" + settings_.language + ")");

    sound_feedback_.setEnabled(settings_.sound_effects);
    text_injector_ = TextInjector(TextInjector::Config{
        settings_.preserve_clipboard,
        settings_.text_injection_method == 0
            ? TextInjector::Method::SendInput
            : TextInjector::Method::ClipboardPaste,
        settings_.paste_delay_ms,
        settings_.restore_delay_ms,
    });

    if (!tray_icon_.initialize()) {
        logger_.error("failed to initialize tray icon");
        return false;
    }

    HotkeyManager::Config hkCfg;
    hkCfg.virtualKeyCode = settings_.hotkey_vk_code;
    hkCfg.mode = settings_.activation_mode == 0
                     ? HotkeyManager::ActivationMode::PushToTalk
                     : HotkeyManager::ActivationMode::DoubleTapToggle;
    hkCfg.doubleTapThresholdMs = settings_.double_tap_threshold_ms;

    SilenceDetector::Config sdCfg;
    sdCfg.threshold = settings_.silence_threshold;
    sdCfg.durationMs = settings_.silence_duration_ms;
    silence_detector_.applyConfig(sdCfg);

    AudioCapture::Config acCfg;
    acCfg.sampleRate = 16000;
    acCfg.channels = 1;
    acCfg.bufferDurationMs = 100;
    acCfg.deviceIndex = -1;

    (void)hkCfg; (void)acCfg;  // consumed in wireCallbacks

    whisper_engine_.setLanguage(settings_.language);

    wireCallbacks();

    if (!hotkey_manager_.start(hkCfg)) {
        logger_.error(
            "hotkey manager failed to start (vk=0x" +
            ([](std::uint32_t v) {
                char buf[16];
                std::snprintf(buf, sizeof(buf), "%02X",
                              static_cast<unsigned>(v));
                return std::string(buf);
            })(hkCfg.virtualKeyCode) +
            ", GetLastError=" +
            std::to_string(hotkey_manager_.lastErrorCode()) + ")");
    } else {
        logger_.info(
            "hotkey manager started (vk=0x" +
            ([](std::uint32_t v) {
                char buf[16];
                std::snprintf(buf, sizeof(buf), "%02X",
                              static_cast<unsigned>(v));
                return std::string(buf);
            })(hkCfg.virtualKeyCode) +
            ", mode=" +
            std::to_string(static_cast<int>(hkCfg.mode)) + ")");
    }

    loadConfiguredModel();
    setState(model_manager_.isModelDownloaded(settings_.model_id)
                 ? State::Idle
                 : State::NotLoaded);
    syncAutostartFromSettings();
    overlay_window_.setEnabled(settings_.show_cursor_indicator);

    if (!onboarding_window_.isOnboarded()) {
        logger_.info("first run, launching onboarding wizard");
        onboarding_window_.show();
    }

    notifier_.setEnabled(settings_.sound_effects);
    initialized_ = true;
    return true;
}

void AppController::applySettings() {
    sound_feedback_.setEnabled(settings_.sound_effects);

    text_injector_ = TextInjector(TextInjector::Config{
        settings_.preserve_clipboard,
        settings_.text_injection_method == 0
            ? TextInjector::Method::SendInput
            : TextInjector::Method::ClipboardPaste,
        settings_.paste_delay_ms,
        settings_.restore_delay_ms,
    });

    HotkeyManager::Config hkCfg;
    hkCfg.virtualKeyCode = settings_.hotkey_vk_code;
    hkCfg.mode = settings_.activation_mode == 0
                     ? HotkeyManager::ActivationMode::PushToTalk
                     : HotkeyManager::ActivationMode::DoubleTapToggle;
    hkCfg.doubleTapThresholdMs = settings_.double_tap_threshold_ms;

    SilenceDetector::Config sdCfg;
    sdCfg.threshold = settings_.silence_threshold;
    sdCfg.durationMs = settings_.silence_duration_ms;
    silence_detector_.applyConfig(sdCfg);

    hotkey_manager_.stop();
    if (!hotkey_manager_.start(hkCfg)) {
        logger_.info("hotkey manager not restarted (no interactive session?)");
    }

    whisper_engine_.setLanguage(settings_.language);
    whisper_engine_.setTranslateMode(settings_.translate_to_english);
    notifier_.setEnabled(settings_.sound_effects);
    syncAutostartFromSettings();
    loadConfiguredModel();
}

void AppController::shutdown() {
    if (!initialized_) {
        return;
    }
    // Stop input first so no new recording/transcribe work is queued.
    hotkey_manager_.stop();
    audio_capture_.stop();
    // The inference lambda captures `this` and touches whisper_engine_,
    // logger_, setState, and text_injector_. MUST join before any member
    // teardown — never detach (that is a UAF on ~AppController).
    if (inference_thread_.joinable()) {
        logger_.info("shutdown: joining inference thread");
        inference_thread_.join();
    }
    inference_running_.store(false);
    whisper_engine_.unloadModel();
    tray_icon_.shutdown();
    initialized_ = false;
    logger_.info("shutdown complete");
}

bool AppController::isInitialized() const { return initialized_; }

AppController::State AppController::state() const {
    std::lock_guard<std::mutex> lk(state_mutex_);
    return state_;
}

const std::wstring& AppController::lastError() const { return last_error_; }

const Settings& AppController::settings() const { return settings_; }

void AppController::startRecording() {
    if (!initialized_) return;
    bool audioOk = false;
    {
        std::lock_guard<std::mutex> lk(state_mutex_);
        if (state_ != State::Idle && state_ != State::NotLoaded) {
            return;
        }
        audio_capture_.clearBuffer();
        AudioCapture::Config cfg;
        cfg.sampleRate = 16000;
        cfg.channels = 1;
        cfg.bufferDurationMs = 100;
        cfg.deviceIndex = -1;
        audioOk = audio_capture_.start(cfg);
        if (!audioOk) {
            last_error_ = L"Failed to start audio capture (no device?)";
        }
    }
    if (audioOk && settings_.sound_effects) {
        sound_feedback_.play(SoundFeedback::Cue::Start);
    }
    setState(audioOk ? State::Recording : State::Error);
}

void AppController::stopRecordingAndTranscribe() {
    if (!initialized_) return;
    {
        std::lock_guard<std::mutex> lk(state_mutex_);
        if (state_ != State::Recording) {
            return;
        }
    }
    audio_capture_.stop();
    if (settings_.sound_effects) {
        sound_feedback_.play(SoundFeedback::Cue::Stop);
    }
    setState(State::Processing);
    std::vector<float> captured = audio_capture_.getBuffer();
    if (inference_thread_.joinable()) {
        inference_thread_.join();
    }
    inference_running_.store(true);
    inference_thread_ = std::thread([this, samples = std::move(captured)]() mutable {
        runInference(std::move(samples));
    });
}

void AppController::cancelRecording() {
    if (!initialized_) return;
    {
        std::lock_guard<std::mutex> lk(state_mutex_);
        if (state_ != State::Recording && state_ != State::Processing) {
            return;
        }
    }
    audio_capture_.stop();
    if (inference_thread_.joinable()) {
        inference_thread_.join();
    }
    setState(State::Idle);
}

void AppController::simulateHotkeyPress() {
    tray_icon_.postHotkeyPressRequest();
    tray_icon_.pumpMessage();
}

void AppController::simulateHotkeyRelease() {
    tray_icon_.postHotkeyReleaseRequest();
    tray_icon_.pumpMessage();
}

void AppController::wireCallbacks() {
    // LL-hook callbacks run on the hotkey pump thread. Marshal to the main
    // thread via the tray HWND so WASAPI/tray/overlay never run on the hook
    // thread (also avoids LowLevelHooksTimeout unhooking us).
    hotkey_manager_.onHotkeyPressed = [this]() {
        tray_icon_.postHotkeyPressRequest();
    };
    hotkey_manager_.onHotkeyReleased = [this]() {
        tray_icon_.postHotkeyReleaseRequest();
    };
    tray_icon_.onHotkeyPressRequest = [this]() {
        logger_.info("hotkey pressed -> startRecording");
        startRecording();
    };
    tray_icon_.onHotkeyReleaseRequest = [this]() {
        logger_.info("hotkey released -> stopRecordingAndTranscribe");
        stopRecordingAndTranscribe();
    };

    audio_capture_.onAudioLevel = [this](float level) {
        silence_detector_.feedSample(level);
        if (onAudioLevelChanged) onAudioLevelChanged(level);
    };
    // Defer the silence-timeout callback to the main thread via the tray
    // window's message queue. Running stopRecordingAndTranscribe() on the
    // audio capture thread would invoke Win32 UI APIs (Shell_NotifyIconW,
    // CreateWindowExW) on the wrong thread, and would race with concurrent
    // tray/hotkey calls causing double transcription of the same audio.
    silence_detector_.onSilenceTimeout = [this]() {
        tray_icon_.postSilenceTimeoutRequest();
    };
    tray_icon_.onSilenceTimeoutRequest = [this]() {
        stopRecordingAndTranscribe();
    };
}

void AppController::runInference(std::vector<float> audio) {
    logger_.info("runInference: samples=" + std::to_string(audio.size()) +
                 " modelLoaded=" +
                 std::string(whisper_engine_.isModelLoaded() ? "1" : "0"));
    if (!whisper_engine_.isModelLoaded()) {
        last_error_ = L"No model loaded";
        logger_.error("runInference: no model loaded");
        setState(State::Error);
        setState(model_manager_.isModelDownloaded(settings_.model_id)
                     ? State::Idle
                     : State::NotLoaded);
        inference_running_.store(false);
        return;
    }
    auto result = whisper_engine_.transcribe(audio);
    if (result.has_value()) {
        logger_.info("runInference: text_len=" +
                     std::to_string(result->text.size()));
        if (!result->text.empty()) {
            text_injector_.inject(result->text);
            if (onTranscriptionComplete) {
                onTranscriptionComplete(result->text);
            }
        }
    } else {
        logger_.error("runInference: transcribe returned nullopt");
    }
    setState(State::Idle);
    inference_running_.store(false);
}

void AppController::loadConfiguredModel() {
    if (!model_manager_.isModelDownloaded(settings_.model_id)) {
        logger_.info("configured model not downloaded: " + settings_.model_id);
        return;
    }
    const auto path = model_manager_.getModelPath(settings_.model_id);
    const auto caps = GpuDetector::detect();
    WhisperEngine::GpuBackend backend;
    backend.name = caps.activeBackendName;
    backend.deviceName = caps.gpuName;
    if (whisper_engine_.loadModel(path, backend, 4)) {
        logger_.info("loaded model " + settings_.model_id +
                     " on " + backend.name);
    } else {
        last_error_ = L"Failed to load model";
        logger_.error("failed to load model: " + settings_.model_id);
    }
}

TrayIcon::State AppController::mapState(State s) const {
    switch (s) {
        case State::NotLoaded:  return TrayIcon::State::NoModel;
        case State::Idle:       return TrayIcon::State::Idle;
        case State::Recording:  return TrayIcon::State::Recording;
        case State::Processing: return TrayIcon::State::Processing;
        case State::Error:      return TrayIcon::State::Error;
    }
    return TrayIcon::State::Idle;
}

void AppController::setState(State newState) {
    State old;
    {
        std::lock_guard<std::mutex> lk(state_mutex_);
        if (state_ == newState) return;
        old = state_;
        state_ = newState;
    }
    tray_icon_.setState(mapState(newState));
    if (newState == State::Recording) {
        overlay_window_.setState(OverlayWindow::State::Recording);
        if (settings_.show_cursor_indicator) overlay_window_.show();
    } else if (newState == State::Processing) {
        overlay_window_.setState(OverlayWindow::State::Processing);
        if (settings_.show_cursor_indicator) overlay_window_.show();
    } else {
        overlay_window_.hide();
    }
    if (newState == State::Error && notifier_.isEnabled()) {
        notifier_.show(L"VocaWin error", last_error_.empty()
                                          ? std::wstring(L"Unknown error")
                                          : last_error_);
    }
    if (onStateChanged) onStateChanged(newState);
    logger_.info("state: " + std::to_string(static_cast<int>(old)) +
                 " -> " + std::to_string(static_cast<int>(newState)));
}

void AppController::syncAutostartFromSettings() {
#if defined(_WIN32)
    const bool want = settings_.launch_at_startup;
    if (want && !autostart_.isEnabled()) {
        if (!autostart_.enable()) {
            logger_.error("failed to enable autostart");
        }
    } else if (!want && autostart_.isEnabled()) {
        if (!autostart_.disable()) {
            logger_.error("failed to disable autostart");
        }
    }
#else
    (void)autostart_;
#endif
}

bool AppController::downloadModel(const std::string& modelId) {
    const std::string id = modelId.empty() ? settings_.model_id : modelId;
    logger_.info("downloading model: " + id);
    tray_icon_.setState(TrayIcon::State::Processing);

    const bool ok = model_manager_.downloadModel(
        id, "",
        [this](float p) {
            if (download_progress_) {
                download_progress_(p);
            }
        });

    if (!ok) {
        last_error_ = L"Model download failed: " +
                      std::wstring(id.begin(), id.end());
        logger_.error("download failed: " + id);
        setState(State::Error);
        // Recover to a stable idle-or-not-loaded state after a brief Error.
        setState(model_manager_.isModelDownloaded(settings_.model_id) &&
                         whisper_engine_.isModelLoaded()
                     ? State::Idle
                     : State::NotLoaded);
        return false;
    }

    settings_.model_id = id;
    if (!settings_store_.save(settings_)) {
        logger_.error("failed to persist model id after download");
    }
    loadConfiguredModel();
    if (whisper_engine_.isModelLoaded()) {
        logger_.info("download complete and model loaded: " + id);
        setState(State::Idle);
        if (notifier_.isEnabled()) {
            notifier_.show(L"VocaWin",
                           L"Model ready. Hold Right Ctrl to dictate.");
        }
        return true;
    }

    last_error_ = L"Downloaded model but failed to load it";
    logger_.error("downloaded but load failed: " + id);
    setState(State::Error);
    setState(State::NotLoaded);
    return false;
}

}  // namespace vocawin
