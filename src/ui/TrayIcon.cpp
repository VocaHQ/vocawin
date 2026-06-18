#include "ui/TrayIcon.h"

#include <cstring>

#if defined(_WIN32)
#include <windows.h>
#include <shellapi.h>
#endif

namespace vocawin {

TrayIcon::TrayIcon() = default;

TrayIcon::~TrayIcon() {
    shutdown();
}

std::wstring TrayIcon::defaultTooltipFor(State state) {
    switch (state) {
        case State::Idle:       return L"VocaWin \u2014 Ready";
        case State::Recording:  return L"VocaWin \u2014 Recording...";
        case State::Processing: return L"VocaWin \u2014 Processing...";
        case State::Error:      return L"VocaWin \u2014 Error";
        case State::NoModel:    return L"VocaWin \u2014 No model loaded";
    }
    return L"VocaWin";
}

bool TrayIcon::initialize() {
    if (initialized_) {
        return true;
    }
    tooltip_ = defaultTooltipFor(state_);
#if defined(_WIN32)
    const HINSTANCE hInstance = GetModuleHandleW(nullptr);
    static const wchar_t kClassName[] = L"VocaWinTrayClass";
    WNDCLASSEXW wc{};
    wc.cbSize = sizeof(wc);
    wc.lpfnWndProc = DefWindowProcW;
    wc.hInstance = hInstance;
    wc.lpszClassName = kClassName;
    RegisterClassExW(&wc);  // ignore ERROR_CLASS_ALREADY_EXISTS
    hwnd_ = CreateWindowExW(0, kClassName, L"VocaWinTray", 0, 0, 0, 0, 0,
                            HWND_MESSAGE, nullptr, hInstance, nullptr);
    if (hwnd_ == nullptr) {
        return false;
    }
    nid_ = calloc(1, sizeof(NOTIFYICONDATAW));
    if (nid_ == nullptr) {
        DestroyWindow(static_cast<HWND>(hwnd_));
        hwnd_ = nullptr;
        return false;
    }
    auto* nid = static_cast<NOTIFYICONDATAW*>(nid_);
    nid->cbSize = sizeof(NOTIFYICONDATAW);
    nid->hWnd = static_cast<HWND>(hwnd_);
    nid->uID = 1;
    nid->uFlags = NIF_ICON | NIF_TIP | NIF_MESSAGE;
    nid->uCallbackMessage = WM_USER + 1;
    nid->hIcon = LoadIconW(nullptr, IDI_APPLICATION);
    currentIcon_ = nid->hIcon;
    wcsncpy(nid->szTip, tooltip_.c_str(), 63);
    nid->szTip[63] = L'\0';
    if (!Shell_NotifyIconW(NIM_ADD, nid)) {
        free(nid_);
        nid_ = nullptr;
        DestroyWindow(static_cast<HWND>(hwnd_));
        hwnd_ = nullptr;
        return false;
    }
    initialized_ = true;
    return true;
#else
    initialized_ = true;
    return true;
#endif
}

void TrayIcon::shutdown() {
    if (!initialized_) {
        return;
    }
#if defined(_WIN32)
    if (nid_ != nullptr) {
        Shell_NotifyIconW(NIM_DELETE, static_cast<NOTIFYICONDATAW*>(nid_));
        free(nid_);
        nid_ = nullptr;
    }
    if (hwnd_ != nullptr) {
        DestroyWindow(static_cast<HWND>(hwnd_));
        hwnd_ = nullptr;
    }
    if (currentIcon_ != nullptr) {
        DestroyIcon(static_cast<HICON>(currentIcon_));
        currentIcon_ = nullptr;
    }
#endif
    initialized_ = false;
}

bool TrayIcon::setState(State state) {
    if (!initialized_) {
        return false;
    }
    state_ = state;
    if (tooltip_.empty()) {
        tooltip_ = defaultTooltipFor(state);
    }
    return updateNotifyArea();
}

void TrayIcon::setTooltip(const std::wstring& tooltip) {
    tooltip_ = tooltip;
    if (initialized_) {
        updateNotifyArea();
    }
}

bool TrayIcon::updateNotifyArea() {
#if defined(_WIN32)
    if (nid_ == nullptr) {
        return false;
    }
    auto* nid = static_cast<NOTIFYICONDATAW*>(nid_);
    nid->uFlags = NIF_TIP | NIF_ICON;
    LPWSTR iconId = IDI_APPLICATION;
    switch (state_) {
        case State::Idle:       iconId = IDI_INFORMATION; break;
        case State::Recording:  iconId = IDI_EXCLAMATION; break;
        case State::Processing: iconId = IDI_APPLICATION;  break;
        case State::Error:      iconId = IDI_WARNING;      break;
        case State::NoModel:    iconId = IDI_QUESTION;     break;
    }
    HICON hNew = LoadIconW(nullptr, iconId);
    if (hNew != nullptr) {
        if (currentIcon_ != nullptr) {
            DestroyIcon(static_cast<HICON>(currentIcon_));
        }
        nid->hIcon = hNew;
        currentIcon_ = hNew;
    }
    wcsncpy(nid->szTip, tooltip_.c_str(), 63);
    nid->szTip[63] = L'\0';
    return Shell_NotifyIconW(NIM_MODIFY, nid) != FALSE;
#else
    (void)tooltip_;
    return true;
#endif
}

}  // namespace vocawin
