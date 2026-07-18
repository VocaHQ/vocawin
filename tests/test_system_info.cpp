#include <cassert>
#include <string>

#include "platform/SystemInfo.h"

int main() {
    using namespace vocawin;

    const auto summary = SystemInfo::summary();
    assert(!summary.empty());

    const auto ramBytes = SystemInfo::totalRamBytes();
#if defined(_WIN32)
    // Any real Windows host has at least 512 MB of physical RAM; a
    // zero return indicates the GlobalMemoryStatusEx call failed.
    assert(ramBytes >= 512ULL * 1024 * 1024);
#else
    // Stub returns 0 on non-Windows; summary still non-empty.
    (void)ramBytes;
#endif

    const auto cpuName = SystemInfo::cpuName();
    assert(!cpuName.empty());

    const auto osName = SystemInfo::osName();
    assert(!osName.empty());

    return 0;
}
