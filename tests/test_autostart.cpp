#include <cassert>
#include <cstdio>
#include <filesystem>

#include "platform/Autostart.h"

int main() {
    using namespace vocawin;

    const std::filesystem::path dummy =
        std::filesystem::temp_directory_path() / "vocawin_test_dummy.exe";

    // 1. Initial state: not enabled, regardless of registry state from prior runs.
    Autostart a(dummy);
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

    return 0;
}
