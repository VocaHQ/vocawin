#pragma once

#include <filesystem>
#include <functional>
#include <string>
#include <vector>

namespace vocawin {

// First-run wizard. Detects the `onboarded.json` marker file in
// dataRoot and exposes callbacks for the various setup steps. Per
// SPEC \u00a75.3 (Welcome \u2192 Microphone \u2192 Model \u2192 Hotkey).
//
// On Win32 the show() method builds an actual Win32 dialog. On other
// platforms it returns false immediately \u2014 the test is sufficient to
// drive headless flows.
class OnboardingWindow {
public:
    using SystemInfoFn = std::function<std::wstring()>;
    using RecommendFn = std::function<std::string()>;
    using EnumerateDevicesFn = std::function<std::vector<std::string>()>;
    using ModelSelectedFn = std::function<void(const std::string& id)>;
    using FinishedFn = std::function<void()>;

    explicit OnboardingWindow(std::filesystem::path markerPath);
    ~OnboardingWindow();

    OnboardingWindow(const OnboardingWindow&) = delete;
    OnboardingWindow& operator=(const OnboardingWindow&) = delete;

    void setSystemInfo(SystemInfoFn fn) { system_info_ = std::move(fn); }
    void setRecommendModel(RecommendFn fn) { recommend_ = std::move(fn); }
    void setEnumerateDevices(EnumerateDevicesFn fn) {
        devices_ = std::move(fn);
    }
    void setOnModelSelected(ModelSelectedFn fn) {
        on_model_selected_ = std::move(fn);
    }
    void setOnFinished(FinishedFn fn) { on_finished_ = std::move(fn); }

    bool isOnboarded() const;
    void markOnboarded();

    bool show();

    std::filesystem::path markerPath_;
    SystemInfoFn system_info_;
    RecommendFn recommend_;
    EnumerateDevicesFn devices_;
    ModelSelectedFn on_model_selected_;
    FinishedFn on_finished_;
};

}  // namespace vocawin
