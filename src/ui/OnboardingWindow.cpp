#include "ui/OnboardingWindow.h"

#include <cstdlib>
#include <fstream>
#include <sstream>

#if defined(_WIN32)
#include <windows.h>
#include <shlobj.h>
#endif

namespace vocawin {

namespace {
constexpr const char* kMarker = R"({"onboarded": true})";

bool headlessMode() {
    const char* v = std::getenv("VOCAWIN_HEADLESS");
    return v != nullptr && v[0] != '\0' && v[0] != '0';
}
}

OnboardingWindow::OnboardingWindow(std::filesystem::path markerPath)
    : markerPath_(std::move(markerPath)) {}

OnboardingWindow::~OnboardingWindow() = default;

bool OnboardingWindow::isOnboarded() const {
    std::ifstream f(markerPath_);
    if (!f.good()) return false;
    std::ostringstream os;
    os << f.rdbuf();
    const std::string body = os.str();
    return body.find("\"onboarded\": true") != std::string::npos ||
           body.find("\"onboarded\":true") != std::string::npos;
}

void OnboardingWindow::markOnboarded() {
    std::error_code ec;
    std::filesystem::create_directories(markerPath_.parent_path(), ec);
    std::ofstream f(markerPath_, std::ios::binary | std::ios::trunc);
    if (f.good()) {
        f << kMarker;
    }
}

bool OnboardingWindow::show() {
    std::string chosen = "tiny.en";
    if (recommend_) {
        const std::string rec = recommend_();
        if (!rec.empty()) {
            chosen = rec;
        }
    }
    if (on_model_selected_) {
        on_model_selected_(chosen);
    }
#if defined(_WIN32)
    // Automated tests set VOCAWIN_HEADLESS=1 so MessageBox cannot hang ctest.
    if (!headlessMode()) {
        const std::wstring wchosen(chosen.begin(), chosen.end());
        const std::wstring body =
            L"VocaWin is ready!\n\n"
            L"1. Right-click the tray icon \u2192 Settings \u2192 Models\n"
            L"2. Click \"Download model\" (recommended: " +
            wchosen +
            L")\n"
            L"3. Hold Right Ctrl to record, release to type at the cursor.\n\n"
            L"100% offline after the model is downloaded.";
        MessageBoxW(nullptr, body.c_str(), L"Welcome to VocaWin",
                    MB_OK | MB_ICONINFORMATION | MB_SETFOREGROUND);
    }
#endif
    if (on_finished_) {
        on_finished_();
    }
    markOnboarded();
    return true;
}

}  // namespace vocawin
