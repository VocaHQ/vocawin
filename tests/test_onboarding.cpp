#include <cassert>
#include <cstdio>
#include <filesystem>
#include <string>
#include <vector>

#include "ui/OnboardingWindow.h"

int main() {
    using namespace vocawin;

    const std::filesystem::path marker =
        std::filesystem::temp_directory_path() / "vocawin_test_onboarded.json";

    std::error_code ec;
    std::filesystem::remove(marker, ec);

    OnboardingWindow win(marker);
    assert(!win.isOnboarded());

    bool modelSelected = false;
    bool finished = false;
    win.setSystemInfo([]() { return std::wstring(L"Test system"); });
    win.setRecommendModel([]() { return std::string("base.en"); });
    win.setEnumerateDevices([]() {
        std::vector<std::string> v;
        v.emplace_back("Default microphone");
        return v;
    });
    win.setOnModelSelected([&](const std::string& id) {
        assert(id == "base.en");
        modelSelected = true;
    });
    win.setOnFinished([&]() { finished = true; });

    // show() completes the first-run flow on all platforms (MessageBox only
    // on Win32; headless platforms still mark onboarded and fire callbacks).
    assert(win.show());
    assert(modelSelected);
    assert(finished);
    assert(win.isOnboarded());

    // A second instance reads the marker.
    OnboardingWindow win2(marker);
    assert(win2.isOnboarded());

    std::filesystem::remove(marker, ec);
    return 0;
}
