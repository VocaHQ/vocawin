#pragma once

#include <filesystem>
#include <string>

namespace vocawin {

// Manages the Windows "Run" registry key so VocaWin can launch itself
// when the user signs in. Per SPEC \u00a79.5.
//
// On Win32 the implementation reads/writes:
//   HKCU\Software\Microsoft\Windows\CurrentVersion\Run\VocaWin
// The value is the absolute path of the executable followed by the
// " --minimized" switch so the app starts in the tray.
//
// On non-Win32 the class is a no-op stub that always reports
// `isEnabled() == false` and returns false from enable()/disable().
// This keeps the unit tests portable.
class Autostart {
public:
    explicit Autostart(std::filesystem::path executablePath);

    bool isEnabled() const;
    bool enable();
    bool disable();

    std::wstring launchPath() const;
    std::filesystem::path executablePath() const { return executablePath_; }

private:
    std::filesystem::path executablePath_;
};

}  // namespace vocawin
