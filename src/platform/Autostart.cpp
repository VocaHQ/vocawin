#include "platform/Autostart.h"

#if defined(_WIN32)
#include <windows.h>
#endif

namespace vocawin {

namespace {
constexpr const wchar_t* kRunKeyPath =
    L"Software\\Microsoft\\Windows\\CurrentVersion\\Run";
constexpr const wchar_t* kValueName = L"VocaWin";
constexpr const wchar_t* kSwitch = L" --minimized";
}  // namespace

Autostart::Autostart(std::filesystem::path executablePath)
    : executablePath_(std::move(executablePath)) {}

bool Autostart::isEnabled() const {
#if defined(_WIN32)
    HKEY key = nullptr;
    if (RegOpenKeyExW(HKEY_CURRENT_USER, kRunKeyPath, 0, KEY_READ, &key) !=
        ERROR_SUCCESS) {
        return false;
    }
    wchar_t buf[1024] = {};
    DWORD bufSize = sizeof(buf);
    DWORD type = 0;
    const LONG rc = RegQueryValueExW(key, kValueName, nullptr, &type,
                                     reinterpret_cast<LPBYTE>(buf), &bufSize);
    RegCloseKey(key);
    return rc == ERROR_SUCCESS && type == REG_SZ && buf[0] != L'\0';
#else
    (void)kRunKeyPath; (void)kValueName;
    return false;
#endif
}

bool Autostart::enable() {
#if defined(_WIN32)
    HKEY key = nullptr;
    if (RegCreateKeyExW(HKEY_CURRENT_USER, kRunKeyPath, 0, nullptr, 0,
                        KEY_WRITE, nullptr, &key, nullptr) != ERROR_SUCCESS) {
        return false;
    }
    // Quoting guards against the "unquoted service path" attack:
    // a path like "C:\Program Files\VocaWin\vocawin.exe" without
    // surrounding quotes causes Windows to search for and possibly
    // execute "C:\Program.exe" first.
    std::wstring value = L"\"" + executablePath_.wstring() + L"\"" + kSwitch;
    const LONG rc = RegSetValueExW(
        key, kValueName, 0, REG_SZ,
        reinterpret_cast<const BYTE*>(value.c_str()),
        static_cast<DWORD>((value.size() + 1) * sizeof(wchar_t)));
    RegCloseKey(key);
    return rc == ERROR_SUCCESS;
#else
    (void)kSwitch;
    return false;
#endif
}

bool Autostart::disable() {
#if defined(_WIN32)
    HKEY key = nullptr;
    if (RegOpenKeyExW(HKEY_CURRENT_USER, kRunKeyPath, 0, KEY_WRITE, &key) !=
        ERROR_SUCCESS) {
        return true;  // nothing to delete
    }
    const LONG rc = RegDeleteValueW(key, kValueName);
    RegCloseKey(key);
    return rc == ERROR_SUCCESS || rc == ERROR_FILE_NOT_FOUND;
#else
    return true;
#endif
}

std::wstring Autostart::launchPath() const {
#if defined(_WIN32)
    HKEY key = nullptr;
    if (RegOpenKeyExW(HKEY_CURRENT_USER, kRunKeyPath, 0, KEY_READ, &key) !=
        ERROR_SUCCESS) {
        return L"";
    }
    wchar_t buf[1024] = {};
    DWORD bufSize = sizeof(buf);
    DWORD type = 0;
    const LONG rc = RegQueryValueExW(key, kValueName, nullptr, &type,
                                     reinterpret_cast<LPBYTE>(buf), &bufSize);
    RegCloseKey(key);
    if (rc == ERROR_SUCCESS && type == REG_SZ) {
        return std::wstring(buf);
    }
    return L"";
#else
    return L"";
#endif
}

}  // namespace vocawin
