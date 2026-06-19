#pragma once

#include <cstddef>
#include <string>

namespace vocawin {

// Lightweight system info probe. Reads CPU brand, OS version string, and
// total physical RAM. Used by the Settings "About" tab and by ModelManager
// to pick a recommended model size. Per SPEC \u00a75.2.5.
class SystemInfo {
public:
    static std::string cpuName();
    static std::string osName();
    static std::size_t totalRamBytes();
    static std::string summary();
};

}  // namespace vocawin
