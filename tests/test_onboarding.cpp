#include <cassert>
#include <cstdio>
#include <filesystem>

#include "ui/OnboardingWindow.h"

int main() {
    using namespace vocawin;

    const std::filesystem::path marker =
        std::filesystem::temp_directory_path() / "vocawin_test_onboarded.json";

    std::error_code ec;
    std::filesystem::remove(marker, ec);

    OnboardingWindow win(marker);
    assert(!win.isOnboarded());

    // Bind all the steps the wizard needs.
    win.setSystemInfo([]() { return std::wstring(L"Test system"); });
    win.setRecommendModel([]() { return std::string("base.en"); });
    win.setEnumerateDevices([]() {
        std::vector<std::string> v;
        v.emplace_back("Default microphone");
        return v;
    });
    win.setOnModelSelected([](const std::string& id) {
        assert(id == "base.en");
    });
    win.setOnFinished([]() { /* no-op for test */ });

    // Calling show() on non-Win32 returns false (no actual window).
#if !defined(_WIN32)
    assert(!win.show());
#endif

    // Mark complete.
    win.markOnboarded();
    assert(win.isOnboarded());

    // A second instance reads the marker.
    OnboardingWindow win2(marker);
    assert(win2.isOnboarded());

    std::filesystem::remove(marker, ec);
    return 0;
}
