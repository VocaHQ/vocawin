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

#if defined(_WIN32)

namespace {

constexpr int kIdGetStarted = 100;
constexpr int kIdModelCombo = 200;

struct WizardData {
    OnboardingWindow* self = nullptr;
    HFONT hFont = nullptr;
    HWND hwndModelCombo = nullptr;
};

LRESULT CALLBACK onboardingWndProc(HWND hwnd, UINT msg, WPARAM wParam,
                                    LPARAM lParam) {
    if (msg == WM_NCCREATE) {
        auto* cs = reinterpret_cast<CREATESTRUCTW*>(lParam);
        SetWindowLongPtrW(hwnd, GWLP_USERDATA,
                          reinterpret_cast<LONG_PTR>(cs->lpCreateParams));
    }
    auto* d = reinterpret_cast<WizardData*>(
        GetWindowLongPtrW(hwnd, GWLP_USERDATA));

    switch (msg) {
        case WM_COMMAND: {
            const WORD id = LOWORD(wParam);
            if (id == kIdGetStarted && d != nullptr) {
                if (d->self->recommend_ && d->self->on_model_selected_) {
                    int sel = static_cast<int>(SendMessageW(
                        d->hwndModelCombo, CB_GETCURSEL, 0, 0));
                    const char* ids[] = {"tiny.en","base.en","small.en","medium.en"};
                    if (sel >= 0 && sel < 4) {
                        d->self->on_model_selected_(ids[sel]);
                    }
                }
                if (d->self->on_finished_) d->self->on_finished_();
                d->self->markOnboarded();
                DestroyWindow(hwnd);
            }
            return 0;
        }
        case WM_CLOSE:
            if (d != nullptr) {
                d->self->markOnboarded();
                if (d->self->on_finished_) d->self->on_finished_();
            }
            DestroyWindow(hwnd);
            return 0;
        case WM_DESTROY:
            if (d != nullptr) {
                if (d->hFont) DeleteObject(d->hFont);
                delete d;
                SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
            }
            return 0;
    }
    return DefWindowProcW(hwnd, msg, wParam, lParam);
}

}  // namespace

bool OnboardingWindow::show() {
    const HINSTANCE hInst = GetModuleHandleW(nullptr);
    static const wchar_t kClassName[] = L"VocaWinOnboard2";
    WNDCLASSEXW wc{};
    wc.cbSize = sizeof(wc);
    wc.lpfnWndProc = onboardingWndProc;
    wc.hInstance = hInst;
    wc.hbrBackground = reinterpret_cast<HBRUSH>(COLOR_WINDOW + 1);
    wc.lpszClassName = kClassName;
    RegisterClassExW(&wc);

    auto* d = new WizardData();
    d->self = this;

    HWND hwnd = CreateWindowExW(
        WS_EX_DLGMODALFRAME, kClassName,
        L"Welcome to VocaWin", WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU,
        CW_USEDEFAULT, CW_USEDEFAULT, 520, 400,
        nullptr, nullptr, hInst, d);
    if (hwnd == nullptr) {
        delete d;
        return false;
    }

    d->hFont = CreateFontW(15, 0, 0, 0, FW_NORMAL, FALSE, FALSE, FALSE,
                           DEFAULT_CHARSET, OUT_DEFAULT_PRECIS,
                           CLIP_DEFAULT_PRECIS, DEFAULT_QUALITY,
                           DEFAULT_PITCH | FF_SWISS, L"Segoe UI");

    auto makeLabel = [&](const wchar_t* text, int x, int y, int w, int h,
                          DWORD extra = 0) {
        HWND lbl = CreateWindowExW(0, L"STATIC", text,
            WS_CHILD | WS_VISIBLE | SS_LEFT | extra,
            x, y, w, h, hwnd, nullptr, hInst, nullptr);
        SendMessageW(lbl, WM_SETFONT, reinterpret_cast<WPARAM>(d->hFont), TRUE);
        return lbl;
    };

    makeLabel(L"VocaWin \u2014 100% Offline Voice-to-Text",
              24, 16, 460, 24, SS_CENTER);
    makeLabel(L"Welcome! VocaWin transcribes your voice locally \u2014 "
              L"no cloud, no telemetry, no accounts.",
              24, 48, 460, 40);
    makeLabel(L"Hold Right Ctrl to record. Release to transcribe and type "
              L"at your cursor.",
              24, 96, 460, 40);
    makeLabel(L"Right-click the tray icon for Settings.",
              24, 144, 460, 20);

    makeLabel(L"Speech recognition model:", 24, 184, 460, 20);
    d->hwndModelCombo = CreateWindowExW(0, L"COMBOBOX", L"",
        WS_CHILD | WS_VISIBLE | CBS_DROPDOWNLIST,
        24, 208, 460, 200, hwnd,
        reinterpret_cast<HMENU>(static_cast<INT_PTR>(kIdModelCombo)),
        hInst, nullptr);
    SendMessageW(d->hwndModelCombo, WM_SETFONT,
                 reinterpret_cast<WPARAM>(d->hFont), TRUE);
    const wchar_t* models[] = {
        L"tiny.en (39M, English, fastest)",
        L"base.en (74M, English, recommended)",
        L"small.en (244M, English, better accuracy)",
        L"medium.en (769M, English, highest accuracy)",
    };
    for (int i = 0; i < 4; ++i) {
        SendMessageW(d->hwndModelCombo, CB_ADDSTRING, 0,
                     reinterpret_cast<LPARAM>(models[i]));
    }
    int sel = 1;
    if (recommend_) {
        const std::string r = recommend_();
        if (r.find("tiny") != std::string::npos) sel = 0;
        else if (r.find("small") != std::string::npos) sel = 2;
        else if (r.find("medium") != std::string::npos) sel = 3;
    }
    SendMessageW(d->hwndModelCombo, CB_SETCURSEL, sel, 0);

    if (system_info_) {
        std::wstring info = L"System: " + system_info_();
        makeLabel(info.c_str(), 24, 244, 460, 20);
    }

    makeLabel(L"Models download automatically on first use. "
              L"You can change everything later in Settings.",
              24, 272, 460, 40);

    HWND btn = CreateWindowExW(0, L"BUTTON", L"Get Started",
        WS_CHILD | WS_VISIBLE | BS_DEFPUSHBUTTON,
        200, 330, 120, 32, hwnd,
        reinterpret_cast<HMENU>(static_cast<INT_PTR>(kIdGetStarted)),
        hInst, nullptr);
    SendMessageW(btn, WM_SETFONT, reinterpret_cast<WPARAM>(d->hFont), TRUE);

    ShowWindow(hwnd, SW_SHOW);
    UpdateWindow(hwnd);
    return true;
}

#else  // non-Win32

bool OnboardingWindow::show() {
    if (recommend_) {
        if (on_model_selected_) on_model_selected_(recommend_());
    }
    if (on_finished_) on_finished_();
    markOnboarded();
    return true;
}

#endif  // _WIN32

}  // namespace vocawin
