#include "ui/TrayIcon.h"

#include <cstring>
#include <filesystem>

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
#if defined(_WIN32)
    if (hwnd_ == nullptr) return false;
    MSG msg;
    while (PeekMessageW(&msg, static_cast<HWND>(hwnd_), 0, 0, PM_REMOVE)) {
        if (msg.message == WM_USER + 1) {
            onTrayMessage(static_cast<unsigned int>(msg.lParam));
            return true;
        }
        if (msg.message == WM_COMMAND) {
            onMenuCommandId(static_cast<unsigned int>(LOWORD(msg.wParam)));
            return true;
        }
        TranslateMessage(&msg);
        DispatchMessageW(&msg);
    }
    return false;
#else
    return false;
#endif
}

bool TrayIcon::updateNotifyArea() {
#if defined(_WIN32)
    if (nid_ == nullptr) {
        return false;
    }
    auto* nid = static_cast<NOTIFYICONDATAW*>(nid_);
    nid->uFlags = NIF_TIP | NIF_ICON;
    const wchar_t* iconName = nullptr;
    switch (state_) {
        case State::Idle:       iconName = L"tray-idle.ico";       break;
        case State::Recording:  iconName = L"tray-recording.ico";  break;
        case State::Processing: iconName = L"tray-processing.ico"; break;
        case State::Error:      iconName = L"tray-error.ico";      break;
        case State::NoModel:    iconName = L"tray-idle.ico";       break;
    }
    if (!iconPath_.empty() && iconName != nullptr) {
        std::filesystem::path full = std::filesystem::path(iconPath_) / iconName;
        HICON hNew = static_cast<HICON>(LoadImageW(
            nullptr, full.wstring().c_str(), IMAGE_ICON, 16, 16,
            LR_LOADFROMFILE | LR_DEFAULTSIZE));
        if (hNew != nullptr) {
            if (currentIcon_ != nullptr && ownsCurrentIcon_) {
                DestroyIcon(static_cast<HICON>(currentIcon_));
            }
            nid->hIcon = hNew;
            currentIcon_ = hNew;
            ownsCurrentIcon_ = true;
        }
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
    AppendMenuW(hMenu, MF_STRING | (settingsPath_.empty() ? MF_GRAYED : 0),
                kMenuSettings, L"Open Settings File...");
    AppendMenuW(hMenu, MF_STRING | (logsDir_.empty() ? MF_GRAYED : 0),
                kMenuLogs, L"Open Log Folder...");
    AppendMenuW(hMenu, MF_STRING, kMenuAbout, L"About VocaWin");
    AppendMenuW(hMenu, MF_SEPARATOR, 0, nullptr);
    AppendMenuW(hMenu, MF_STRING, kMenuQuit, L"Quit");

    POINT pt;
    GetCursorPos(&pt);
    SetForegroundWindow(static_cast<HWND>(hwnd_));
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
