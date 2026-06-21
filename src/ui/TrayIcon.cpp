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

#if defined(_WIN32)

static LRESULT CALLBACK trayWndProc(HWND hwnd, UINT msg, WPARAM wParam,
                                     LPARAM lParam) {
    if (msg == WM_NCCREATE) {
        auto* cs = reinterpret_cast<CREATESTRUCTW*>(lParam);
        SetWindowLongPtrW(hwnd, GWLP_USERDATA,
                          reinterpret_cast<LONG_PTR>(cs->lpCreateParams));
        return DefWindowProcW(hwnd, msg, wParam, lParam);
    }
    auto* self = reinterpret_cast<TrayIcon*>(
        GetWindowLongPtrW(hwnd, GWLP_USERDATA));
    if (self == nullptr) {
        return DefWindowProcW(hwnd, msg, wParam, lParam);
    }
    if (msg == WM_USER + 1) {
        self->onTrayMessage(static_cast<unsigned int>(lParam));
        return 0;
    }
    if (msg == WM_COMMAND) {
        self->onMenuCommandId(static_cast<unsigned int>(LOWORD(wParam)));
        return 0;
    }
    return DefWindowProcW(hwnd, msg, wParam, lParam);
}

#endif

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
    wc.lpfnWndProc = trayWndProc;
    wc.hInstance = hInstance;
    wc.lpszClassName = kClassName;
    RegisterClassExW(&wc);

    hwnd_ = CreateWindowExW(0, kClassName, L"VocaWinTray", 0, 0, 0, 0, 0,
                            HWND_MESSAGE, nullptr, hInstance, this);
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
    if (currentIcon_ != nullptr && ownsCurrentIcon_) {
        DestroyIcon(static_cast<HICON>(currentIcon_));
        currentIcon_ = nullptr;
        ownsCurrentIcon_ = false;
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

void TrayIcon::setPaths(const std::wstring& settingsPath,
                        const std::wstring& logsDir) {
    settingsPath_ = settingsPath;
    logsDir_ = logsDir;
}

bool TrayIcon::pumpMessage() {
    return false;
}

bool TrayIcon::updateNotifyArea() {
#if defined(_WIN32)
    if (nid_ == nullptr) {
        return false;
    }
    auto* nid = static_cast<NOTIFYICONDATAW*>(nid_);
    nid->uFlags = NIF_TIP | NIF_ICON;
    LPWSTR stockIcon = IDI_APPLICATION;
    switch (state_) {
        case State::Idle:       stockIcon = IDI_INFORMATION; break;
        case State::Recording:  stockIcon = IDI_EXCLAMATION; break;
        case State::Processing: stockIcon = IDI_APPLICATION;  break;
        case State::Error:      stockIcon = IDI_WARNING;      break;
        case State::NoModel:    stockIcon = IDI_QUESTION;     break;
    }
    HICON hNew = LoadIconW(nullptr, stockIcon);
    if (hNew != nullptr) {
        nid->hIcon = hNew;
        currentIcon_ = hNew;
        ownsCurrentIcon_ = false;
    }
    wcsncpy(nid->szTip, tooltip_.c_str(), 63);
    nid->szTip[63] = L'\0';
    return Shell_NotifyIconW(NIM_MODIFY, nid) != FALSE;
#else
    (void)tooltip_;
    return true;
#endif
}

#if defined(_WIN32)
void TrayIcon::showContextMenu() {
    HMENU hMenu = CreatePopupMenu();
    if (hMenu == nullptr) return;

    AppendMenuW(hMenu, MF_STRING, kMenuToggle, state_ == State::Recording
                                                   ? L"Stop Recording"
                                                   : L"Start Recording");
    AppendMenuW(hMenu, MF_SEPARATOR, 0, nullptr);
    AppendMenuW(hMenu, MF_STRING, kMenuSettings, L"Settings...");
    AppendMenuW(hMenu, MF_STRING | (logsDir_.empty() ? MF_GRAYED : 0),
                kMenuLogs, L"Open Log Folder...");
    AppendMenuW(hMenu, MF_STRING, kMenuAbout, L"About VocaWin");
    AppendMenuW(hMenu, MF_SEPARATOR, 0, nullptr);
    AppendMenuW(hMenu, MF_STRING, kMenuQuit, L"Quit");

    POINT pt;
    GetCursorPos(&pt);
    TrackPopupMenu(hMenu, TPM_RIGHTBUTTON | TPM_BOTTOMALIGN | TPM_RIGHTALIGN,
                    pt.x, pt.y, 0, static_cast<HWND>(hwnd_), nullptr);
    DestroyMenu(hMenu);
}

void TrayIcon::onTrayMessage(unsigned int msg) {
    switch (msg) {
        case WM_RBUTTONUP:
            showContextMenu();
            break;
        case WM_LBUTTONUP:
            if (onMenuCommand) onMenuCommand(MenuCommand::ToggleRecording);
            break;
        default:
            break;
    }
}

void TrayIcon::onMenuCommandId(unsigned int id) {
    if (!onMenuCommand) return;
    switch (id) {
        case kMenuToggle:   onMenuCommand(MenuCommand::ToggleRecording); break;
        case kMenuSettings: onMenuCommand(MenuCommand::OpenSettings);  break;
        case kMenuLogs:     onMenuCommand(MenuCommand::OpenLogs);      break;
        case kMenuAbout:    onMenuCommand(MenuCommand::About);         break;
        case kMenuQuit:     onMenuCommand(MenuCommand::Quit);          break;
        default: break;
    }
}
#endif  // _WIN32

}  // namespace vocawin
