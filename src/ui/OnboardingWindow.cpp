#include "ui/OnboardingWindow.h"

#include <fstream>
#include <sstream>

#if defined(_WIN32)
#include <windows.h>
#include <shlobj.h>
#endif

namespace vocawin {

namespace {
constexpr const char* kMarker = R"({"onboarded": true})";
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
    if (recommend_ && on_model_selected_) {
        on_model_selected_(recommend_());
    }
#if defined(_WIN32)
    MessageBoxW(nullptr,
        L"VocaWin is ready!\n\n"
        L"Hold Right Ctrl to record.\n"
        L"Release to transcribe and type at your cursor.\n\n"
        L"Right-click the tray icon for Settings.\n"
        L"Models download automatically on first use.",
        L"Welcome to VocaWin",
        MB_OK | MB_ICONINFORMATION | MB_SETFOREGROUND);
#endif
    if (on_finished_) on_finished_();
    markOnboarded();
    return true;
}

}  // namespace vocawin
