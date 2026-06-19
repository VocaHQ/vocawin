#include <cassert>
#include <string>

#include "platform/SystemInfo.h"

int main() {
    using namespace vocawin;

    const auto summary = SystemInfo::summary();
    assert(!summary.empty());

    const auto ramBytes = SystemInfo::totalRamBytes();
    // Any real Windows host has at least 512 MB of physical RAM; a
    // zero return indicates the GlobalMemoryStatusEx call failed
    // (which would be a real bug).
    assert(ramBytes >= 512ULL * 1024 * 1024);

    const auto cpuName = SystemInfo::cpuName();
    assert(!cpuName.empty());

    const auto osName = SystemInfo::osName();
    assert(!osName.empty());

    return 0;
}
