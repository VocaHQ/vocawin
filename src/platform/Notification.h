#pragma once

#include <functional>
#include <string>

namespace vocawin {

// Thin wrapper around Windows toast notifications. Per SPEC \u00a74.2.10.
// On non-Win32 platforms the methods are no-ops that always return true
// and never display anything. The user's lastError() message is exposed
// so callers can route toast failures to a log file.
class Notifier {
public:
    Notifier();
    ~Notifier();

    Notifier(const Notifier&) = delete;
    Notifier& operator=(const Notifier&) = delete;

    bool isEnabled() const { return enabled_; }
    void setEnabled(bool enabled) { enabled_ = enabled; }

    bool show(const std::string& title, const std::string& body);
    bool show(const std::wstring& title, const std::wstring& body);

    std::string lastError() const { return lastError_; }

private:
    bool enabled_{false};
    std::string lastError_;
};

}  // namespace vocawin
