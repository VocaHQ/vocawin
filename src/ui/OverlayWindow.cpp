#include "ui/OverlayWindow.h"

#include <atomic>
#include <chrono>
#include <filesystem>
#include <thread>

#if defined(_WIN32)
#include <windows.h>
#endif

namespace vocawin {

OverlayWindow::OverlayWindow() = default;

OverlayWindow::~OverlayWindow() {
    hide();
}

void OverlayWindow::show() {
    if (visible_ || !enabled_) {
        return;
    }
#if defined(_WIN32)
    ensureWindow();
    if (hwnd_ != nullptr) {
        paintIcon();
        ShowWindow(static_cast<HWND>(hwnd_), SW_SHOWNOACTIVATE);
        followLoop();
    }
#endif
    visible_ = true;
}

void OverlayWindow::hide() {
#if defined(_WIN32)
    stopFollow_.store(true);
    if (followThread_.joinable()) {
        followThread_.join();
    }
    destroyWindow();
#endif
    visible_ = false;
}

#if defined(_WIN32)

void OverlayWindow::ensureWindow() {
    if (hwnd_ != nullptr) {
        return;
    }
    const HINSTANCE hInst = GetModuleHandleW(nullptr);
    static const wchar_t kClassName[] = L"VocaWinOverlayClass";
    WNDCLASSEXW wc{};
    wc.cbSize = sizeof(wc);
    wc.lpfnWndProc = DefWindowProcW;
    wc.hInstance = hInst;
    wc.hbrBackground = nullptr;
    wc.lpszClassName = kClassName;
    RegisterClassExW(&wc);  // ignore ERROR_CLASS_ALREADY_EXISTS

    hwnd_ = CreateWindowExW(
        WS_EX_TOPMOST | WS_EX_TRANSPARENT | WS_EX_LAYERED
            | WS_EX_TOOLWINDOW | WS_EX_NOPARENTNOTIFY,
        kClassName, L"",
        WS_POPUP,
        0, 0, 16, 16,
        nullptr, nullptr, hInst, nullptr);
    if (hwnd_ == nullptr) {
        return;
    }
    SetLayeredWindowAttributes(static_cast<HWND>(hwnd_),
                               RGB(0, 0, 0),
                               255, LWA_ALPHA);
}

void OverlayWindow::paintIcon() {
    if (hwnd_ == nullptr) {
        return;
    }
    if (hicon_ != nullptr) {
        DestroyIcon(static_cast<HICON>(hicon_));
        hicon_ = nullptr;
    }
    const wchar_t* name = nullptr;
    switch (state_) {
        case State::Recording:  name = L"tray-recording.ico";  break;
        case State::Processing: name = L"tray-processing.ico"; break;
        case State::Idle:       name = nullptr;                break;
    }
    if (name == nullptr || iconDir_.empty()) {
        return;
    }
    std::filesystem::path full = std::filesystem::path(iconDir_) / name;
    hicon_ = LoadImageW(nullptr, full.wstring().c_str(),
                        IMAGE_ICON, 16, 16,
                        LR_LOADFROMFILE);
    if (hicon_ == nullptr) {
        return;
    }
    HDC hdc = GetDC(static_cast<HWND>(hwnd_));
    if (hdc != nullptr) {
        DrawIconEx(hdc, 0, 0, static_cast<HICON>(hicon_), 16, 16,
                   0, nullptr, DI_NORMAL);
        ReleaseDC(static_cast<HWND>(hwnd_), hdc);
    }
}

void OverlayWindow::followLoop() {
    if (followThread_.joinable()) {
        return;  // already running
    }
    stopFollow_.store(false);
    followThread_ = std::thread([this]() {
        POINT pt{};
        RECT rc{};
        while (!stopFollow_.load()) {
            if (GetCursorPos(&pt) &&
                GetWindowRect(static_cast<HWND>(hwnd_), &rc)) {
                const int w = rc.right - rc.left;
                const int h = rc.bottom - rc.top;
                const int x = pt.x + 20;  // offset right of cursor
                const int y = pt.y + 20;
                SetWindowPos(static_cast<HWND>(hwnd_), HWND_TOPMOST,
                             x, y, w, h,
                             SWP_NOACTIVATE | SWP_NOSIZE | SWP_SHOWWINDOW);
            }
            std::this_thread::sleep_for(std::chrono::milliseconds(500));
        }
    });
}

void OverlayWindow::destroyWindow() {
    if (hwnd_ != nullptr) {
        DestroyWindow(static_cast<HWND>(hwnd_));
        hwnd_ = nullptr;
    }
    if (hicon_ != nullptr) {
        DestroyIcon(static_cast<HICON>(hicon_));
        hicon_ = nullptr;
    }
}

#endif  // _WIN32

}  // namespace vocawin
