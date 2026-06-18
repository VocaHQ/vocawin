#include "app/AppController.h"

#if defined(_WIN32)
#include <windows.h>
#include <shellapi.h>
#endif
#include <utility>

namespace vocawin {

namespace {

std::wstring mutexNameFromPath(const std::filesystem::path& path) {
    const auto value = path.string();
    return std::wstring(value.begin(), value.end());
}

}  // namespace

AppController::AppController(std::filesystem::path data_root)
    : data_root_(std::move(data_root)),
      single_instance_(L"Global\\VocaWinMutex-" + mutexNameFromPath(data_root_)),
      settings_store_(data_root_ / "config.json"),
      logger_(data_root_ / "logs" / "vocawin.log"),
      model_manager_(data_root_ / "models"),
      sound_feedback_(data_root_ / "sounds") {
    tray_icon_.setPaths(settings_store_.configPath().wstring(),
                        (data_root_ / "logs").wstring());
    tray_icon_.onMenuCommand = [this](TrayIcon::MenuCommand cmd) {
        switch (cmd) {
            case TrayIcon::MenuCommand::ToggleRecording:
                if (state_ == State::Recording) {
                    stopRecordingAndTranscribe();
                } else {
                    startRecording();
                }
                break;
            case TrayIcon::MenuCommand::OpenSettings: {
                const auto path = settings_store_.configPath();
                ShellExecuteW(nullptr, L"open", path.wstring().c_str(),
                              nullptr, nullptr, SW_SHOWNORMAL);
                break;
            }
            case TrayIcon::MenuCommand::OpenLogs: {
                const auto dir = data_root_ / "logs";
                std::error_code ec;
                std::filesystem::create_directories(dir, ec);
                ShellExecuteW(nullptr, L"open", dir.wstring().c_str(),
                              nullptr, nullptr, SW_SHOWNORMAL);
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

    AudioCapture::Config acCfg;
    acCfg.sampleRate = 16000;
    acCfg.channels = 1;
    acCfg.bufferDurationMs = 100;
    acCfg.deviceIndex = -1;

    (void)hkCfg; (void)sdCfg; (void)acCfg;  // consumed in wireCallbacks

    whisper_engine_.setLanguage(settings_.language);

    wireCallbacks();

    if (!hotkey_manager_.start(hkCfg)) {
        logger_.info("hotkey manager not started (no interactive session?)");
    }

    loadConfiguredModel();
    setState(model_manager_.isModelDownloaded(settings_.model_id)
                 ? State::Idle
                 : State::NotLoaded);
    initialized_ = true;
    return true;
}

void AppController::shutdown() {
    if (!initialized_) {
        return;
    }
    hotkey_manager_.stop();
    audio_capture_.stop();
    whisper_engine_.unloadModel();
    if (inference_thread_.joinable()) {
        inference_thread_.join();
    }
    logger_.info("shutdown complete");
    tray_icon_.shutdown();
    initialized_ = false;
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
    const bool started = audio_capture_.start(cfg);
    if (!started) {
        last_error_ = L"Failed to start audio capture (no device?)";
        setState(State::Error);
        return;
    }
    if (settings_.sound_effects) {
        sound_feedback_.play(SoundFeedback::Cue::Start);
    }
    setState(State::Recording);
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

void AppController::wireCallbacks() {
    hotkey_manager_.onHotkeyPressed = [this]() { startRecording(); };
    hotkey_manager_.onHotkeyReleased = [this]() { stopRecordingAndTranscribe(); };

    audio_capture_.onAudioLevel = [this](float level) {
        if (onAudioLevelChanged) onAudioLevelChanged(level);
    };
}

void AppController::runInference(std::vector<float> audio) {
    auto result = whisper_engine_.transcribe(audio);
    if (result.has_value() && !result->text.empty()) {
        text_injector_.inject(result->text);
        if (onTranscriptionComplete) {
            onTranscriptionComplete(result->text);
        }
    } else if (!whisper_engine_.isModelLoaded()) {
        last_error_ = L"No model loaded";
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
    if (whisper_engine_.loadModel(path, 4)) {
        logger_.info("loaded model: " + settings_.model_id);
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
    if (onStateChanged) onStateChanged(newState);
    logger_.info("state: " + std::to_string(static_cast<int>(old)) +
                 " -> " + std::to_string(static_cast<int>(newState)));
}

}  // namespace vocawin
