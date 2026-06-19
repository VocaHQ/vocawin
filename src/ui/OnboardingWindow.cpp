#include "ui/OnboardingWindow.h"

#include <fstream>
#include <sstream>

#if defined(_WIN32)
#include <windows.h>
#include <commctrl.h>
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

constexpr int kStepCount = 4;
constexpr int kIdBack = 100;
constexpr int kIdNext = 101;
constexpr int kIdSkip = 102;
constexpr int kIdFinish = 103;
constexpr int kIdDeviceCombo = 200;
constexpr int kIdModelCombo = 201;
constexpr int kIdDownload = 202;
constexpr int kIdProgress = 203;
constexpr int kIdHotkeyEdit = 300;

struct WizardData {
    OnboardingWindow* self = nullptr;
    int currentStep = 0;
    HWND hwndPage[4] = {nullptr, nullptr, nullptr, nullptr};
    HWND hwndStepLabel = nullptr;
    HWND hwndBack = nullptr;
    HWND hwndNext = nullptr;
    HWND hwndSkip = nullptr;
    HWND hwndFinish = nullptr;
    HFONT hFont = nullptr;
};

void setFontRecursive(HWND h, HFONT f) {
    SendMessageW(h, WM_SETFONT, reinterpret_cast<WPARAM>(f), TRUE);
    HWND child = GetWindow(h, GW_CHILD);
    while (child != nullptr) {
        setFontRecursive(child, f);
        child = GetWindow(child, GW_HWNDNEXT);
    }
}

void showStep(WizardData* d, int step) {
    d->currentStep = step;
    for (int i = 0; i < kStepCount; ++i) {
        ShowWindow(d->hwndPage[i], i == step ? SW_SHOW : SW_HIDE);
    }
    const wchar_t* titles[] = {
        L"Welcome", L"Microphone", L"Model", L"Hotkey"
    };
    wchar_t buf[64];
    swprintf(buf, 64, L"Step %d of %d — %s",
             step + 1, kStepCount, titles[step]);
    SetWindowTextW(d->hwndStepLabel, buf);
    EnableWindow(d->hwndBack, step > 0);
    if (step == kStepCount - 1) {
        ShowWindow(d->hwndNext, SW_HIDE);
        ShowWindow(d->hwndFinish, SW_SHOW);
    } else {
        ShowWindow(d->hwndFinish, SW_HIDE);
        ShowWindow(d->hwndNext, SW_SHOW);
    }
}

LRESULT CALLBACK wizardWndProc(HWND hwnd, UINT msg, WPARAM wParam,
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
            if (id == kIdBack && d != nullptr && d->currentStep > 0) {
                showStep(d, d->currentStep - 1);
            } else if (id == kIdNext && d != nullptr &&
                       d->currentStep < kStepCount - 1) {
                if (d->currentStep == kStepCount - 2) {
                    if (d->self->recommend_ &&
                        d->self->on_model_selected_) {
                        d->self->on_model_selected_(
                            d->self->recommend_());
                    }
                }
                showStep(d, d->currentStep + 1);
            } else if (id == kIdSkip) {
                if (d != nullptr && d->self->on_finished_) {
                    d->self->on_finished_();
                }
                d->self->markOnboarded();
                DestroyWindow(hwnd);
            } else if (id == kIdFinish) {
                if (d != nullptr && d->self->on_finished_) {
                    d->self->on_finished_();
                }
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

void populateDeviceCombo(HWND combo, OnboardingWindow* self) {
    SendMessageW(combo, CB_RESETCONTENT, 0, 0);
    if (self->devices_) {
        for (const auto& d : self->devices_()) {
            std::wstring wd(d.begin(), d.end());
            SendMessageW(combo, CB_ADDSTRING, 0,
                         reinterpret_cast<LPARAM>(wd.c_str()));
        }
    }
    if (SendMessageW(combo, CB_GETCOUNT, 0, 0) == 0) {
        SendMessageW(combo, CB_ADDSTRING, 0,
                     reinterpret_cast<LPARAM>(L"System default"));
    }
    SendMessageW(combo, CB_SETCURSEL, 0, 0);
}

void populateModelCombo(HWND combo, OnboardingWindow* self) {
    SendMessageW(combo, CB_RESETCONTENT, 0, 0);
    const wchar_t* models[] = {
        L"tiny.en (39M, English)",
        L"base.en (74M, English)",
        L"small.en (244M, English)",
        L"medium.en (769M, English)",
    };
    for (int i = 0; i < 4; ++i) {
        SendMessageW(combo, CB_ADDSTRING, 0,
                     reinterpret_cast<LPARAM>(models[i]));
    }
    int sel = 1;  // default: base.en
    if (self->recommend_) {
        const std::string r = self->recommend_();
        if (r.find("tiny") != std::string::npos) sel = 0;
        else if (r.find("small") != std::string::npos) sel = 2;
        else if (r.find("medium") != std::string::npos) sel = 3;
    }
    SendMessageW(combo, CB_SETCURSEL, sel, 0);
}

}  // namespace

bool OnboardingWindow::show() {
    const HINSTANCE hInst = GetModuleHandleW(nullptr);
    static const wchar_t kClassName[] = L"VocaWinOnboardClass";
    WNDCLASSEXW wc{};
    wc.cbSize = sizeof(wc);
    wc.lpfnWndProc = wizardWndProc;
    wc.hInstance = hInst;
    wc.hbrBackground = reinterpret_cast<HBRUSH>(COLOR_WINDOW + 1);
    wc.lpszClassName = kClassName;
    RegisterClassExW(&wc);

    auto* d = new WizardData();
    d->self = this;

    HWND hwnd = CreateWindowExW(
        WS_EX_DLGMODALFRAME, kClassName,
        L"VocaWin Setup", WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU,
        CW_USEDEFAULT, CW_USEDEFAULT, 540, 420,
        nullptr, nullptr, hInst, d);
    if (hwnd == nullptr) {
        delete d;
        return false;
    }

    d->hFont = CreateFontW(14, 0, 0, 0, FW_NORMAL, FALSE, FALSE, FALSE,
                           DEFAULT_CHARSET, OUT_DEFAULT_PRECIS,
                           CLIP_DEFAULT_PRECIS, DEFAULT_QUALITY,
                           DEFAULT_PITCH | FF_SWISS, L"Segoe UI");
    setFontRecursive(hwnd, d->hFont);

    d->hwndStepLabel = CreateWindowExW(0, L"STATIC", L"",
        WS_CHILD | WS_VISIBLE | SS_LEFT,
        16, 12, 500, 20, hwnd, nullptr, hInst, nullptr);
    SendMessageW(d->hwndStepLabel, WM_SETFONT,
                 reinterpret_cast<WPARAM>(d->hFont), TRUE);

    constexpr int pageH = 300;
    constexpr int pageY = 40;
    for (int i = 0; i < kStepCount; ++i) {
        d->hwndPage[i] = CreateWindowExW(0, L"STATIC", L"",
            WS_CHILD, 16, pageY, 500, pageH,
            hwnd, nullptr, hInst, nullptr);
    }

    // Step 0: Welcome
    CreateWindowExW(0, L"STATIC",
        L"Welcome to VocaWin.\r\n\r\n"
        L"VocaWin converts your voice to text, completely offline.\r\n"
        L"No data ever leaves your computer.\r\n\r\n"
        L"Click Next to set up your microphone and model.",
        WS_CHILD | WS_VISIBLE | SS_LEFT,
        24, pageY + 8, 480, 120,
        d->hwndPage[0], nullptr, hInst, nullptr);

    // Step 1: Microphone
    CreateWindowExW(0, L"STATIC",
        L"Select your microphone:",
        WS_CHILD | SS_LEFT,
        24, pageY + 8, 480, 20,
        d->hwndPage[1], nullptr, hInst, nullptr);
    HWND hDev = CreateWindowExW(0, L"COMBOBOX", L"",
        WS_CHILD | WS_VISIBLE | CBS_DROPDOWNLIST,
        24, pageY + 32, 480, 200,
        d->hwndPage[1],
        reinterpret_cast<HMENU>(static_cast<INT_PTR>(kIdDeviceCombo)),
        hInst, nullptr);
    populateDeviceCombo(hDev, this);
    CreateWindowExW(0, L"STATIC",
        L"Tip: say something to test your microphone.",
        WS_CHILD | SS_LEFT,
        24, pageY + 70, 480, 40,
        d->hwndPage[1], nullptr, hInst, nullptr);

    // Step 2: Model
    CreateWindowExW(0, L"STATIC",
        L"Select a speech recognition model:",
        WS_CHILD | SS_LEFT,
        24, pageY + 8, 480, 20,
        d->hwndPage[2], nullptr, hInst, nullptr);
    HWND hModel = CreateWindowExW(0, L"COMBOBOX", L"",
        WS_CHILD | WS_VISIBLE | CBS_DROPDOWNLIST,
        24, pageY + 32, 480, 200,
        d->hwndPage[2],
        reinterpret_cast<HMENU>(static_cast<INT_PTR>(kIdModelCombo)),
        hInst, nullptr);
    populateModelCombo(hModel, this);
    if (system_info_) {
        std::wstring info = L"Detected: " + system_info_();
        CreateWindowExW(0, L"STATIC", info.c_str(),
            WS_CHILD | SS_LEFT,
            24, pageY + 70, 480, 40,
            d->hwndPage[2], nullptr, hInst, nullptr);
    }

    // Step 3: Hotkey
    CreateWindowExW(0, L"STATIC",
        L"Default hotkey: Right Ctrl (hold to record)\r\n\r\n"
        L"You can change this later in Settings.",
        WS_CHILD | SS_LEFT,
        24, pageY + 8, 480, 120,
        d->hwndPage[3], nullptr, hInst, nullptr);

    constexpr int btnY = 360;
    d->hwndBack = CreateWindowExW(0, L"BUTTON", L"< Back",
        WS_CHILD | WS_VISIBLE | BS_PUSHBUTTON,
        260, btnY, 80, 28, hwnd,
        reinterpret_cast<HMENU>(static_cast<INT_PTR>(kIdBack)),
        hInst, nullptr);
    d->hwndNext = CreateWindowExW(0, L"BUTTON", L"Next >",
        WS_CHILD | WS_VISIBLE | BS_DEFPUSHBUTTON,
        350, btnY, 80, 28, hwnd,
        reinterpret_cast<HMENU>(static_cast<INT_PTR>(kIdNext)),
        hInst, nullptr);
    CreateWindowExW(0, L"BUTTON", L"Skip",
        WS_CHILD | WS_VISIBLE, 440, btnY, 80, 28, hwnd,
        reinterpret_cast<HMENU>(static_cast<INT_PTR>(kIdSkip)),
        hInst, nullptr);
    CreateWindowExW(0, L"BUTTON", L"Finish",
        WS_CHILD, 350, btnY, 80, 28, hwnd,
        reinterpret_cast<HMENU>(static_cast<INT_PTR>(kIdFinish)),
        hInst, nullptr);

    showStep(d, 0);
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
