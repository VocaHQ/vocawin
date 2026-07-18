#include <cassert>
#include <cstdio>
#include <filesystem>

#include "platform/Autostart.h"

int main() {
    using namespace vocawin;

    const std::filesystem::path dummy =
        std::filesystem::temp_directory_path() / "vocawin_test_dummy.exe";

    Autostart a(dummy);

#if defined(_WIN32)
    // 1. Initial state: not enabled.
    a.disable();
    assert(!a.isEnabled());

    // 2. Enable writes the registry key.
    assert(a.enable());
    assert(a.isEnabled());

    // 3. Round-trip: query the path VocaWin would launch with.
    const auto path = a.launchPath();
    assert(!path.empty());

    // 4. Disable removes the registry value.
    assert(a.disable());
    assert(!a.isEnabled());

    // 5. Re-enable then disable twice (idempotent on disable).
    assert(a.enable());
    assert(a.disable());
    assert(a.disable());
    assert(!a.isEnabled());
#else
    // Non-Windows stub: enable is a no-op failure, disable is success.
    assert(!a.isEnabled());
    assert(!a.enable());
    assert(!a.isEnabled());
    assert(a.disable());
    assert(a.launchPath().empty());
#endif

    return 0;
}
