#include "platform/Notification.h"

#if defined(_WIN32)
#include <windows.h>
#endif

namespace vocawin {

namespace {

std::wstring utf8ToWide(const std::string& s) {
#if defined(_WIN32)
    if (s.empty()) return std::wstring{};
    const int size = ::MultiByteToWideChar(
        CP_UTF8, 0, s.data(), static_cast<int>(s.size()), nullptr, 0);
    if (size <= 0) return std::wstring{};
    std::wstring out(static_cast<std::size_t>(size), L'\0');
    ::MultiByteToWideChar(CP_UTF8, 0, s.data(), static_cast<int>(s.size()),
                          out.data(), size);
    return out;
#else
    return std::wstring(s.begin(), s.end());
#endif
}

}  // namespace

Notifier::Notifier() = default;
Notifier::~Notifier() = default;

bool Notifier::show(const std::string& title, const std::string& body) {
    if (!enabled_) {
        return true;
    }
#if defined(_WIN32)
    const std::wstring wt = utf8ToWide(title);
    const std::wstring wb = utf8ToWide(body);
    const int rc = ::MessageBoxW(nullptr, wb.c_str(), wt.c_str(), MB_OK |
                                  MB_ICONINFORMATION | MB_SETFOREGROUND);
    return rc != 0;
#else
    (void)title; (void)body;
    return true;
#endif
}

bool Notifier::show(const std::wstring& title, const std::wstring& body) {
    if (!enabled_) {
        return true;
    }
#if defined(_WIN32)
    const int rc = ::MessageBoxW(nullptr, body.c_str(), title.c_str(), MB_OK |
                                  MB_ICONINFORMATION | MB_SETFOREGROUND);
    return rc != 0;
#else
    (void)title; (void)body;
    return true;
#endif
}

}  // namespace vocawin
